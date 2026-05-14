# Price Alert Feature Plan

## Overview

Add a price alert system that monitors cryptocurrency prices and notifies users when conditions are met.

---

## 1. Data Model

### Alert Structure (`src/config.rs` or new `src/alerts.rs`)

```rust
struct PriceAlert {
    id: u64,
    pair: String,           // e.g., "btc_idr"
    condition: AlertCondition,
    created_at: u64,
    triggered_at: Option<u64>,
    status: AlertStatus,    // Active, Triggered, Cancelled
}

enum AlertCondition {
    Above(f64),   // Price > threshold
    Below(f64),   // Price < threshold
    ChangePercent { percent: f64, direction: AboveOrBelow },
}

enum AlertStatus {
    Active,
    Triggered,
    Cancelled,
}
```

### Storage

- Store alerts in `~/.config/indodax/alerts.json`
- Auto-save on modification
- Load on startup

---

## 2. CLI Commands

### Proposed Commands

```bash
# Add alert
indodax alert add -p btc_idr --above 100000000     # Price above 100M
indodax alert add -p btc_idr --below 50000000       # Price below 50M
indodax alert add -p btc_idr --change +5             # 5% increase
indodax alert add -p btc_idr --change -10           # 10% decrease

# List alerts
indodax alert list                                   # All active alerts
indodax alert list --history                         # Including triggered/cancelled

# Cancel alerts
indodax alert cancel -i 1                            # Cancel by ID
indodax alert cancel --all                           # Cancel all

# Check alerts (for monitoring)
indodax alert check                                   # Check all active alerts
indodax alert check -i 1                              # Check specific alert
```

### Command Structure

Add to `src/commands/`:
- `alert.rs` — new module with alert commands
- Update `mod.rs` — add `pub mod alert;`

Add to `Command` enum in `lib.rs`:
```rust
Alert {
    #[command(subcommand)]
    cmd: commands::alert::AlertCommand,
}
```

---

## 3. Core Logic

### `alert.rs` Module

```rust
pub fn add_alert(pair: &str, condition: Condition) -> Result<Alert>
pub fn list_alerts(include_history: bool) -> Vec<Alert>
pub fn cancel_alert(id: u64) -> Result<()>
pub fn cancel_all_alerts() -> Result<()>
pub fn check_alerts(client: &IndodaxClient) -> Result<Vec<TriggeredAlert>>
fn evaluate_alert(alert: &Alert, current_price: f64) -> bool
fn trigger_alert(alert: &mut Alert) -> Alert
fn save_alerts(alerts: &[Alert]) -> Result<()>
fn load_alerts() -> Result<Vec<Alert>>
```

---

## 4. Notification System

### Notification Methods (extensible)

1. **Console output** (default)
   - Print alert when triggered during `alert check`

2. **Webhook** (optional)
   - Send HTTP POST to configured URL
   - JSON payload with alert details

3. **System notification** (optional)
   - Use `notify-rust` crate for desktop notifications

### Implementation

```rust
trait Notifier {
    async fn notify(&self, alert: &TriggeredAlert) -> Result<()>;
}

struct ConsoleNotifier;
struct WebhookNotifier { url: String };
struct SystemNotifier;
```

---

## 5. Background Monitoring (Optional)

### Daemon Mode

```bash
indodax alert monitor --interval 60    # Check every 60 seconds
indodax alert monitor --daemon        # Run in background
```

### Implementation Options

1. **Polling mode** — Simple `tokio::time::interval` loop
2. **WebSocket mode** — Subscribe to ticker stream, evaluate on each update

---

## 6. MCP Integration

Add to MCP tools for AI agent support:

```json
{
  "name": "alert_add",
  "description": "Create a price alert",
  "inputSchema": {
    "pair": "btc_idr",
    "condition": "above" | "below" | "change",
    "value": 100000000
  }
}
```

---

## 7. File Changes

| File | Action |
|------|--------|
| `src/commands/alert.rs` | New — alert command implementation |
| `src/commands/mod.rs` | Update — add `pub mod alert;` |
| `src/lib.rs` | Update — add `Alert` command variant |
| `src/config.rs` | Update — add `alerts_path()` function |
| `src/alerts.rs` | New (optional) — separate alerts module if complex |

---

## 8. Dependencies

```toml
# Cargo.toml
notify-rust = "5"           # Optional: system notifications
tokio = { version = "1", features = ["full"] }
```

---

## 9. Implementation Order

1. **Phase 1: Basic CRUD**
   - Data model and storage
   - `alert add` command
   - `alert list` command
   - `alert cancel` command

2. **Phase 2: Alert Evaluation**
   - `alert check` command with price comparison
   - Trigger logic (Above/Below/Condition)

3. **Phase 3: Notifications**
   - Console output on trigger
   - Webhook support (optional)

4. **Phase 4: MCP Integration**
   - Add alert tools to MCP server

5. **Phase 5: Background Mode** (optional)
   - Daemon/monitor mode

---

## 10. Edge Cases

- **Invalid pair**: Validate against available pairs
- **Price at exact threshold**: Trigger on `>=` or `>` as specified
- **Network failure**: Retry with backoff, log failures
- **Duplicate alerts**: Allow duplicates, use unique IDs
- **Stale price data**: Show last update timestamp

---

## 11. Example Usage

```bash
# Set up alerts
indodax alert add -p btc_idr --above 150000000
indodax alert add -p btc_idr --below 40000000

# Monitor
indodax alert check
# [ALERT] btc_idr above 150000000 (current: 155000000)

# List all
indodax alert list
```