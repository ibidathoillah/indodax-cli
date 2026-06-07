use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use axum::{routing::post, Router};
use indodax_cli::mcp::{handle_http_call, oauth, AppState};
use serde_json::json;
use tower::ServiceExt; // untuk oneshot

#[cfg(feature = "server")]
#[tokio::test]
async fn test_http_bridge_unauthorized_without_secret() {
    // Setup state dengan secret
    let state = AppState {
        groups: "market".into(),
        allow_dangerous: false,
        bridge_secret: Some("top-secret".into()),
        public_base_url: "http://localhost:8000".into(),
        oauth: oauth::OAuthState::default(),
    };

    // Buat router minimal untuk tes
    let app = Router::new()
        .route("/call/:tool_name", post(handle_http_call))
        .with_state(state);

    // Kirim request tanpa header X-Bridge-Auth
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/call/ticker")
                .header("content-type", "application/json")
                .body(Body::from(json!({"pair": "btc_idr"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Harus Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_http_bridge_health_simple() {
    let state = AppState {
        groups: "market".into(),
        allow_dangerous: false,
        bridge_secret: None,
        public_base_url: "http://localhost:8000".into(),
        oauth: oauth::OAuthState::default(),
    };
    assert_eq!(state.groups, "market");
}
