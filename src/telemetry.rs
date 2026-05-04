const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn user_agent() -> String {
    format!("{}/{}", APP_NAME, APP_VERSION)
}

pub fn client_identifier() -> String {
    format!("indodax-cli/{}", APP_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agent() {
        let ua = user_agent();
        assert!(ua.contains("indodax-cli"));
        assert!(ua.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_client_identifier() {
        let id = client_identifier();
        assert!(id.contains("indodax-cli/"));
        assert!(id.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_user_agent_format() {
        let ua = user_agent();
        let parts: Vec<&str> = ua.split('/').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "indodax-cli");
    }

    #[test]
    fn test_client_identifier_format() {
        let id = client_identifier();
        let parts: Vec<&str> = id.split('/').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "indodax-cli");
    }
}
