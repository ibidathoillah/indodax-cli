use crate::auth::Signer;
use crate::errors::{ErrorCategory, IndodaxError};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::collections::{HashMap, BTreeMap};
use tokio::sync::Mutex;

use std::time::{Duration, Instant};

const PUBLIC_BASE_URL: &str = "https://indodax.com";
const PRIVATE_V1_URL: &str = "https://indodax.com/tapi";
const PRIVATE_V2_BASE: &str = "https://tapi.btcapi.net";
const WS_TOKEN_URL: &str = "https://indodax.com/api/private_ws/v1/generate_token";
const MAX_RETRIES: u32 = 3;

#[derive(Debug)]
struct RateLimiterState {
    tokens: u64,
    last_refill: Instant,
}

/// Token-bucket rate limiter for proactive 429 avoidance.
#[derive(Debug)]
struct RateLimiter {
    capacity: u64,
    refill_per_sec: u64,
    state: Mutex<RateLimiterState>,
}

impl RateLimiter {
    fn new(capacity: u64, refill_per_sec: u64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            state: Mutex::new(RateLimiterState {
                tokens: capacity,
                last_refill: Instant::now(),
            }),
        }
    }

    fn from_env() -> Self {
        let rps = std::env::var("INDODAX_RATE_LIMIT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5)
            .max(1);
        Self::new(rps, rps)
    }

    async fn acquire(&self) {
        loop {
            let mut state = self.state.lock().await;
            let elapsed = state.last_refill.elapsed();
            if elapsed >= Duration::from_secs(1) {
                let secs = elapsed.as_secs();
                let add = self.refill_per_sec * secs;
                state.tokens = state.tokens.saturating_add(add).min(self.capacity);
                state.last_refill += Duration::from_secs(secs);
            }
            if state.tokens > 0 {
                state.tokens -= 1;
                return;
            }
            let elapsed_ms = elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
            let wait = if elapsed_ms < 1000 {
                Duration::from_millis(1000 - elapsed_ms)
            } else {
                Duration::from_millis(50)
            };
            drop(state);
            tokio::time::sleep(wait).await;
        }
    }
}

#[derive(Debug)]
pub struct IndodaxClient {
    http: Client,
    signer: Option<Signer>,
    rate_limiter: RateLimiter,
}

#[derive(Debug, serde::Deserialize)]
pub struct IndodaxV1Response<T> {
    pub success: i32,
    #[serde(rename = "return")]
    pub return_data: Option<T>,
    pub error: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct IndodaxV2Response<T> {
    pub data: Option<T>,
    pub code: Option<i64>,
    pub error: Option<String>,
}

impl IndodaxClient {
    pub fn new(signer: Option<Signer>) -> Result<Self, IndodaxError> {
        let http = Client::builder()
            .user_agent(format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(2)
            .build()
            .map_err(|e| IndodaxError::Other(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            http,
            signer,
            rate_limiter: RateLimiter::from_env(),
        })
    }

    pub fn signer(&self) -> Option<&Signer> {
        self.signer.as_ref()
    }

    pub fn http_client(&self) -> &Client {
        &self.http
    }

    pub async fn public_get<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, IndodaxError> {
        let url = format!("{}{}", PUBLIC_BASE_URL, path);
        let resp = self.retry_get(&url).await?;
        self.handle_response(resp).await
    }

    pub async fn countdown_cancel_all(
        &self,
        pair: Option<&str>,
        countdown_time: u64,
    ) -> Result<serde_json::Value, IndodaxError> {
        let signer = self.signer.as_ref().ok_or_else(|| {
            IndodaxError::Config("API credentials required for countdown cancel all".into())
        })?;

        let mut body_parts: Vec<String> = vec![
            format!("countdownTime={}", countdown_time),
        ];
        if let Some(p) = pair {
            body_parts.push(format!("pair={}", p));
        }

        let body = body_parts.join("&");
        let (payload, signature) = signer.sign_v1(&body)?;

        let url = format!("{}/countdownCancelAll", PRIVATE_V1_URL);
        let req = self
            .http
            .post(&url)
            .header("Key", signer.api_key())
            .header("Sign", &signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload);
        let resp = self.send_with_retry(req).await?;

        let body_text = resp.text().await?;
        let data: serde_json::Value = serde_json::from_str(&body_text)?;

        if let Some(success) = data.get("success").and_then(|v| v.as_i64()) {
            if success == 1 {
                Ok(data)
            } else {
                let error_msg = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                let error_code = data
                    .get("error_code")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let category = match error_code.as_deref() {
                    Some("invalid_credentials") => ErrorCategory::Authentication,
                    Some("rate_limit") => ErrorCategory::RateLimit,
                    Some(c) if c.contains("invalid") => ErrorCategory::Validation,
                    _ => ErrorCategory::Unknown,
                };
                Err(IndodaxError::api(error_msg, category, error_code))
            }
        } else {
            Ok(data)
        }
    }

    pub async fn generate_ws_token(&self) -> Result<String, IndodaxError> {
        let signer = self.signer.as_ref().ok_or_else(|| {
            IndodaxError::Config("API credentials required for WebSocket token generation".into())
        })?;

        let nonce = signer.next_nonce_str();
        let (_, signature) = signer.sign_v1(&nonce)?;

        let req = self
            .http
            .post(WS_TOKEN_URL)
            .header("Key", signer.api_key())
            .header("Sign", &signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("nonce={}", nonce));
        let resp = self.send_with_retry(req).await?;

        let body_text = resp.text().await?;
        let val: serde_json::Value = serde_json::from_str(&body_text)?;

        val.get("token")
            .and_then(|t| t.as_str())
            .map(|t| t.to_string())
            .or_else(|| val.get("data").and_then(|d| d.get("token")).and_then(|t| t.as_str()).map(|t| t.to_string()))
            .ok_or_else(|| IndodaxError::WsToken(format!("No token in response: {}", body_text)))
    }

    pub async fn public_get_v2<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, IndodaxError> {
        let url = format!("{}{}", PUBLIC_BASE_URL, path);
        let resp = self.retry_get_with_params(&url, params).await?;
        self.handle_response(resp).await
    }

    pub async fn private_post_v1<T: DeserializeOwned>(
        &self,
        method: &str,
        params: &HashMap<String, String>,
    ) -> Result<T, IndodaxError> {
        let signer = self.signer.as_ref().ok_or_else(|| {
            IndodaxError::Config("API credentials required for private endpoints".into())
        })?;

        let mut full_params: BTreeMap<String, String> = params
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        full_params.insert("method".into(), method.to_string());
        full_params.insert("nonce".into(), signer.next_nonce_str());

        let body = serde_urlencoded_str(&full_params);
        let (_, signature) = signer.sign_v1(&body)?;

        let resp = self
            .retry_post(PRIVATE_V1_URL, &body, signer.api_key(), &signature)
            .await?;

        let body_text = resp.text().await?;
        let envelope: IndodaxV1Response<T> = serde_json::from_str(&body_text).map_err(|e| {
            IndodaxError::Parse(format!(
                "Failed to parse response: {} (body: {})",
                e, body_text
            ))
        })?;

        if envelope.success == 1 {
            envelope.return_data.ok_or_else(|| {
                IndodaxError::Parse("API returned success but no 'return' data".into())
            })
        } else {
            Err(IndodaxError::api(
                envelope.error.unwrap_or_else(|| "Unknown error".into()),
                match envelope.error_code.as_deref() {
                    Some("invalid_credentials") => ErrorCategory::Authentication,
                    Some("rate_limit") => ErrorCategory::RateLimit,
                    Some(c) if c.contains("invalid") => ErrorCategory::Validation,
                    _ => ErrorCategory::Unknown,
                },
                envelope.error_code,
            ))
        }
    }

    pub async fn private_get_v2<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &HashMap<String, String>,
    ) -> Result<T, IndodaxError> {
        let signer = self.signer.as_ref().ok_or_else(|| {
            IndodaxError::Config("API credentials required for private endpoints".into())
        })?;

        let mut qs_parts: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let timestamp = Signer::now_millis();
        qs_parts.push(format!("timestamp={}", timestamp));
        qs_parts.push("recvWindow=5000".to_string());
        qs_parts.sort();
        let query_string = qs_parts.join("&");

        let signature = signer.sign_v2(&query_string, timestamp)?;
        let url = format!("{}{}?{}", PRIVATE_V2_BASE, path, query_string);

        let req = self
            .http
            .get(&url)
            .header("X-APIKEY", signer.api_key())
            .header("Sign", &signature)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");
        let resp = self.send_with_retry(req).await?;

        let body_text = resp.text().await?;
        let envelope: IndodaxV2Response<T> = serde_json::from_str(&body_text).map_err(|e| {
            IndodaxError::Parse(format!(
                "Failed to parse v2 response: {} (body: {})",
                e, body_text
            ))
        })?;

        if let Some(data) = envelope.data {
            Ok(data)
        } else if let Some(error) = envelope.error {
            Err(IndodaxError::api(error, ErrorCategory::Unknown, None))
        } else {
            Ok(serde_json::from_str(&body_text)?)
        }
    }

    async fn retry_get(&self, url: &str) -> Result<Response, IndodaxError> {
        let req = self.http.get(url);
        self.send_with_retry(req).await
    }

    async fn retry_get_with_params(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<Response, IndodaxError> {
        let req = self.http.get(url).query(params);
        self.send_with_retry(req).await
    }

    async fn retry_post(
        &self,
        url: &str,
        body: &str,
        api_key: &str,
        signature: &str,
    ) -> Result<Response, IndodaxError> {
        let req = self
            .http
            .post(url)
            .header("Key", api_key)
            .header("Sign", signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string());
        self.send_with_retry(req).await
    }

    async fn send_with_retry(
        &self,
        builder: RequestBuilder,
    ) -> Result<Response, IndodaxError> {
        self.rate_limiter.acquire().await;
        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt - 1))).await;
            }

            let req = builder
                .try_clone()
                .ok_or_else(|| IndodaxError::Other("Failed to clone request".into()))?;

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp);
                    }

                    if status == StatusCode::TOO_MANY_REQUESTS {
                        last_err = Some(IndodaxError::api(
                            format!("Rate limited (HTTP {})", status.as_u16()),
                            ErrorCategory::RateLimit,
                            None,
                        ));
                        continue;
                    }

                    if status.is_server_error() {
                        last_err = Some(IndodaxError::api(
                            format!("Server error (HTTP {})", status.as_u16()),
                            ErrorCategory::Server,
                            None,
                        ));
                        continue;
                    }

                    last_err = Some(IndodaxError::api(
                        format!("HTTP {}", status.as_u16()),
                        ErrorCategory::Unknown,
                        None,
                    ));
                    break;
                }
                Err(e) => {
                    if e.is_timeout() || e.is_connect() {
                        last_err = Some(IndodaxError::Http(e));
                        continue;
                    }
                    return Err(IndodaxError::Http(e));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            IndodaxError::Other("Max retries exceeded".into())
        }))
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        resp: Response,
    ) -> Result<T, IndodaxError> {
        let body_text = resp.text().await?;
        serde_json::from_str(&body_text).map_err(|e| {
            IndodaxError::Parse(format!(
                "Failed to parse response: {} (body: {})",
                e, body_text
            ))
        })
    }
}

fn serde_urlencoded_str(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                url::form_urlencoded::byte_serialize(k.as_bytes()).collect::<String>(),
                url::form_urlencoded::byte_serialize(v.as_bytes()).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Signer;

    #[test]
    fn test_indodax_client_new_with_signer() {
        let signer = Signer::new("key", "secret");
        let client = IndodaxClient::new(Some(signer)).unwrap();
        assert!(client.signer().is_some());
    }

    #[test]
    fn test_indodax_client_new_without_signer() {
        let client = IndodaxClient::new(None).unwrap();
        assert!(client.signer().is_none());
    }

    #[test]
    fn test_indodax_client_signer() {
        let signer = Signer::new("mykey", "mysecret");
        let client = IndodaxClient::new(Some(signer)).unwrap();
        let s = client.signer().unwrap();
        assert_eq!(s.api_key(), "mykey");
    }

    #[test]
    fn test_indodax_v1_response_success() {
        let json = serde_json::json!({
            "success": 1,
            "return": {"balance": {"btc": "1.0"}},
            "error": null,
            "error_code": null
        });
        let resp: IndodaxV1Response<serde_json::Value> = serde_json::from_value(json).unwrap();
        assert_eq!(resp.success, 1);
        assert!(resp.return_data.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_indodax_v1_response_failure() {
        let json = serde_json::json!({
            "success": 0,
            "return": null,
            "error": "Invalid credentials",
            "error_code": "invalid_credentials"
        });
        let resp: IndodaxV1Response<serde_json::Value> = serde_json::from_value(json).unwrap();
        assert_eq!(resp.success, 0);
        assert!(resp.return_data.is_none());
        assert!(resp.error.is_some());
        assert!(resp.error_code.is_some());
    }

    #[test]
    fn test_indodax_v2_response_success() {
        let json = serde_json::json!({
            "data": {"name": "test"},
            "code": null,
            "error": null
        });
        let resp: IndodaxV2Response<serde_json::Value> = serde_json::from_value(json).unwrap();
        assert!(resp.data.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_indodax_v2_response_error() {
        let json = serde_json::json!({
            "data": null,
            "code": 400,
            "error": "Bad request"
        });
        let resp: IndodaxV2Response<serde_json::Value> = serde_json::from_value(json).unwrap();
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
        assert!(resp.code.is_some());
    }

    #[test]
    fn test_serde_urlencoded_str_single() {
        let mut params = std::collections::BTreeMap::new();
        params.insert("method".into(), "getInfo".into());
        params.insert("nonce".into(), "12345".into());
        
        let result = serde_urlencoded_str(&params);
        assert!(result.contains("method=getInfo"));
        assert!(result.contains("nonce=12345"));
    }

    #[test]
    fn test_serde_urlencoded_str_empty() {
        let params = std::collections::BTreeMap::new();
        let result = serde_urlencoded_str(&params);
        assert_eq!(result, "");
    }

    #[test]
    fn test_serde_urlencoded_str_special_chars() {
        let mut params = std::collections::BTreeMap::new();
        params.insert("key with space".into(), "value&more".into());
        
        let result = serde_urlencoded_str(&params);
        // Should be URL encoded
        assert!(result.contains("%20") || result.contains("+"));
    }

    #[test]
    fn test_public_base_url() {
        assert!(PUBLIC_BASE_URL.contains("indodax.com"));
    }

    #[test]
    fn test_private_v1_url() {
        assert!(PRIVATE_V1_URL.contains("indodax.com/tapi"));
    }

    #[test]
    fn test_private_v2_base() {
        assert!(PRIVATE_V2_BASE.contains("tapi.btcapi.net"));
    }

    #[test]
    fn test_max_retries_constant() {
        assert_eq!(MAX_RETRIES, 3);
    }

    #[test]
    fn test_indodax_v1_response_debug() {
        let resp: IndodaxV1Response<serde_json::Value> = IndodaxV1Response {
            success: 1,
            return_data: Some(serde_json::json!({})),
            error: None,
            error_code: None,
        };
        let debug_str = format!("{:?}", resp);
        assert!(debug_str.contains("success"));
    }

    #[test]
    fn test_indodax_v2_response_debug() {
        let resp: IndodaxV2Response<serde_json::Value> = IndodaxV2Response {
            data: Some(serde_json::json!({})),
            code: None,
            error: None,
        };
        let debug_str = format!("{:?}", resp);
        assert!(debug_str.contains("data"));
    }

    #[test]
    fn test_rate_limiter_from_env_default() {
        // Without env var, should default to 10
        let rl = RateLimiter::from_env();
        // If INDODAX_RATE_LIMIT is set in environment, test may fail
        // so we just verify it doesn't panic
        assert!(rl.capacity > 0);
        assert!(rl.refill_per_sec > 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire_single() {
        let rl = RateLimiter::new(5, 5);
        rl.acquire().await;
        let state = rl.state.lock().await;
        assert_eq!(state.tokens, 4);
    }

    #[tokio::test]
    async fn test_rate_limiter_token_exhaustion_refills() {
        let rl = RateLimiter::new(3, 10);
        for _ in 0..3 {
            rl.acquire().await;
        }
        {
            let state = rl.state.lock().await;
            assert_eq!(state.tokens, 0);
        }

        {
            let mut state = rl.state.lock().await;
            state.last_refill = Instant::now() - Duration::from_secs(1);
        }

        rl.acquire().await;
        let state = rl.state.lock().await;
        assert_eq!(state.tokens, 2);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill_capped_at_capacity() {
        let rl = RateLimiter::new(5, 100);
        for _ in 0..5 {
            rl.acquire().await;
        }
        {
            let state = rl.state.lock().await;
            assert_eq!(state.tokens, 0);
        }

        {
            let mut state = rl.state.lock().await;
            state.last_refill = Instant::now() - Duration::from_secs(10);
        }

        rl.acquire().await;
        let state = rl.state.lock().await;
        assert_eq!(state.tokens, 4);
    }

    #[test]
    fn test_rate_limiter_new_custom() {
        let rl = RateLimiter::new(25, 25);
        assert_eq!(rl.capacity, 25);
        assert_eq!(rl.refill_per_sec, 25);
    }
}
