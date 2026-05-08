use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

const DEFAULT_BALANCE_IDR: f64 = 100_000_000.0;
const DEFAULT_BALANCE_BTC: f64 = 1.0;
const DEFAULT_ETH: f64 = 10.0;
const DEFAULT_USDT: f64 = 50_000.0;
const TAKER_FEE: f64 = 0.0026;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOrder {
    pub id: u64, pub pair: String, pub side: String, pub price: f64, pub amount: f64,
    pub remaining: f64, pub order_type: String, pub status: String, pub created_at: u64,
    #[serde(default)] pub fees_paid: f64, #[serde(default)] pub filled_price: f64,
    #[serde(default)] pub total_spent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperState {
    pub balances: HashMap<String, f64>, pub orders: Vec<PaperOrder>,
    pub next_order_id: u64, pub trade_count: u64,
    #[serde(default)] pub total_fees_paid: f64,
    #[serde(default)] pub initial_balances: Option<HashMap<String, f64>>,
}

impl Default for PaperState {
    fn default() -> Self {
        let mut b = HashMap::new();
        b.insert("idr".into(), DEFAULT_BALANCE_IDR);
        b.insert("btc".into(), DEFAULT_BALANCE_BTC);
        b.insert("eth".into(), DEFAULT_ETH);
        b.insert("usdt".into(), DEFAULT_USDT);
        let initial = b.clone();
        Self {
            balances: b,
            orders: Vec::new(),
            next_order_id: 1,
            trade_count: 0,
            total_fees_paid: 0.0,
            initial_balances: Some(initial),
        }
    }
}

#[wasm_bindgen]
pub struct PaperTrader { state: PaperState }

#[wasm_bindgen]
impl PaperTrader {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { state: PaperState::default() } }

    #[wasm_bindgen]
    pub fn init(&mut self) -> JsValue {
        self.state = PaperState::default();
        self.get_state_js()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        self.state = PaperState::default();
        self.get_state_js()
    }

    #[wasm_bindgen]
    pub fn topup(&mut self, currency: &str, amount: f64) -> JsValue {
        let balance = self.state.balances.entry(currency.to_lowercase()).or_insert(0.0);
        *balance += amount;
        self.get_state_js()
    }

    #[wasm_bindgen]
    pub fn get_balances(&self) -> JsValue {
        if self.state.balances.is_empty() {
            let mut default_balances = std::collections::HashMap::new();
            default_balances.insert("idr".to_string(), 100_000_000.0);
            default_balances.insert("btc".to_string(), 1.0);
            default_balances.insert("eth".to_string(), 10.0);
            default_balances.insert("usdt".to_string(), 50_000.0);
            serde_wasm_bindgen::to_value(&default_balances).unwrap_or(JsValue::NULL)
        } else {
            serde_wasm_bindgen::to_value(&self.state.balances).unwrap_or(JsValue::NULL)
        }
    }

    #[wasm_bindgen]
    pub fn get_orders(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.state.orders).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn get_state(&self) -> JsValue {
        self.get_state_js()
    }

    fn get_state_js(&self) -> JsValue {
        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("balances"),
            &serde_wasm_bindgen::to_value(&self.state.balances).unwrap_or(JsValue::NULL));
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("orders"),
            &serde_wasm_bindgen::to_value(&self.state.orders).unwrap_or(JsValue::NULL));
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("trade_count"),
            &JsValue::from(self.state.trade_count));
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("total_fees_paid"),
            &JsValue::from(self.state.total_fees_paid));
        obj.into()
    }

    #[wasm_bindgen]
    pub fn buy(&mut self, pair: &str, price: f64, amount: f64) -> Result<JsValue, JsValue> {
        self.place_order_internal(pair, "buy", price, amount)
    }

    #[wasm_bindgen]
    pub fn sell(&mut self, pair: &str, price: f64, amount: f64) -> Result<JsValue, JsValue> {
        self.place_order_internal(pair, "sell", price, amount)
    }

    fn place_order_internal(&mut self, pair: &str, side: &str, price: f64, amount: f64) -> Result<JsValue, JsValue> {
        if price <= 0.0 { return Err(JsValue::from_str("Price must be greater than 0")); }
        if amount <= 0.0 { return Err(JsValue::from_str("Amount must be greater than 0")); }

        let base = pair.split('_').next().unwrap_or(pair);
        let quote = pair.split('_').last().unwrap_or("idr");
        let total = price * amount;

        if side == "buy" {
            let b = self.state.balances.entry(quote.to_string()).or_insert(0.0);
            if *b < total {
                return Err(JsValue::from_str(&format!(
                    "Insufficient {} balance. Need {:.2}, have {:.2}",
                    quote.to_uppercase(), total, b
                )));
            }
            *b -= total;
        } else {
            let b = self.state.balances.entry(base.to_string()).or_insert(0.0);
            if *b < amount {
                return Err(JsValue::from_str(&format!(
                    "Insufficient {} balance. Need {:.8}, have {:.8}",
                    base.to_uppercase(), amount, b
                )));
            }
            *b -= amount;
        }

        let id = self.state.next_order_id;
        self.state.next_order_id += 1;
        let now = js_sys::Date::now() as u64;

        self.state.orders.push(PaperOrder {
            id,
            pair: pair.to_string(),
            side: side.to_string(),
            price,
            amount,
            remaining: amount,
            order_type: "limit".into(),
            status: "pending".into(),
            created_at: now,
            fees_paid: 0.0,
            filled_price: 0.0,
            total_spent: total,
        });

        let r = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("success"), &JsValue::from_bool(true));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("order_id"), &JsValue::from(id));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("status"), &JsValue::from_str("pending"));
        Ok(r.into())
    }

    #[wasm_bindgen]
    pub fn check_fills(&mut self, market_prices: JsValue) -> Result<JsValue, JsValue> {
        let prices: HashMap<String, f64> = serde_wasm_bindgen::from_value(market_prices)
            .map_err(|e| JsValue::from_str(&format!("Invalid market prices: {}", e)))?;

        let fill_ids: Vec<u64> = self.state.orders.iter()
            .filter(|o| o.status == "pending")
            .filter_map(|o| {
                let current_price = prices.get(&o.pair)?;
                let should_fill = match o.side.as_str() {
                    "buy" => *current_price <= o.price,
                    "sell" => *current_price >= o.price,
                    _ => false,
                };
                if should_fill { Some(o.id) } else { None }
            })
            .collect();

        for id in &fill_ids {
            self.execute_fill(*id)?;
        }

        let r = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("filled_count"),
            &JsValue::from(fill_ids.len() as u32));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("filled_ids"),
            &serde_wasm_bindgen::to_value(&fill_ids).unwrap_or(JsValue::NULL));
        Ok(r.into())
    }

    fn execute_fill(&mut self, order_id: u64) -> Result<(), JsValue> {
        let (base, quote, side, price, amount) = {
            let order = self.state.orders.iter().find(|o| o.id == order_id)
                .ok_or_else(|| JsValue::from_str("Order not found"))?;
            (order.pair.split('_').next().unwrap().to_string(),
             order.pair.split('_').last().unwrap_or("idr").to_string(),
             order.side.clone(),
             order.price,
             order.remaining)
        };

        let total = price * amount;
        let fee = total * TAKER_FEE;

        if side == "buy" {
            *self.state.balances.entry(base.clone()).or_insert(0.0) += amount;
            *self.state.balances.entry(quote.clone()).or_insert(0.0) -= fee;
        } else {
            *self.state.balances.entry(quote.clone()).or_insert(0.0) += total - fee;
        }

        if let Some(order) = self.state.orders.iter_mut().find(|o| o.id == order_id) {
            order.remaining = 0.0;
            order.status = "filled".to_string();
            order.fees_paid = fee;
            order.filled_price = price;
        }

        self.state.total_fees_paid += fee;
        self.state.trade_count += 1;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn cancel_order(&mut self, order_id: u64) -> Result<JsValue, JsValue> {
        let order = self.state.orders.iter().find(|o| o.id == order_id)
            .ok_or_else(|| JsValue::from_str("Order not found"))?;

        if order.status != "pending" {
            return Err(JsValue::from_str(&format!("Order already {}", order.status)));
        }

        let base = order.pair.split('_').next().unwrap_or("btc");
        let quote = order.pair.split('_').last().unwrap_or("idr");
        let refund = order.price * order.remaining;

        if order.side == "buy" {
            *self.state.balances.entry(quote.to_string()).or_insert(0.0) += refund;
        } else {
            *self.state.balances.entry(base.to_string()).or_insert(0.0) += order.remaining;
        }

        if let Some(order) = self.state.orders.iter_mut().find(|o| o.id == order_id) {
            order.status = "cancelled".to_string();
            order.remaining = 0.0;
        }

        let r = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("success"), &JsValue::from_bool(true));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("order_id"), &JsValue::from(order_id));
        Ok(r.into())
    }

    #[wasm_bindgen]
    pub fn get_status(&self) -> JsValue {
        let fc = self.state.orders.iter().filter(|o| o.status == "filled").count();
        let pc = self.state.orders.iter().filter(|o| o.status == "pending").count();
        let cc = self.state.orders.iter().filter(|o| o.status == "cancelled").count();
        let r = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("trade_count"), &JsValue::from(self.state.trade_count));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("filled_count"), &JsValue::from(fc as u32));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("pending_count"), &JsValue::from(pc as u32));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("cancelled_count"), &JsValue::from(cc as u32));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("total_fees_paid"), &JsValue::from(self.state.total_fees_paid));
        let _ = js_sys::Reflect::set(&r, &JsValue::from_str("total_orders"), &JsValue::from(self.state.orders.len() as u32));
        r.into()
    }

    #[wasm_bindgen]
    pub fn get_initial_balances(&self) -> JsValue {
        match &self.state.initial_balances {
            Some(b) => serde_wasm_bindgen::to_value(b).unwrap_or(JsValue::NULL),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn save_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn load_state(&mut self, json: &str) -> Result<JsValue, JsValue> {
        self.state = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
        if self.state.initial_balances.is_none() {
            let mut b = HashMap::new();
            b.insert("idr".into(), DEFAULT_BALANCE_IDR);
            b.insert("btc".into(), DEFAULT_BALANCE_BTC);
            b.insert("eth".into(), DEFAULT_ETH);
            b.insert("usdt".into(), DEFAULT_USDT);
            self.state.initial_balances = Some(b);
        }
        Ok(self.get_state_js())
    }
}

impl Default for PaperTrader { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topup() {
        let mut state = PaperState::default();
        assert_eq!(state.balances.get("idr"), Some(&100_000_000.0));
        
        let currency = "idr".to_string();
        let amount = 50000000.0;
        let balance = state.balances.entry(currency.to_lowercase()).or_insert(0.0);
        *balance += amount;
        
        assert_eq!(state.balances.get("idr"), Some(&150_000_000.0));
    }
}
