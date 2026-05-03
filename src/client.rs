use crate::auth::Signer;
use crate::errors::{ErrorCategory, IndodaxError};
use crate::telemetry;
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::collections::{HashMap, BTreeMap};
use std::time::Duration;

const PUBLIC_BASE_URL: &str = "https://indodax.com";
const PRIVATE_V1_URL: &str = "https://indodax.com/tapi";
const PRIVATE_V2_BASE: &str = "https://tapi.indodax.com";
const MAX_RETRIES: u32 = 3;

pub struct IndodaxClient {
    http: Client,
    signer: Option<Signer>,
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
    pub fn new(signer: Option<Signer>) -> Self {
        let http = Client::builder()
            .user_agent(telemetry::user_agent())
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(2)
            .build()
            .expect("Failed to create HTTP client");

        Self { http, signer }
    }

    pub fn signer(&self) -> Option<&Signer> {
        self.signer.as_ref()
    }

    pub async fn public_get<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, IndodaxError> {
        let url = format!("{}{}", PUBLIC_BASE_URL, path);
        let resp = self.retry_get(&url).await?;
        self.handle_response(resp).await
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
        let (_, signature) = signer.sign_v1(&body, false);

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
        qs_parts.push("recvWindow=10000".to_string());
        qs_parts.sort();
        let query_string = qs_parts.join("&");

        let signature = signer.sign_v2(&query_string, timestamp);
        let url = format!("{}{}?{}", PRIVATE_V2_BASE, path, query_string);

        let resp = self
            .http
            .get(&url)
            .header("X-APIKEY", signer.api_key())
            .header("Sign", &signature)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send()
            .await?;

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
        self.send_with_retry(req, true).await
    }

    async fn retry_get_with_params(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<Response, IndodaxError> {
        let req = self.http.get(url).query(params);
        self.send_with_retry(req, true).await
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
        self.send_with_retry(req, false).await
    }

    async fn send_with_retry(
        &self,
        builder: RequestBuilder,
        _is_get: bool,
    ) -> Result<Response, IndodaxError> {
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
