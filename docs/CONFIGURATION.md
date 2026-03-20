# Configuration Guide

Schoolnify API uses a layered configuration system: TOML defaults → environment-specific TOML → environment variables.

---

## Configuration Loading Order

1. `config/default.toml` — base defaults (checked into git)
2. `config/{RUN_ENV}.toml` — environment overrides (e.g. `config/production.toml`)
3. Environment variables with `APP__` prefix — highest priority

The `RUN_ENV` environment variable controls which overlay file is loaded. Defaults to `development`.

---

## Environment Variables

All configuration can be overridden via environment variables prefixed with `APP__`. Double underscores (`__`) denote nesting.

### Required

| Variable | Example | Description |
|----------|---------|-------------|
| `APP__DATABASE__URL` | `postgresql://user:pass@host:5432/schoolnify` | PostgreSQL connection string |
| `APP__WORKOS__API_KEY` | `sk_test_...` or `sk_live_...` | WorkOS API key |
| `APP__WORKOS__CLIENT_ID` | `client_01K...` | WorkOS client ID |
| `APP__WORKOS__CLIENT_SECRET` | `sk_test_...` | WorkOS client secret |

### Server

| Variable | Default | Description |
|----------|---------|-------------|
| `APP__SERVER__HOST` | `0.0.0.0` | Bind address |
| `APP__SERVER__PORT` | `8080` | Listen port |
| `APP__SERVER__REQUEST_TIMEOUT_SECS` | `30` | Request timeout |

### Database

| Variable | Default | Description |
|----------|---------|-------------|
| `APP__DATABASE__URL` | *(required)* | PostgreSQL connection string |
| `APP__DATABASE__MAX_CONNECTIONS` | `20` | Connection pool max |
| `APP__DATABASE__MIN_CONNECTIONS` | `5` | Connection pool min |
| `APP__DATABASE__ACQUIRE_TIMEOUT_SECS` | `5` | Connection acquire timeout |

### Authentication

| Variable | Default | Description |
|----------|---------|-------------|
| `APP__AUTH__SESSION_COOKIE_NAME` | `session_token` | Name of the session cookie |
| `APP__AUTH__REFRESH_COOKIE_NAME` | `refresh_token` | Name of the refresh cookie |
| `APP__AUTH__ACCESS_TOKEN_EXPIRY_SECS` | `900` | JWT lifetime (15 min) |
| `APP__AUTH__REFRESH_TOKEN_EXPIRY_DAYS` | `30` | Refresh token lifetime |
| `APP__AUTH__COOKIE_SECURE` | `true` | Set `false` in dev `.env` for HTTP |
| `APP__AUTH__COOKIE_HTTP_ONLY` | `true` | Always `true` — prevents JS access |
| `APP__AUTH__COOKIE_SAME_SITE` | `lax` | `lax`, `strict`, or `none` |
| `APP__AUTH__COOKIE_DOMAIN` | `""` | Empty = host-only cookie. Set `.schoolnify.com` in prod |
| `APP__AUTH__POST_LOGIN_REDIRECT_URL` | `""` | OAuth callback redirect (set in .env for dev) |
| `APP__AUTH__EXPOSE_TOKEN_IN_RESPONSE` | `false` | Set `true` in dev `.env` to include tokens in JSON |

### CORS

| Variable | Default | Description |
|----------|---------|-------------|
| `APP__CORS__ALLOWED_ORIGINS` | `http://localhost:3000,...` | Comma-separated list of allowed origins |
| `APP__CORS__BASE_DOMAIN` | `localhost` | For dynamic subdomain CORS matching |

### WorkOS

| Variable | Default | Description |
|----------|---------|-------------|
| `APP__WORKOS__API_KEY` | *(required)* | WorkOS API key |
| `APP__WORKOS__CLIENT_ID` | *(required)* | WorkOS client ID |
| `APP__WORKOS__CLIENT_SECRET` | *(required)* | WorkOS client secret |
| `APP__WORKOS__REDIRECT_URI` | *(required)* | OAuth callback URL |
| `APP__WORKOS__API_BASE_URL` | `https://api.workos.com` | WorkOS API base URL |

---

## Environment Profiles

### Development

```bash
# .env
RUN_ENV=development

APP__DATABASE__URL=postgresql://postgres:password@localhost:5432/schoolnify
APP__WORKOS__API_KEY=sk_test_...
APP__WORKOS__CLIENT_ID=client_...
APP__WORKOS__CLIENT_SECRET=sk_test_...
APP__WORKOS__REDIRECT_URI=http://localhost:8080/api/v1/auth/callback

APP__SERVER__PORT=8080

APP__CORS__ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001,http://localhost:5173
APP__CORS__BASE_DOMAIN=localhost

APP__AUTH__POST_LOGIN_REDIRECT_URL=http://localhost:3000
APP__AUTH__EXPOSE_TOKEN_IN_RESPONSE=true
```

### Production

```bash
RUN_ENV=production

APP__DATABASE__URL=postgresql://user:pass@db-host:5432/schoolnify

APP__WORKOS__API_KEY=sk_live_...
APP__WORKOS__CLIENT_ID=client_...
APP__WORKOS__CLIENT_SECRET=sk_live_...
APP__WORKOS__REDIRECT_URI=https://api.schoolnify.com/api/v1/auth/callback

APP__CORS__ALLOWED_ORIGINS=https://schoolnify.com
APP__CORS__BASE_DOMAIN=schoolnify.com

APP__AUTH__COOKIE_SECURE=true
APP__AUTH__COOKIE_DOMAIN=.schoolnify.com
APP__AUTH__POST_LOGIN_REDIRECT_URL=https://schoolnify.com
APP__AUTH__EXPOSE_TOKEN_IN_RESPONSE=false
```

---

## Default Configuration (config/default.toml)

```toml
[server]
host = "0.0.0.0"
port = 8080
request_timeout_secs = 30

[database]
url = ""
max_connections = 20
min_connections = 5
acquire_timeout_secs = 5

[workos]
api_key = ""
client_id = ""
client_secret = ""
redirect_uri = ""
api_base_url = "https://api.workos.com"

[auth]
session_cookie_name = "session_token"
refresh_cookie_name = "refresh_token"
access_token_expiry_secs = 900
refresh_token_expiry_days = 30
cookie_secure = true
cookie_http_only = true
cookie_same_site = "lax"
cookie_domain = ""
post_login_redirect_url = ""
# Set to true in development if you need tokens in JSON responses
expose_token_in_response = false

[cors]
allowed_origins = ["http://localhost:3000", "http://localhost:3001", "http://localhost:5173"]
base_domain = "localhost"
```

---

## CORS Behavior

The API supports both static and dynamic CORS origins:

1. **Static origins:** Listed in `allowed_origins`. Exact match.
2. **Dynamic subdomains:** Any `{slug}.{base_domain}` is allowed (single-level subdomain only).

Examples with `base_domain = "schoolnify.com"`:
- `https://springfield.schoolnify.com` — allowed
- `https://schoolnify.com` — allowed (bare base domain)
- `https://evil.springfield.schoolnify.com` — rejected (multi-level)
- `https://evil.com` — rejected

Invalid entries in `allowed_origins` are logged as warnings and skipped.

---

## Security Notes

| Setting | Dev | Prod | Why |
|---------|-----|------|-----|
| `cookie_secure` | `false` | `true` | HTTPS only in production |
| `cookie_domain` | `""` | `.schoolnify.com` | Subdomain cookie sharing in prod |
| `expose_token_in_response` | `true` | `false` | Tokens only in cookies in prod |
| `cookie_same_site` | `lax` | `lax` | Mitigates CSRF risk, allows same-site nav |

**Sensitive fields are redacted in debug logs:** `api_key`, `client_secret`, and `database.url` are printed as `[REDACTED]` when the config struct is debug-formatted.
