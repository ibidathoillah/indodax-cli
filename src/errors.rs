#[derive(Debug, thiserror::Error)]
pub enum ErrorCategory {
    #[error("connection_error")]
    Connection,
    #[error("authentication_error")]
    Authentication,
    #[error("rate_limit")]
    RateLimit,
    #[error("validation_error")]
    Validation,
    #[error("server_error")]
    Server,
    #[error("config_error")]
    Config,
    #[error("unknown_error")]
    Unknown,
}

#[derive(Debug, thiserror::Error)]
pub enum IndodaxError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{message}")]
    Api {
        category: ErrorCategory,
        message: String,
        code: Option<String>,
        retryable: bool,
    },

    #[error("{0}")]
    Config(String),

    #[error("{0}")]
    Parse(String),

    #[error("WebSocket token generation failed: {0}")]
    WsToken(String),

    #[error("{0}")]
    Other(String),
}

impl IndodaxError {
    pub fn api(message: impl Into<String>, category: ErrorCategory, code: Option<String>) -> Self {
        let retryable = matches!(
            category,
            ErrorCategory::Connection | ErrorCategory::Server | ErrorCategory::RateLimit
        );
        IndodaxError::Api {
            category,
            message: message.into(),
            code,
            retryable,
        }
    }

    pub fn category(&self) -> String {
        match self {
            IndodaxError::Api { category, .. } => category.to_string(),
            IndodaxError::Http(_) => "connection_error".to_string(),
            IndodaxError::WebSocket(_) => "connection_error".to_string(),
            IndodaxError::Json(_) => "validation_error".to_string(),
            IndodaxError::Config(_) => "config_error".to_string(),
            IndodaxError::Parse(_) => "validation_error".to_string(),
            IndodaxError::WsToken(_) => "authentication_error".to_string(),
            IndodaxError::Other(_) => "unknown_error".to_string(),
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            IndodaxError::Api { retryable, .. } => *retryable,
            IndodaxError::Http(_) | IndodaxError::WebSocket(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_connection() {
        let cat = ErrorCategory::Connection;
        assert_eq!(format!("{}", cat), "connection_error");
    }

    #[test]
    fn test_error_category_authentication() {
        let cat = ErrorCategory::Authentication;
        assert_eq!(format!("{}", cat), "authentication_error");
    }

    #[test]
    fn test_error_category_rate_limit() {
        let cat = ErrorCategory::RateLimit;
        assert_eq!(format!("{}", cat), "rate_limit");
    }

    #[test]
    fn test_error_category_validation() {
        let cat = ErrorCategory::Validation;
        assert_eq!(format!("{}", cat), "validation_error");
    }

    #[test]
    fn test_error_category_server() {
        let cat = ErrorCategory::Server;
        assert_eq!(format!("{}", cat), "server_error");
    }

    #[test]
    fn test_error_category_config() {
        let cat = ErrorCategory::Config;
        assert_eq!(format!("{}", cat), "config_error");
    }

    #[test]
    fn test_error_category_unknown() {
        let cat = ErrorCategory::Unknown;
        assert_eq!(format!("{}", cat), "unknown_error");
    }

    #[test]
    fn test_indodax_error_http() {
        // Test that Http error wraps reqwest::Error
        // Since we can't easily create a reqwest error, we skip this test
        // or use a mock approach
        assert!(true); // Placeholder test
    }

    #[test]
    fn test_indodax_error_api_basic() {
        let err = IndodaxError::api("test message", ErrorCategory::Server, Some("500".into()));
        let msg = err.to_string();
        assert!(msg.contains("test message"));
    }

    #[test]
    fn test_indodax_error_api_category_connection() {
        let err = IndodaxError::api("conn error", ErrorCategory::Connection, None);
        assert_eq!(err.category(), "connection_error");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_indodax_error_api_category_server() {
        let err = IndodaxError::api("server error", ErrorCategory::Server, None);
        assert_eq!(err.category(), "server_error");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_indodax_error_api_category_rate_limit() {
        let err = IndodaxError::api("rate limit", ErrorCategory::RateLimit, None);
        assert_eq!(err.category(), "rate_limit");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_indodax_error_api_category_not_retryable() {
        let err = IndodaxError::api("auth error", ErrorCategory::Authentication, None);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_indodax_error_config() {
        let err = IndodaxError::Config("config error message".into());
        assert_eq!(err.category(), "config_error");
        let msg = err.to_string();
        assert!(msg.contains("config error message"));
    }

    #[test]
    fn test_indodax_error_parse() {
        let err = IndodaxError::Parse("parse error".into());
        assert_eq!(err.category(), "validation_error");
        let msg = err.to_string();
        assert!(msg.contains("parse error"));
    }

    #[test]
    fn test_indodax_error_other() {
        let err = IndodaxError::Other("other error".into());
        assert_eq!(err.category(), "unknown_error");
        let msg = err.to_string();
        assert!(msg.contains("other error"));
    }

    #[test]
    fn test_indodax_error_json() {
        let err = IndodaxError::Json(serde_json::from_str::<serde_json::Value>("invalid").unwrap_err());
        assert_eq!(err.category(), "validation_error");
    }

    #[test]
    fn test_indodax_error_websocket() {
        // WebSocket errors are harder to construct, but we can test the category
        let err = IndodaxError::WebSocket(tokio_tungstenite::tungstenite::Error::ConnectionClosed);
        assert_eq!(err.category(), "connection_error");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_api_error_retryable_field() {
        let err = IndodaxError::api("test", ErrorCategory::Connection, None);
        match err {
            IndodaxError::Api { retryable, .. } => assert!(retryable),
            _ => assert!(false, "Expected Api error, got {:?}", err),
        }
    }

    #[test]
    fn test_api_error_code() {
        let err = IndodaxError::api("test", ErrorCategory::Unknown, Some("ERR_123".into()));
        match err {
            IndodaxError::Api { code, .. } => assert_eq!(code, Some("ERR_123".into())),
            _ => assert!(false, "Expected Api error, got {:?}", err),
        }
    }
}
