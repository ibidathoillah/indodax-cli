use std::io::IsTerminal;

use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use anyhow::Result;
use std::collections::HashMap;

async fn get_account_info(client: &IndodaxClient) -> Result<serde_json::Value> {
    let params = HashMap::new();
    Ok(client.private_post_v1("getInfo", &params).await?)
}

#[derive(Debug, clap::Subcommand)]
pub enum TradeCommand {
    #[command(name = "buy", about = "Place a buy order")]
    Buy {
        #[arg(short, long)]
        pair: String,
        #[arg(short = 'i', long, help = "The total IDR amount to spend.")]
        idr: f64,
        #[arg(long, help = "Limit price. If omitted, a market order will be placed.")]
        price: Option<f64>,
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(["limit", "market", "stoplimit"]), help = "Order type: limit, market, or stoplimit. Inferred from --price/--stop-price if omitted.")]
        order_type: Option<String>,
        #[arg(long, help = "Trigger price for stop-limit orders.")]
        stop_price: Option<f64>,
        #[arg(long, help = "Optional: Custom client order ID for tracking.")]
        client_id: Option<String>,
        #[arg(long, help = "Validate order without executing (dry-run).")]
        validate: bool,
    },

    #[command(name = "sell", about = "Place a sell order")]
    Sell {
        #[arg(short, long)]
        pair: String,
        #[arg(short = 'a', long, help = "Amount in base currency (e.g. BTC)")]
        amount: f64,
        #[arg(
            short = 'r',
            long,
            help = "Limit price. If omitted, a market order will be placed."
        )]
        price: Option<f64>,
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(["limit", "market", "stoplimit"]), help = "Order type: limit, market, or stoplimit. Inferred from --price/--stop-price if omitted.")]
        order_type: Option<String>,
        #[arg(long, help = "Trigger price for stop-limit orders.")]
        stop_price: Option<f64>,
        #[arg(long, help = "Optional: Custom client order ID for tracking.")]
        client_id: Option<String>,
        #[arg(long, help = "Validate order without executing (dry-run).")]
        validate: bool,
    },

    #[command(name = "open", about = "List open orders")]
    Open {
        #[arg(short, long, help = "Filter by trading pair")]
        pair: Option<String>,
    },

    #[command(name = "closed", about = "List closed orders (history)")]
    Closed {
        #[arg(short, long, default_value = "btc_idr")]
        pair: String,
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },

    #[command(name = "history", about = "Get order history")]
    History {
        #[arg(short, long, default_value = "btc_idr")]
        pair: String,
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },

    #[command(name = "get", about = "Get order details by order ID")]
    Get {
        #[arg(long)]
        order_id: u64,
        #[arg(long)]
        pair: String,
    },

    #[command(name = "get-by-client-id", about = "Get order details by client order ID")]
    GetByClientId {
        #[arg(long)]
        client_order_id: String,
    },

    #[command(name = "cancel", about = "Cancel an order by ID")]
    Cancel {
        #[arg(short = 'i', long)]
        order_id: u64,
        #[arg(short = 'p', long)]
        pair: String,
        #[arg(short = 't', long, help = "Order side: buy or sell")]
        order_type: String,
    },

    #[command(
        name = "cancel-by-client-id",
        about = "Cancel an order by client order ID"
    )]
    CancelByClientId {
        #[arg(long)]
        client_order_id: String,
    },

    #[command(
        name = "cancel-batch",
        about = "Cancel multiple orders by ID"
    )]
    CancelBatch {
        #[arg(short = 'p', long)]
        pair: String,
        #[arg(short = 'i', long, help = "Comma-separated order IDs")]
        order_ids: String,
        #[arg(short = 't', long, help = "Order side for ALL IDs: buy or sell")]
        order_type: String,
    },

    #[command(
        name = "cancel-all",
        about = "Cancel all open orders, optionally filtered by pair"
    )]
    CancelAll {
        #[arg(
            short,
            long,
            help = "Only cancel orders for this trading pair (e.g. btc_idr)"
        )]
        pair: Option<String>,
    },

    #[command(name = "edit", about = "Edit an existing order (Cancel + Replace)")]
    Edit {
        #[arg(long)]
        order_id: u64,
        #[arg(long)]
        pair: String,
        #[arg(long, help = "Order side: buy or sell")]
        order_type: String,
        #[arg(long, help = "New limit price")]
        price: f64,
        #[arg(long, help = "New amount (base for sell, IDR for buy)")]
        amount: f64,
    },

    #[command(name = "countdown", about = "Start deadman switch countdown")]
    CountdownCancelAll {
        #[arg(short, long)]
        pair: Option<String>,
        #[arg(short, long, help = "Countdown in milliseconds (0 to disable)")]
        countdown_time: u64,
    },

    #[command(
        hide = true,
        name = "get-order-by-client-id",
        about = "Get order details by client order ID"
    )]
    GetOrderByClientId {
        #[arg(long)]
        client_order_id: String,
    },
}

pub async fn execute(
    client: &IndodaxClient,
    cmd: &TradeCommand,
    yes: bool,
) -> Result<CommandOutput> {
    match cmd {
        TradeCommand::Buy {
            pair,
            idr,
            price,
            order_type,
            stop_price,
            client_id,
            validate,
        } => {
            let pair = helpers::normalize_pair(pair);
            place_buy_order(
                client,
                &pair,
                *idr,
                *price,
                order_type.as_deref(),
                *stop_price,
                client_id.as_deref(),
                *validate,
            )
            .await
        }
        TradeCommand::Sell {
            pair,
            price,
            amount,
            order_type,
            stop_price,
            client_id,
            validate,
        } => {
            let pair = helpers::normalize_pair(pair);
            place_sell_order(
                client,
                &pair,
                *price,
                *amount,
                order_type.as_deref(),
                *stop_price,
                client_id.as_deref(),
                *validate,
            )
            .await
        }
        TradeCommand::Open { pair } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            super::account::open_orders(client, pair.as_deref()).await
        }
        TradeCommand::Closed { pair, limit } | TradeCommand::History { pair, limit } => {
            let pair = helpers::normalize_pair(pair);
            super::account::order_history(client, &pair, *limit).await
        }
        TradeCommand::Get { order_id, pair } => {
            let pair = helpers::normalize_pair(pair);
            super::account::get_order(client, *order_id, &pair).await
        }
        TradeCommand::GetByClientId { client_order_id } | TradeCommand::GetOrderByClientId { client_order_id } => {
            get_order_by_client_id(client, client_order_id).await
        }
        TradeCommand::Cancel {
            order_id,
            pair,
            order_type,
        } => {
            let pair = helpers::normalize_pair(pair);
            cancel_order(client, *order_id, &pair, order_type).await
        }
        TradeCommand::CancelByClientId { client_order_id } => {
            cancel_by_client_id(client, client_order_id).await
        }
        TradeCommand::CancelBatch { pair, order_ids, order_type } => {
            let pair = helpers::normalize_pair(pair);
            cancel_batch(client, &pair, order_ids, order_type).await
        }
        TradeCommand::CancelAll { pair } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            cancel_all_orders(client, pair.as_deref(), yes).await
        }
        TradeCommand::Edit { order_id, pair, order_type, price, amount } => {
            let pair = helpers::normalize_pair(pair);
            edit_order(client, *order_id, &pair, order_type, *price, *amount).await
        }
        TradeCommand::CountdownCancelAll {
            pair,
            countdown_time,
        } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            countdown_cancel_all(client, pair.as_deref(), *countdown_time).await
        }
    }
}

async fn get_order_by_client_id(
    client: &IndodaxClient,
    client_order_id: &str,
) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    params.insert("client_order_id".into(), client_order_id.to_string());

    let data: serde_json::Value = client
        .private_post_v1("getOrderByClientOrderId", &params)
        .await?;

    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

pub(crate) async fn place_buy_order(
    client: &IndodaxClient,
    pair: &str,
    idr_amount: f64,
    price: Option<f64>,
    explicit_type: Option<&str>,
    stop_price: Option<f64>,
    client_id: Option<&str>,
    validate: bool,
) -> Result<CommandOutput> {
    let info = get_account_info(client).await?;

    if idr_amount <= 0.0 {
        return Err(anyhow::anyhow!(
            "IDR amount must be positive, got {}",
            idr_amount
        ));
    }

    let idr_balance = helpers::parse_balance(&info, "idr");

    if idr_balance + helpers::BALANCE_EPSILON < idr_amount {
        return Err(anyhow::anyhow!(
            "Insufficient IDR balance. Need {:.2}, have {:.2}",
            idr_amount,
            idr_balance
        ));
    }

    let mut warnings: Vec<String> = Vec::new();
    let mut params = HashMap::new();
    params.insert("pair".to_string(), pair.to_string());
    params.insert("type".to_string(), "buy".to_string());
    params.insert("idr".to_string(), idr_amount.to_string());
    if let Some(cid) = client_id {
        params.insert("client_order_id".to_string(), cid.to_string());
    }

    let order_type_str = if let Some(order_type) = explicit_type {
        match order_type {
            "limit" => {
                let p = price.ok_or_else(|| {
                    anyhow::anyhow!("--price is required when --order-type is 'limit'")
                })?;
                if p <= 0.0 {
                    return Err(anyhow::anyhow!("Price must be positive, got {}", p));
                }
                if let Some(warning) = helpers::validate_tick_size(client, pair, p).await {
                    warnings.push(warning);
                }
                params.insert("price".to_string(), p.to_string());
                "limit"
            }
            "market" => {
                if price.is_some() {
                    warnings.push("[TRADE] Warning: --price specified but order type is 'market'. Price will be ignored for market orders.".to_string());
                }
                warnings.push("[TRADE] Warning: Market buy order without limit price. Indodax may reject market orders with IDR amount. Consider setting a limit price.".to_string());
                params.insert("order_type".to_string(), "market".to_string());
                "market"
            }
            "stoplimit" => {
                let p = price.ok_or_else(|| {
                    anyhow::anyhow!("--price is required when --order-type is 'stoplimit'")
                })?;
                let sp = stop_price.ok_or_else(|| {
                    anyhow::anyhow!("--stop-price is required when --order-type is 'stoplimit'")
                })?;
                params.insert("price".to_string(), p.to_string());
                params.insert("stop_price".to_string(), sp.to_string());
                params.insert("order_type".to_string(), "stoplimit".to_string());
                "stoplimit"
            }
            _ => unreachable!(),
        }
    } else if let Some(sp) = stop_price {
        let p = price.ok_or_else(|| {
            anyhow::anyhow!("--price (limit price) is required for stop-limit orders")
        })?;
        params.insert("price".to_string(), p.to_string());
        params.insert("stop_price".to_string(), sp.to_string());
        params.insert("order_type".to_string(), "stoplimit".to_string());
        "stoplimit"
    } else if let Some(p) = price {
        if p <= 0.0 {
            return Err(anyhow::anyhow!("Price must be positive, got {}", p));
        }
        if let Some(warning) = helpers::validate_tick_size(client, pair, p).await {
            warnings.push(warning);
        }
        params.insert("price".to_string(), p.to_string());
        "limit"
    } else {
        warnings.push("[TRADE] Warning: Market buy order without limit price. Indodax may reject market orders with IDR amount. Consider setting a limit price.".to_string());
        params.insert("order_type".to_string(), "market".to_string());
        "market"
    };

    if validate {
        let headers = vec!["Field".into(), "Value".into()];
        let mut rows = vec![
            vec!["Status".into(), "PREVIEW (Not executed)".into()],
            vec!["Side".into(), "BUY".into()],
            vec!["Pair".into(), pair.to_uppercase()],
            vec!["Type".into(), order_type_str.to_uppercase()],
            vec!["Amount (IDR)".into(), format!("{:.0}", idr_amount)],
        ];
        if let Some(p) = params.get("price") {
            rows.push(vec!["Limit Price".into(), p.clone()]);
        }
        if let Some(sp) = params.get("stop_price") {
            rows.push(vec!["Stop Price".into(), sp.clone()]);
        }
        if let Some(cid) = client_id {
            rows.push(vec!["Client ID".into(), cid.to_string()]);
        }
        
        let mut output = CommandOutput::new(serde_json::json!(params), headers, rows);
        for w in warnings {
            output = output.with_warning(w);
        }
        return Ok(output);
    }

    let data: serde_json::Value = client.private_post_v1("trade", &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    let mut output = CommandOutput::new(data, headers, rows).with_addendum(format!(
        "Buy order ({}) placed for {} IDR on pair {}",
        order_type_str, idr_amount, pair
    ));
    for w in warnings {
        output = output.with_warning(w);
    }
    Ok(output)
}

pub(crate) async fn place_sell_order(
    client: &IndodaxClient,
    pair: &str,
    price: Option<f64>,
    amount: f64,
    explicit_type: Option<&str>,
    stop_price: Option<f64>,
    client_id: Option<&str>,
    validate: bool,
) -> Result<CommandOutput> {
    let base_currency = pair.split('_').next().unwrap_or_default();
    if base_currency.is_empty() {
        return Err(anyhow::anyhow!("Invalid pair format: {}", pair));
    }

    let info = get_account_info(client).await?;

    if amount <= 0.0 {
        return Err(anyhow::anyhow!("Amount must be positive, got {}", amount));
    }

    let base_balance = helpers::parse_balance(&info, base_currency);

    if base_balance + helpers::BALANCE_EPSILON < amount {
        return Err(anyhow::anyhow!(
            "Insufficient {} balance. Need {:.8}, have {:.8}",
            base_currency.to_uppercase(),
            amount,
            base_balance
        ));
    }

    let mut warnings: Vec<String> = Vec::new();
    let mut params = HashMap::new();
    params.insert("pair".to_string(), pair.to_string());
    params.insert("type".to_string(), "sell".to_string());
    params.insert(base_currency.to_string(), amount.to_string());
    if let Some(cid) = client_id {
        params.insert("client_order_id".to_string(), cid.to_string());
    }

    let order_type = if let Some(order_type) = explicit_type {
        match order_type {
            "limit" => {
                let p = price.ok_or_else(|| {
                    anyhow::anyhow!("--price is required when --order-type is 'limit'")
                })?;
                if p <= 0.0 {
                    return Err(anyhow::anyhow!("Price must be positive, got {}", p));
                }
                if let Some(warning) = helpers::validate_tick_size(client, pair, p).await {
                    warnings.push(warning);
                }
                params.insert("price".to_string(), p.to_string());
                "limit"
            }
            "market" => {
                if price.is_some() {
                    warnings.push("[TRADE] Warning: --price specified but order type is 'market'. Price will be ignored for market orders.".to_string());
                }
                params.insert("order_type".to_string(), "market".to_string());
                "market"
            }
            "stoplimit" => {
                let p = price.ok_or_else(|| {
                    anyhow::anyhow!("--price is required when --order-type is 'stoplimit'")
                })?;
                let sp = stop_price.ok_or_else(|| {
                    anyhow::anyhow!("--stop-price is required when --order-type is 'stoplimit'")
                })?;
                params.insert("price".to_string(), p.to_string());
                params.insert("stop_price".to_string(), sp.to_string());
                params.insert("order_type".to_string(), "stoplimit".to_string());
                "stoplimit"
            }
            _ => unreachable!(),
        }
    } else if let Some(sp) = stop_price {
        let p = price.ok_or_else(|| {
            anyhow::anyhow!("--price (limit price) is required for stop-limit orders")
        })?;
        params.insert("price".to_string(), p.to_string());
        params.insert("stop_price".to_string(), sp.to_string());
        params.insert("order_type".to_string(), "stoplimit".to_string());
        "stoplimit"
    } else if let Some(p) = price {
        if p <= 0.0 {
            return Err(anyhow::anyhow!("Price must be positive, got {}", p));
        }
        if let Some(warning) = helpers::validate_tick_size(client, pair, p).await {
            warnings.push(warning);
        }
        params.insert("price".to_string(), p.to_string());
        "limit"
    } else {
        params.insert("order_type".to_string(), "market".to_string());
        "market"
    };

    if validate {
        let headers = vec!["Field".into(), "Value".into()];
        let mut rows = vec![
            vec!["Status".into(), "PREVIEW (Not executed)".into()],
            vec!["Side".into(), "SELL".into()],
            vec!["Pair".into(), pair.to_uppercase()],
            vec!["Type".into(), order_type.to_uppercase()],
            vec!["Amount (Base)".into(), format!("{:.8}", amount)],
        ];
        if let Some(p) = params.get("price") {
            rows.push(vec!["Limit Price".into(), p.clone()]);
        }
        if let Some(sp) = params.get("stop_price") {
            rows.push(vec!["Stop Price".into(), sp.clone()]);
        }
        if let Some(cid) = client_id {
            rows.push(vec!["Client ID".into(), cid.to_string()]);
        }
        
        let mut output = CommandOutput::new(serde_json::json!(params), headers, rows);
        for w in warnings {
            output = output.with_warning(w);
        }
        return Ok(output);
    }

    let data: serde_json::Value = client.private_post_v1("trade", &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    let addendum = if let Some(p) = price {
        format!(
            "Sell order placed: {} {} @ {} ({})",
            amount, pair, p, order_type
        )
    } else {
        format!(
            "Sell order ({}) placed for {} {} on pair {}",
            order_type,
            amount,
            pair.split('_').next().unwrap_or(""),
            pair
        )
    };

    let mut output = CommandOutput::new(data, headers, rows).with_addendum(addendum);
    for w in warnings {
        output = output.with_warning(w);
    }
    Ok(output)
}

async fn cancel_order(
    client: &IndodaxClient,
    order_id: u64,
    pair: &str,
    order_type: &str,
) -> Result<CommandOutput> {
    let normalized = order_type.to_lowercase();
    if normalized != "buy" && normalized != "sell" {
        return Err(anyhow::anyhow!(
            "Invalid order type '{}'. Must be 'buy' or 'sell'",
            order_type
        ));
    }

    let mut params = HashMap::new();
    params.insert("symbol".into(), helpers::normalize_pair_v2(pair));

    let path = format!("/api/v2/order/{}", order_id);
    let data: serde_json::Value = client.private_delete_v2(&path, &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    Ok(
        CommandOutput::new(data, headers, rows).with_addendum(format!(
            "Cancelled {} order {} on {}",
            order_type, order_id, pair
        )),
    )
}

async fn cancel_by_client_id(
    client: &IndodaxClient,
    client_order_id: &str,
) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    params.insert("client_order_id".into(), client_order_id.to_string());

    let data: serde_json::Value = client
        .private_post_v1("cancelByClientOrderId", &params)
        .await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    Ok(
        CommandOutput::new(data, headers, rows).with_addendum(format!(
            "Cancelled order by client order ID: {}",
            client_order_id
        )),
    )
}

async fn cancel_all_orders(
    client: &IndodaxClient,
    pair: Option<&str>,
    force: bool,
) -> Result<CommandOutput> {
    if pair.is_none() && !force {
        if std::io::stdin().is_terminal() {
            use dialoguer::Confirm;
            let confirmed = Confirm::new()
                .with_prompt("No --pair filter specified. This will cancel ALL orders across ALL pairs. Continue?")
                .default(false)
                .interact()
                .unwrap_or(false);
            if !confirmed {
                return Ok(CommandOutput::json(serde_json::json!({
                    "cancelled": false,
                    "reason": "user_cancelled",
                }))
                .with_addendum("Cancel all orders aborted by user."));
            }
        } else {
            return Err(anyhow::anyhow!("No --pair filter specified and --force not used in non-interactive mode. Refusing to cancel all orders across all pairs."));
        }
    }

    let (cancelled_ids, failed_ids) = helpers::cancel_all_open_orders(client, pair)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let headers = vec!["Metric".into(), "Value".into()];
    let mut rows = vec![
        vec!["Cancelled".into(), cancelled_ids.len().to_string()],
        vec!["Order IDs".into(), cancelled_ids.join(", ")],
    ];
    if !failed_ids.is_empty() {
        rows.push(vec!["Failed".into(), failed_ids.len().to_string()]);
        rows.push(vec!["Failed IDs".into(), failed_ids.join(", ")]);
    }

    let data = serde_json::json!({
        "cancelled_count": cancelled_ids.len(),
        "cancelled_ids": cancelled_ids,
        "failed_count": failed_ids.len(),
        "failed_ids": failed_ids,
    });

    let addendum = if failed_ids.is_empty() {
        format!("Cancelled {} order(s)", cancelled_ids.len())
    } else {
        format!(
            "Cancelled {} order(s), {} failed",
            cancelled_ids.len(),
            failed_ids.len()
        )
    };

    Ok(
        CommandOutput::new(data, headers, rows).with_addendum(addendum)
    )
    }

    async fn cancel_batch(
        client: &IndodaxClient,
        pair: &str,
        order_ids: &str,
        _order_type: &str,
    ) -> Result<CommandOutput> {
        let ids: Vec<&str> = order_ids.split(',').map(|s| s.trim()).collect();
        let mut results = Vec::new();
        let mut success_ids = Vec::new();
        let mut fail_ids = Vec::new();

        for id in ids {
            if let Ok(order_id) = id.parse::<u64>() {
                let mut params = HashMap::new();
                params.insert("symbol".into(), helpers::normalize_pair_v2(pair));

                let path = format!("/api/v2/order/{}", order_id);
                match client.private_delete_v2::<serde_json::Value>(&path, &params).await {
                    Ok(_) => {
                        success_ids.push(id.to_string());
                    }
                    Err(e) => {
                        fail_ids.push(format!("{}: {}", id, e));
                    }
                }
            } else {
                fail_ids.push(format!("{}: invalid_id", id));
            }
        }

        let data = serde_json::json!({
            "success_ids": success_ids,
            "fail_ids": fail_ids,
        });

        let headers = vec!["Status".into(), "Count".into(), "IDs/Details".into()];
        results.push(vec!["Success".into(), success_ids.len().to_string(), success_ids.join(", ")]);
        results.push(vec!["Failed".into(), fail_ids.len().to_string(), fail_ids.join(", ")]);

        Ok(CommandOutput::new(data, headers, results).with_addendum(format!(
            "Batch cancel completed: {} success, {} failed",
            success_ids.len(), fail_ids.len()
        )))
    }

    async fn edit_order(
        client: &IndodaxClient,
        order_id: u64,
        pair: &str,
        order_type: &str,
        new_price: f64,
        new_amount: f64,
    ) -> Result<CommandOutput> {
        eprintln!("Editing order {}: Cancelling old order...", order_id);

        // 1. Cancel old order
        let mut cancel_params = HashMap::new();
        cancel_params.insert("symbol".into(), helpers::normalize_pair_v2(pair));

        let path = format!("/api/v2/order/{}", order_id);
        client.private_delete_v2::<serde_json::Value>(&path, &cancel_params).await?;

        eprintln!("Old order cancelled. Placing new order...");

        // 2. Place new order
        if order_type.to_lowercase() == "buy" {
            place_buy_order(client, pair, new_amount, Some(new_price), None, None, None, false).await
        } else {
            place_sell_order(client, pair, Some(new_price), new_amount, None, None, None, false).await
        }
    }

async fn countdown_cancel_all(
    client: &IndodaxClient,
    pair: Option<&str>,
    countdown_time: u64,
) -> Result<CommandOutput> {
    let data = client
        .countdown_cancel_all(pair, countdown_time)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let msg = if countdown_time == 0 {
        "Deadman switch disabled".into()
    } else {
        format!("Deadman switch active: {}ms countdown", countdown_time)
    };

    Ok(CommandOutput::json(data).with_addendum(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_command_variants() {
        let _cmd1 = TradeCommand::Buy {
            pair: "btc_idr".into(),
            idr: 100_000.0,
            price: Some(100_000_000.0),
            order_type: None,
            stop_price: None,
            client_id: None,
        };
        let _cmd2 = TradeCommand::Sell {
            pair: "btc_idr".into(),
            price: Some(100_000_000.0),
            amount: 0.5,
            order_type: None,
            stop_price: None,
            client_id: None,
        };
        let _cmd3 = TradeCommand::Cancel {
            order_id: 123,
            pair: "btc_idr".into(),
            order_type: "buy".into(),
        };
        let _cmd4 = TradeCommand::CancelByClientId {
            client_order_id: "client_123".into(),
        };
        let _cmd5 = TradeCommand::CancelAll {
            pair: Some("btc_idr".into()),
        };
        let _cmd6 = TradeCommand::CountdownCancelAll {
            pair: Some("btc_idr".into()),
            countdown_time: 60000,
        };
        let _cmd7 = TradeCommand::GetOrderByClientId {
            client_order_id: "client_123".into(),
        };
    }

    #[test]
    fn test_trade_command_buy_market_order() {
        let cmd = TradeCommand::Buy {
            pair: "btc_idr".into(),
            idr: 100_000.0,
            price: None,
            order_type: None,
            stop_price: None,
            client_id: None,
        };
        match cmd {
            TradeCommand::Buy {
                pair,
                idr,
                price,
                order_type,
                stop_price,
                client_id,
            } => {
                assert_eq!(pair, "btc_idr");
                assert_eq!(idr, 100_000.0);
                assert!(price.is_none());
                assert!(order_type.is_none());
                assert!(stop_price.is_none());
                assert!(client_id.is_none());
            }
            _ => panic!("Expected Buy command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_command_sell_market_order() {
        let cmd = TradeCommand::Sell {
            pair: "eth_idr".into(),
            price: None,
            amount: 1.0,
            order_type: None,
            stop_price: None,
            client_id: None,
        };
        match cmd {
            TradeCommand::Sell { price, .. } => {
                assert!(price.is_none());
            }
            _ => panic!("Expected Sell command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_command_sell_limit_order() {
        let cmd = TradeCommand::Sell {
            pair: "btc_idr".into(),
            price: Some(100_000_000.0),
            amount: 0.5,
            order_type: None,
            stop_price: None,
            client_id: None,
        };
        match cmd {
            TradeCommand::Sell { price, .. } => {
                assert_eq!(price, Some(100_000_000.0));
            }
            _ => panic!("Expected Sell command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_command_cancel_all_no_pair() {
        let cmd = TradeCommand::CountdownCancelAll {
            pair: None,
            countdown_time: 0,
        };
        match cmd {
            TradeCommand::CountdownCancelAll {
                pair,
                countdown_time,
            } => {
                assert!(pair.is_none());
                assert_eq!(countdown_time, 0);
            }
            _ => panic!("Expected CountdownCancelAll command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_cancel_all_parse() {
        let cmd = TradeCommand::CancelAll {
            pair: Some("btc_idr".into()),
        };
        match cmd {
            TradeCommand::CancelAll { pair } => {
                assert_eq!(pair, Some("btc_idr".into()));
            }
            _ => panic!("Expected CancelAll command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_cancel_all_no_pair_filter() {
        let cmd = TradeCommand::CancelAll { pair: None };
        match cmd {
            TradeCommand::CancelAll { pair } => {
                assert!(pair.is_none());
            }
            _ => panic!("Expected CancelAll command, got {:?}", cmd),
        }
    }
}
