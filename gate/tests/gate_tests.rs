use indodax_gate::{IndodaxGate, ExchangeGate};

#[tokio::test]
async fn test_indodax_gate_rest() {
    let gate = IndodaxGate::new();
    
    // Test list_pairs
    let pairs = gate.list_pairs().await.expect("failed to list pairs");
    assert!(!pairs.is_empty(), "symbols list should not be empty");
    
    // Check if btc_idr is in the list
    let btc_idr = pairs.iter().find(|p| p.symbol == "BTC/IDR");
    assert!(btc_idr.is_some(), "BTC/IDR should be in the list of pairs");
    let pair = btc_idr.unwrap();
    assert_eq!(pair.base, "BTC");
    assert_eq!(pair.quote, "IDR");

    // Test last_price
    let tick = gate.last_price("BTC/IDR").await.expect("failed to get last price");
    assert_eq!(tick.symbol, "BTC/IDR");
    assert!(tick.price > 0.0, "BTC/IDR price should be positive");
}
