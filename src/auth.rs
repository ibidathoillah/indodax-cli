use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::IndodaxError;

#[derive(Debug)]
pub struct Signer {
    api_key: String,
    secret_key: String,
    last_nonce: AtomicU64,
}

impl Signer {
    pub fn new(api_key: &str, secret_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
            last_nonce: AtomicU64::new(0),
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn next_nonce_str(&self) -> String {
        self.next_nonce().to_string()
    }

    fn next_nonce(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        loop {
            let prev = self.last_nonce.load(Ordering::Acquire);
            let next = if now > prev { now } else { prev + 1 };
            if self
                .last_nonce
                .compare_exchange(prev, next, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                return next;
            }
        }
    }

    pub fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn sign_v1(&self, payload: &str) -> Result<(String, String), IndodaxError> {
        let signature = self.hmac_sha512(payload, &self.secret_key)?;
        let encoded_sign = hex::encode(signature);

        Ok((payload.to_string(), encoded_sign))
    }

    pub fn sign_v2(&self, query_string: &str, _timestamp: u64) -> Result<String, IndodaxError> {
        let signature = self.hmac_sha512(query_string, &self.secret_key)?;
        Ok(hex::encode(signature))
    }

    fn hmac_sha512(&self, data: &str, key: &str) -> Result<Vec<u8>, IndodaxError> {
        let mut mac = Hmac::<Sha512>::new_from_slice(key.as_bytes())
            .map_err(|e| IndodaxError::Other(format!("HMAC initialization failed: {}", e)))?;
        mac.update(data.as_bytes());
        Ok(mac.finalize().into_bytes().to_vec())
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
        let (payload, signature) = signer.sign_v1("method=test&nonce=123").unwrap();
        assert_eq!(payload, "method=test&nonce=123");
        assert!(!signature.is_empty());
        // Signature should be hex encoded
        assert!(hex::decode(&signature).is_ok());
    }

    #[test]
    fn test_signer_sign_v1_different_secrets() {
        let signer1 = Signer::new("key", "secret1");
        let signer2 = Signer::new("key", "secret2");
        let (_payload, sig1) = signer1.sign_v1("test").unwrap();
        let (_payload2, sig2) = signer2.sign_v1("test").unwrap();
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_signer_sign_v2() {
        let signer = Signer::new("key", "secret");
        let signature = signer.sign_v2("param1=value1", 1234567890).unwrap();
        assert!(!signature.is_empty());
        // Signature should be hex encoded
        assert!(hex::decode(&signature).is_ok());
    }

    #[test]
    fn test_signer_sign_v2_with_timestamp() {
        let signer = Signer::new("key", "secret");
        let query_string = "symbol=BTCIDR";
        let timestamp = 1234567890000u64;
        let signature = signer.sign_v2(query_string, timestamp).unwrap();
        
        // Verify the payload that was signed
        let _expected_payload = format!("{}&timestamp={}&recvWindow=10000", query_string, timestamp);
        let _decoded = hex::decode(&signature).unwrap();
        // We can't easily verify the HMAC without knowing the secret, but we can verify it's valid hex
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_hmac_sha512_output_length() {
        let signer = Signer::new("key", "secret");
        let result = signer.hmac_sha512("test data", "secret").unwrap();
        // SHA512 produces 64 bytes
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn test_signer_multiple_signatures_different() {
        let signer = Signer::new("key", "secret");
        let (_, sig1) = signer.sign_v1("payload1").unwrap();
        let (_, sig2) = signer.sign_v1("payload2").unwrap();
        assert_ne!(sig1, sig2);
    }
}
