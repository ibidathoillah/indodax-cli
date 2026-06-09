# ChatGPT OAuth + Tunnel Setup

This guide exposes the Indodax MCP HTTP bridge to ChatGPT through a public HTTPS tunnel and uses OAuth Authorization Code + PKCE so ChatGPT receives a short-lived bearer token instead of raw Indodax API credentials.

> Safety default: the compose file enables only `market,account,paper,auth` groups. Live trading and funding are intentionally disabled until you opt in.

## Architecture

```text
ChatGPT OAuth connector
  -> https://your-public-domain
  -> Cloudflare Tunnel
  -> indodax-cli mcp --http --port 8000
  -> Indodax API
```

The bridge already exposes the required OAuth endpoints:

- `GET /.well-known/oauth-protected-resource`
- `GET /.well-known/oauth-authorization-server`
- `GET /oauth/authorize`
- `POST /oauth/token`
- `POST /oauth/register`
- `POST /call/{tool_name}`

## 1. Create a Cloudflare Tunnel

Create a tunnel in Cloudflare Zero Trust and route a public hostname, for example:

```text
indodax-mcp.example.com -> http://indodax-mcp:8000
```

Copy the tunnel token into your environment.

## 2. Configure environment

```bash
export MCP_PUBLIC_BASE_URL="https://indodax-mcp.example.com"
export CLOUDFLARE_TUNNEL_TOKEN="YOUR_CLOUDFLARE_TUNNEL_TOKEN"

# Optional local port for debugging only.
export MCP_PORT=8000

# Safe default. Add trade/funding only after you intentionally want live execution exposed.
export MCP_GROUPS="market,account,paper,auth"
```

Do not set `BRIDGE_SECRET` for ChatGPT OAuth. ChatGPT can authenticate with Bearer tokens, but it usually cannot add your custom `X-Bridge-Auth` header during OAuth discovery and token exchange.

## 3. Run the bridge + tunnel

```bash
docker compose -f docker-compose.chatgpt-tunnel.yml up -d --build
```

Health check:

```bash
curl "$MCP_PUBLIC_BASE_URL/health"
```

OAuth discovery check:

```bash
curl "$MCP_PUBLIC_BASE_URL/.well-known/oauth-protected-resource" | jq
curl "$MCP_PUBLIC_BASE_URL/.well-known/oauth-authorization-server" | jq
```

## 4. Register in ChatGPT

Use these URLs when configuring a ChatGPT custom connector/action that supports OAuth:

```text
Authorization URL: https://indodax-mcp.example.com/oauth/authorize
Token URL:         https://indodax-mcp.example.com/oauth/token
Scopes:            indodax:market indodax:account indodax:paper
```

Use the OpenAPI file in this repository as the Actions schema:

```text
openapi/chatgpt-actions.json
```

During authorization, the bridge displays a login form asking for your Indodax API key and secret. The server stores them only in memory behind an authorization code / bearer token pair. Restarting the container clears issued codes and tokens.

## 5. Test a tool call with OAuth bearer

After ChatGPT completes OAuth, it will call tools with:

```http
Authorization: Bearer <access_token>
```

Manual market-data check, which does not require a token:

```bash
curl -X POST "$MCP_PUBLIC_BASE_URL/call/ticker" \
  -H 'content-type: application/json' \
  -d '{"pair":"btc_idr"}'
```

Manual private check requires either OAuth Bearer or legacy headers:

```bash
curl -X POST "$MCP_PUBLIC_BASE_URL/call/balance" \
  -H "authorization: Bearer $ACCESS_TOKEN" \
  -H 'content-type: application/json' \
  -d '{}'
```

## 6. Enabling live trading intentionally

Only after you are ready to expose live trading/funding operations to an AI connector:

```bash
export MCP_GROUPS="market,account,paper,auth,trade,funding"
```

Then edit the compose command to include:

```yaml
- --allow-dangerous
```

Without `--allow-dangerous`, dangerous MCP tools still require explicit acknowledgement where supported.

## Important security notes

- Use a subdomain dedicated to this bridge.
- Prefer read-only Indodax API permissions for normal ChatGPT use.
- Keep live trading/funding out of `MCP_GROUPS` unless you have a separate risk-control layer.
- Restart the container to revoke all in-memory OAuth tokens.
- Do not publish the tunnel token, Indodax API key, or Indodax API secret.
