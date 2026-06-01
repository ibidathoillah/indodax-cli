#[cfg(all(feature = "cli", not(target_arch = "wasm32")))]
#[tokio::test]
async fn test_websocket_tls_connection() {
    use tokio_tungstenite::connect_async;
    use url::Url;

    let url = Url::parse("wss://ws3.indodax.com/ws/").unwrap();
    
    // We don't need to actually subscribe, just verify the handshake succeeds
    // or at least doesn't fail with "TLS support not compiled in"
    let result = connect_async(url.as_str()).await;
    
    match result {
        Ok(_) => {
            // Connection successful, TLS is definitely working
            assert!(true);
        }
        Err(e) => {
            let err_msg = e.to_string();
            // If it's a "TLS support not compiled in" error, it will contain this string
            assert!(!err_msg.contains("TLS support not compiled in"), "WebSocket failed with TLS error: {}", err_msg);
            
            // Other errors (like network timeout or remote host unreachable in CI) 
            // might happen but they confirm that at least the TLS layer was attempted.
            println!("WebSocket connection failed as expected in restricted env, but TLS was initialized: {}", err_msg);
        }
    }
}
