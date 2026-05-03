const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn user_agent() -> String {
    format!("{}/{}", APP_NAME, APP_VERSION)
}

pub fn client_identifier() -> String {
    format!("indodax-cli/{}", APP_VERSION)
}
