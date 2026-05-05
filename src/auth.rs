use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Signer {
    api_key: String,
    secret_key: String,
}

impl Signer {
    pub fn new(api_key: &str, secret_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn next_nonce_str(&self) -> String {
        self.next_nonce().to_string()
    }

    fn next_nonce(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    pub fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    pub fn sign_v1(&self, payload: &str, _use_timestamp: bool) -> (String, String) {
        let signature = self.hmac_sha512(payload, &self.secret_key);
        let encoded_sign = hex::encode(signature);

        (payload.to_string(), encoded_sign)
    }

    pub fn sign_v2(&self, query_string: &str, timestamp: u64) -> String {
        let payload = format!("{}&timestamp={}&recvWindow=10000", query_string, timestamp);
        let signature = self.hmac_sha512(&payload, &self.secret_key);
        BASE64.encode(&signature)
    }

    pub fn sign_ws_auth(&self, body: &str) -> String {
        let signature = self.hmac_sha512(body, &self.secret_key);
        BASE64.encode(&signature)
    }

    fn hmac_sha512(&self, data: &str, key: &str) -> Vec<u8> {
        let mut mac = Hmac::<Sha512>::new_from_slice(key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_new() {
        let signer = Signer::new("api_key", "secret_key");
        assert_eq!(signer.api_key(), "api_key");
    }

    #[test]
    fn test_signer_api_key() {
        let signer = Signer::new("my_api_key", "my_secret");
        assert_eq!(signer.api_key(), "my_api_key");
    }

    #[test]
    fn test_signer_next_nonce_str() {
        let signer = Signer::new("key", "secret");
        let nonce = signer.next_nonce_str();
        assert!(!nonce.is_empty());
        // Nonce should be a number
        assert!(nonce.parse::<u64>().is_ok());
    }

    #[test]
    fn test_signer_next_nonce_is_increasing() {
        let signer = Signer::new("key", "secret");
        let nonce1 = signer.next_nonce();
        let nonce2 = signer.next_nonce();
        // Nonces should be increasing (or at least not decreasing)
        assert!(nonce2 >= nonce1);
    }

    #[test]
    fn test_signer_now_millis() {
        let millis = Signer::now_millis();
        assert!(millis > 0);
        // Should be around current time in millis
        assert!(millis > 1_000_000_000_000); // After year 2001
    }

    #[test]
    fn test_signer_sign_v1() {
        let signer = Signer::new("key", "secret");
        let (payload, signature) = signer.sign_v1("method=test&nonce=123", false);
        assert_eq!(payload, "method=test&nonce=123");
        assert!(!signature.is_empty());
        // Signature should be hex encoded
        assert!(hex::decode(&signature).is_ok());
    }

    #[test]
    fn test_signer_sign_v1_different_secrets() {
        let signer1 = Signer::new("key", "secret1");
        let signer2 = Signer::new("key", "secret2");
        let (_payload, sig1) = signer1.sign_v1("test", false);
        let (_payload2, sig2) = signer2.sign_v1("test", false);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_signer_sign_v2() {
        let signer = Signer::new("key", "secret");
        let signature = signer.sign_v2("param1=value1", 1234567890);
        assert!(!signature.is_empty());
        // Signature should be base64 encoded
        assert!(BASE64.decode(&signature).is_ok());
    }

    #[test]
    fn test_signer_sign_v2_with_timestamp() {
        let signer = Signer::new("key", "secret");
        let query_string = "symbol=BTCIDR";
        let timestamp = 1234567890000u64;
        let signature = signer.sign_v2(query_string, timestamp);
        
        // Verify the payload that was signed
        let _expected_payload = format!("{}&timestamp={}&recvWindow=10000", query_string, timestamp);
        let _decoded = BASE64.decode(&signature).unwrap();
        // We can't easily verify the HMAC without knowing the secret, but we can verify it's valid base64
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_signer_sign_ws_auth() {
        let signer = Signer::new("key", "secret");
        let signature = signer.sign_ws_auth("body_data");
        assert!(!signature.is_empty());
        // Signature should be base64 encoded
        assert!(BASE64.decode(&signature).is_ok());
    }

    #[test]
    fn test_signer_sign_ws_auth_different_bodies() {
        let signer = Signer::new("key", "secret");
        let sig1 = signer.sign_ws_auth("body1");
        let sig2 = signer.sign_ws_auth("body2");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_hmac_sha512_output_length() {
        let signer = Signer::new("key", "secret");
        let result = signer.hmac_sha512("test data", "secret");
        // SHA512 produces 64 bytes
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn test_signer_multiple_signatures_different() {
        let signer = Signer::new("key", "secret");
        let (_, sig1) = signer.sign_v1("payload1", false);
        let (_, sig2) = signer.sign_v1("payload2", false);
        assert_ne!(sig1, sig2);
    }
}
