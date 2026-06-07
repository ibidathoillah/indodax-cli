#[cfg(feature = "server")]
use crate::auth::Signer;
#[cfg(feature = "server")]
use axum::{
    extract::{Form, Query, State},
    http::{
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
#[cfg(feature = "server")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
#[cfg(feature = "server")]
use rand::RngCore;
#[cfg(feature = "server")]
use serde::Deserialize;
#[cfg(feature = "server")]
use serde_json::{json, Value};
#[cfg(feature = "server")]
use sha2::{Digest, Sha256};
#[cfg(feature = "server")]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "server")]
const DEFAULT_SCOPES: &str = "indodax:market indodax:account indodax:trade indodax:funding";
#[cfg(feature = "server")]
const AUTH_CODE_TTL_SECONDS: u64 = 10 * 60;
#[cfg(feature = "server")]
const ACCESS_TOKEN_TTL_SECONDS: u64 = 12 * 60 * 60;

#[cfg(feature = "server")]
pub type HttpError = (StatusCode, HeaderMap, Json<Value>);

#[cfg(feature = "server")]
#[derive(Clone, Default)]
pub struct OAuthState {
    codes: Arc<RwLock<HashMap<String, AuthorizationCode>>>,
    tokens: Arc<RwLock<HashMap<String, AccessToken>>>,
}

#[cfg(feature = "server")]
#[derive(Clone)]
struct AuthorizationCode {
    api_key: String,
    api_secret: String,
    scopes: Vec<String>,
    redirect_uri: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    resource: String,
    expires_at: u64,
}

#[cfg(feature = "server")]
#[derive(Clone)]
struct AccessToken {
    api_key: String,
    api_secret: String,
    scopes: Vec<String>,
    resource: String,
    expires_at: u64,
}

#[cfg(feature = "server")]
#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    scope: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    resource: Option<String>,
}

#[cfg(feature = "server")]
#[derive(Debug, Deserialize)]
pub struct AuthorizeSubmit {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: String,
    scope: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    resource: Option<String>,
    api_key: String,
    api_secret: String,
}

#[cfg(feature = "server")]
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    grant_type: String,
    code: String,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    client_id: Option<String>,
    resource: Option<String>,
}

#[cfg(feature = "server")]
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    redirect_uris: Option<Vec<String>>,
    token_endpoint_auth_method: Option<String>,
    grant_types: Option<Vec<String>>,
    response_types: Option<Vec<String>>,
    scope: Option<String>,
    client_name: Option<String>,
}

#[cfg(feature = "server")]
pub async fn protected_resource_metadata(State(state): State<super::AppState>) -> Json<Value> {
    Json(json!({
        "resource": state.public_base_url,
        "authorization_servers": [state.public_base_url],
        "scopes_supported": scopes_supported(),
        "resource_documentation": format!("{}/health", state.public_base_url),
        "token_endpoint_auth_methods_supported": ["none"],
    }))
}

#[cfg(feature = "server")]
pub async fn authorization_server_metadata(State(state): State<super::AppState>) -> Json<Value> {
    Json(json!({
        "issuer": state.public_base_url,
        "authorization_endpoint": format!("{}/oauth/authorize", state.public_base_url),
        "token_endpoint": format!("{}/oauth/token", state.public_base_url),
        "registration_endpoint": format!("{}/oauth/register", state.public_base_url),
        "token_endpoint_auth_methods_supported": ["none"],
        "grant_types_supported": ["authorization_code"],
        "response_types_supported": ["code"],
        "code_challenge_methods_supported": ["S256"],
        "scopes_supported": scopes_supported(),
    }))
}

#[cfg(feature = "server")]
pub async fn register_client(Json(payload): Json<ClientRegistrationRequest>) -> Json<Value> {
    let token_auth_method = payload
        .token_endpoint_auth_method
        .unwrap_or_else(|| "none".to_string());

    Json(json!({
        "client_id": format!("indodax-mcp-{}", random_token(12)),
        "client_id_issued_at": now_secs(),
        "redirect_uris": payload.redirect_uris.unwrap_or_default(),
        "token_endpoint_auth_method": token_auth_method,
        "grant_types": payload.grant_types.unwrap_or_else(|| vec!["authorization_code".to_string()]),
        "response_types": payload.response_types.unwrap_or_else(|| vec!["code".to_string()]),
        "scope": payload.scope.unwrap_or_else(|| DEFAULT_SCOPES.to_string()),
        "client_name": payload.client_name.unwrap_or_else(|| "ChatGPT".to_string()),
    }))
}

#[cfg(feature = "server")]
pub async fn authorize_page(Query(query): Query<AuthorizeQuery>) -> Response {
    if query.response_type.as_deref().unwrap_or("code") != "code" {
        return html_error(StatusCode::BAD_REQUEST, "Only response_type=code is supported.");
    }

    let Some(redirect_uri) = query.redirect_uri.clone() else {
        return html_error(StatusCode::BAD_REQUEST, "Missing redirect_uri.");
    };

    if !is_allowed_redirect_uri(&redirect_uri) {
        return html_error(
            StatusCode::BAD_REQUEST,
            "redirect_uri is not allowed. Add ChatGPT connector callback URL or set MCP_OAUTH_ALLOW_INSECURE_REDIRECTS=true for local testing only.",
        );
    }

    let scope = normalize_scope(query.scope.as_deref());
    let resource = query.resource.unwrap_or_default();

    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>Authorize Indodax MCP</title>
  <style>
    body {{ font-family: system-ui, -apple-system, Segoe UI, sans-serif; background:#0b1020; color:#f8fafc; margin:0; padding:32px; }}
    main {{ max-width:520px; margin:auto; background:#111827; border:1px solid #334155; border-radius:16px; padding:24px; }}
    label {{ display:block; margin:16px 0 6px; font-weight:600; }}
    input {{ width:100%; box-sizing:border-box; border-radius:10px; border:1px solid #475569; background:#020617; color:#f8fafc; padding:12px; }}
    button {{ margin-top:20px; width:100%; border:0; border-radius:10px; padding:12px; font-weight:700; cursor:pointer; }}
    .muted {{ color:#94a3b8; font-size:14px; line-height:1.5; }}
    code {{ color:#93c5fd; }}
  </style>
</head>
<body>
<main>
  <h1>Authorize Indodax MCP</h1>
  <p class="muted">ChatGPT is requesting access to this MCP bridge. Indodax does not provide native OAuth, so this bridge wraps your Indodax API key/secret into a short-lived OAuth bearer token.</p>
  <p class="muted">Requested scopes: <code>{scope}</code></p>
  <form method="post" action="/oauth/authorize">
    <input type="hidden" name="response_type" value="{response_type}" />
    <input type="hidden" name="client_id" value="{client_id}" />
    <input type="hidden" name="redirect_uri" value="{redirect_uri}" />
    <input type="hidden" name="scope" value="{scope}" />
    <input type="hidden" name="state" value="{state}" />
    <input type="hidden" name="code_challenge" value="{code_challenge}" />
    <input type="hidden" name="code_challenge_method" value="{code_challenge_method}" />
    <input type="hidden" name="resource" value="{resource}" />

    <label for="api_key">Indodax API Key</label>
    <input id="api_key" name="api_key" autocomplete="username" required />

    <label for="api_secret">Indodax API Secret</label>
    <input id="api_secret" name="api_secret" type="password" autocomplete="current-password" required />

    <button type="submit">Authorize ChatGPT</button>
  </form>
</main>
</body>
</html>"#,
        response_type = html_escape(query.response_type.as_deref().unwrap_or("code")),
        client_id = html_escape(query.client_id.as_deref().unwrap_or("")),
        redirect_uri = html_escape(&redirect_uri),
        scope = html_escape(&scope),
        state = html_escape(query.state.as_deref().unwrap_or("")),
        code_challenge = html_escape(query.code_challenge.as_deref().unwrap_or("")),
        code_challenge_method = html_escape(query.code_challenge_method.as_deref().unwrap_or("S256")),
        resource = html_escape(&resource),
    ))
    .into_response()
}

#[cfg(feature = "server")]
pub async fn authorize_submit(
    State(state): State<super::AppState>,
    Form(form): Form<AuthorizeSubmit>,
) -> Response {
    if form.response_type.as_deref().unwrap_or("code") != "code" {
        return html_error(StatusCode::BAD_REQUEST, "Only response_type=code is supported.");
    }

    if !is_allowed_redirect_uri(&form.redirect_uri) {
        return html_error(StatusCode::BAD_REQUEST, "redirect_uri is not allowed.");
    }

    if form.api_key.trim().is_empty() || form.api_secret.trim().is_empty() {
        return html_error(StatusCode::BAD_REQUEST, "API key and secret are required.");
    }

    let code = random_token(32);
    let scope = normalize_scope(form.scope.as_deref());
    let resource = form
        .resource
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.public_base_url.clone());

    let authorization_code = AuthorizationCode {
        api_key: form.api_key.trim().to_string(),
        api_secret: form.api_secret.trim().to_string(),
        scopes: split_scopes(&scope),
        redirect_uri: form.redirect_uri.clone(),
        code_challenge: form.code_challenge.filter(|value| !value.trim().is_empty()),
        code_challenge_method: form
            .code_challenge_method
            .filter(|value| !value.trim().is_empty())
            .or_else(|| Some("S256".to_string())),
        resource,
        expires_at: now_secs() + AUTH_CODE_TTL_SECONDS,
    };

    state
        .oauth
        .codes
        .write()
        .expect("oauth code store poisoned")
        .insert(code.clone(), authorization_code);

    let mut redirect_url = match url::Url::parse(&form.redirect_uri) {
        Ok(url) => url,
        Err(_) => return html_error(StatusCode::BAD_REQUEST, "Invalid redirect_uri."),
    };

    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("code", &code);
        if let Some(state_value) = form.state.as_deref() {
            if !state_value.is_empty() {
                pairs.append_pair("state", state_value);
            }
        }
    }

    Redirect::to(redirect_url.as_str()).into_response()
}

#[cfg(feature = "server")]
pub async fn token(
    State(state): State<super::AppState>,
    Form(form): Form<TokenRequest>,
) -> Response {
    if form.grant_type != "authorization_code" {
        return oauth_json_error(StatusCode::BAD_REQUEST, "unsupported_grant_type");
    }

    let authorization_code = {
        let mut codes = state.oauth.codes.write().expect("oauth code store poisoned");
        codes.remove(&form.code)
    };

    let Some(authorization_code) = authorization_code else {
        return oauth_json_error(StatusCode::BAD_REQUEST, "invalid_grant");
    };

    if authorization_code.expires_at < now_secs() {
        return oauth_json_error(StatusCode::BAD_REQUEST, "expired_code");
    }

    if let Some(redirect_uri) = form.redirect_uri.as_deref() {
        if redirect_uri != authorization_code.redirect_uri {
            return oauth_json_error(StatusCode::BAD_REQUEST, "redirect_uri_mismatch");
        }
    }

    if let Some(expected_challenge) = authorization_code.code_challenge.as_deref() {
        if authorization_code.code_challenge_method.as_deref() != Some("S256") {
            return oauth_json_error(StatusCode::BAD_REQUEST, "unsupported_code_challenge_method");
        }

        let Some(code_verifier) = form.code_verifier.as_deref() else {
            return oauth_json_error(StatusCode::BAD_REQUEST, "missing_code_verifier");
        };

        if pkce_s256(code_verifier) != expected_challenge {
            return oauth_json_error(StatusCode::BAD_REQUEST, "invalid_code_verifier");
        }
    }

    if let Some(resource) = form.resource.as_deref() {
        if resource != authorization_code.resource {
            return oauth_json_error(StatusCode::BAD_REQUEST, "resource_mismatch");
        }
    }

    let token = random_token(40);
    let expires_at = now_secs() + ACCESS_TOKEN_TTL_SECONDS;

    state
        .oauth
        .tokens
        .write()
        .expect("oauth token store poisoned")
        .insert(
            token.clone(),
            AccessToken {
                api_key: authorization_code.api_key,
                api_secret: authorization_code.api_secret,
                scopes: authorization_code.scopes.clone(),
                resource: authorization_code.resource.clone(),
                expires_at,
            },
        );

    Json(json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TOKEN_TTL_SECONDS,
        "scope": authorization_code.scopes.join(" "),
        "resource": authorization_code.resource,
    }))
    .into_response()
}

#[cfg(feature = "server")]
pub fn required_scope_for_tool(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "balance"
        | "account_info"
        | "open_orders"
        | "order_history"
        | "trade_history"
        | "get_order"
        | "get_order_by_client_id"
        | "trans_history"
        | "equity_snap"
        | "equity_history"
        | "list_downline"
        | "check_downline" => Some("indodax:account"),
        "buy_order" | "sell_order" | "cancel_order" | "cancel_all_orders" => {
            Some("indodax:trade")
        }
        "withdraw" | "deposit_address" => Some("indodax:funding"),
        _ => None,
    }
}

#[cfg(feature = "server")]
pub fn resolve_signer(
    state: &super::AppState,
    headers: &HeaderMap,
    required_scope: Option<&'static str>,
) -> Result<Option<Signer>, HttpError> {
    let api_key = headers.get("x-api-key").and_then(|h| h.to_str().ok());
    let api_secret = headers.get("x-api-secret").and_then(|h| h.to_str().ok());

    if let (Some(k), Some(s)) = (api_key, api_secret) {
        return Ok(Some(Signer::new(k, s)));
    }

    let bearer = bearer_token(headers);

    if let Some(token) = bearer {
        let stored_token = {
            let tokens = state.oauth.tokens.read().expect("oauth token store poisoned");
            tokens.get(token).cloned()
        };

        let Some(stored_token) = stored_token else {
            if let Some(scope) = required_scope {
                return Err(oauth_challenge(state, scope, "invalid_token"));
            }
            return Ok(None);
        };

        if stored_token.expires_at < now_secs() {
            if let Some(scope) = required_scope {
                return Err(oauth_challenge(state, scope, "expired_token"));
            }
            return Ok(None);
        }

        if stored_token.resource != state.public_base_url {
            if let Some(scope) = required_scope {
                return Err(oauth_challenge(state, scope, "invalid_resource"));
            }
            return Ok(None);
        }

        if let Some(scope) = required_scope {
            if !has_scope(&stored_token.scopes, scope) {
                return Err(oauth_challenge(state, scope, "insufficient_scope"));
            }
        }

        return Ok(Some(Signer::new(
            stored_token.api_key.as_str(),
            stored_token.api_secret.as_str(),
        )));
    }

    if let Some(scope) = required_scope {
        return Err(oauth_challenge(state, scope, "missing_token"));
    }

    Ok(None)
}

#[cfg(feature = "server")]
pub fn json_error(status: StatusCode, message: &str) -> HttpError {
    (
        status,
        HeaderMap::new(),
        Json(json!({
            "error": true,
            "message": message,
        })),
    )
}

#[cfg(feature = "server")]
fn oauth_challenge(state: &super::AppState, scope: &str, error: &str) -> HttpError {
    let mut headers = HeaderMap::new();
    let challenge = format!(
        "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\", scope=\"{}\", error=\"{}\"",
        state.public_base_url, scope, error
    );

    if let Ok(value) = HeaderValue::from_str(&challenge) {
        headers.insert(WWW_AUTHENTICATE, value);
    }

    (
        StatusCode::UNAUTHORIZED,
        headers,
        Json(json!({
            "error": true,
            "message": "OAuth authorization required.",
            "required_scope": scope,
        })),
    )
}

#[cfg(feature = "server")]
fn oauth_json_error(status: StatusCode, error: &str) -> Response {
    (
        status,
        Json(json!({
            "error": error,
        })),
    )
        .into_response()
}

#[cfg(feature = "server")]
fn html_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Html(format!(
            "<!doctype html><title>OAuth Error</title><h1>OAuth Error</h1><p>{}</p>",
            html_escape(message)
        )),
    )
        .into_response()
}

#[cfg(feature = "server")]
fn scopes_supported() -> Vec<&'static str> {
    vec![
        "indodax:market",
        "indodax:account",
        "indodax:trade",
        "indodax:funding",
    ]
}

#[cfg(feature = "server")]
fn normalize_scope(scope: Option<&str>) -> String {
    let value = scope
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SCOPES);

    split_scopes(value).join(" ")
}

#[cfg(feature = "server")]
fn split_scopes(scope: &str) -> Vec<String> {
    scope
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(feature = "server")]
fn has_scope(scopes: &[String], required: &str) -> bool {
    scopes.iter().any(|scope| {
        scope == required
            || scope == "indodax:*"
            || (required == "indodax:account" && scope == "indodax:trade")
    })
}

#[cfg(feature = "server")]
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ")
}

#[cfg(feature = "server")]
fn is_allowed_redirect_uri(redirect_uri: &str) -> bool {
    if std::env::var("MCP_OAUTH_ALLOW_INSECURE_REDIRECTS").as_deref() == Ok("true") {
        return true;
    }

    redirect_uri.starts_with("https://chatgpt.com/connector/oauth/")
        || redirect_uri == "https://chatgpt.com/connector_platform_oauth_redirect"
}

#[cfg(feature = "server")]
fn pkce_s256(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(feature = "server")]
fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

#[cfg(feature = "server")]
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(feature = "server")]
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
