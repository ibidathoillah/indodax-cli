use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::time::{SystemTime, UNIX_EPOCH};

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
