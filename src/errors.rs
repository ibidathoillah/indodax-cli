#[derive(Debug, thiserror::Error)]
pub enum ErrorCategory {
    #[error("connection_error")]
    Connection,
    #[error("authentication_error")]
    Authentication,
    #[error("authorization_error")]
    Authorization,
    #[error("rate_limit")]
    RateLimit,
    #[error("validation_error")]
    Validation,
    #[error("server_error")]
    Server,
    #[error("not_found")]
    NotFound,
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

    #[error("{0}")]
    Io(#[from] std::io::Error),

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

    pub fn category(&self) -> &str {
        match self {
            IndodaxError::Api { category, .. } => match category {
                ErrorCategory::Connection => "connection_error",
                ErrorCategory::Authentication => "authentication_error",
                ErrorCategory::Authorization => "authorization_error",
                ErrorCategory::RateLimit => "rate_limit",
                ErrorCategory::Validation => "validation_error",
                ErrorCategory::Server => "server_error",
                ErrorCategory::NotFound => "not_found",
                ErrorCategory::Config => "config_error",
                ErrorCategory::Unknown => "unknown_error",
            },
            IndodaxError::Http(_) => "connection_error",
            IndodaxError::WebSocket(_) => "connection_error",
            IndodaxError::Json(_) => "validation_error",
            IndodaxError::Config(_) => "config_error",
            IndodaxError::Parse(_) => "validation_error",
            IndodaxError::Io(_) => "io_error",
            IndodaxError::Other(_) => "unknown_error",
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
