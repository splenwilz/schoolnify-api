# Architecture

Schoolnify API is a production-grade Rust REST API built with Axum, PostgreSQL, and WorkOS for authentication.

---

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Web framework | Axum | 0.8 |
| Database | PostgreSQL + SQLx | 0.8 |
| Authentication | WorkOS User Management | — |
| JWT validation | jsonwebtoken + JWKS | 10 |
| HTTP client | reqwest | 0.12 |
| Middleware | tower-http | 0.6 (CORS, tracing, compression, timeout) |
| Error handling | thiserror | 2 |
| Config | config-rs | — |
| API docs | utoipa + Swagger UI | — |

---

## Project Structure

```
src/
├── main.rs              # Entry point: tracing, config, DB pool, server startup
├── lib.rs               # Router assembly, middleware layers, OpenAPI schema
├── config.rs            # Configuration loading (TOML + env vars)
├── state.rs             # AppState shared across all handlers
├── errors.rs            # Centralized AppError enum → consistent HTTP responses
├── db.rs                # Database pool creation
├── routes/
│   ├── mod.rs           # Route tree assembly
│   ├── auth.rs          # Auth route definitions
│   └── health.rs        # Health check routes
├── handlers/
│   ├── auth.rs          # Auth request handlers
│   └── health.rs        # Health check handler
├── services/
│   ├── workos.rs        # WorkOS API client (auth, orgs, JWKS)
│   ├── user.rs          # User DB operations
│   └── organization.rs  # Organization DB operations
├── models/
│   ├── auth.rs          # Auth DTOs (request/response types, WorkOS types)
│   ├── user.rs          # User DB model + UserResponse DTO
│   ├── organization.rs  # Organization DB model + OrganizationResponse DTO
│   └── health.rs        # Health check response types
└── middleware/
    └── auth.rs          # JWT validation middleware
```

---

## Design Principles

### Layered Architecture

```
HTTP Request
    │
    ▼
┌─────────┐
│ Routes   │  Thin: map paths → handlers
└────┬─────┘
     ▼
┌──────────┐
│ Handlers  │  Extract data, call services, return responses
└────┬──────┘
     ▼
┌──────────┐
│ Services  │  Business logic, DB queries, external API calls
└────┬──────┘
     ▼
┌──────────┐
│ Models    │  Data-only structs (no side effects)
└──────────┘
```

- **Routes** — map HTTP methods + paths to handler functions. No logic.
- **Handlers** — extract request data (path params, JSON body, cookies), delegate to services, format responses. No SQL, no external API calls.
- **Services** — contain all business logic. Talk to the database and external APIs (WorkOS).
- **Models** — plain data structs. DB models use `FromRow`. API responses use `Serialize` + `ToSchema`.

### Error Handling

All errors flow through `AppError` (in `errors.rs`):

```rust
pub enum AppError {
    Unauthorized(String),   // 401
    Forbidden(String),      // 403
    NotFound(String),       // 404
    BadRequest(String),     // 400
    Conflict(String),       // 409
    ExternalService(String), // 502 (message redacted in response)
    Internal(String),       // 500 (message redacted in response)
    Database(sqlx::Error),  // 500 (message redacted in response)
    ...
}
```

Client-facing errors (`Unauthorized`, `Forbidden`, `NotFound`, `BadRequest`, `Conflict`) include the error message in the response. Server-side errors (`ExternalService`, `Internal`, `Database`) always return a generic message to prevent information leakage.

### File Size Rule

No single file should exceed 700 lines. If a file grows beyond this, split it into focused modules.

---

## Authentication Flow

The API uses WorkOS User Management with a custom UI (not AuthKit). The frontend controls the UI; the API proxies to WorkOS.

### Password Authentication

```
Frontend                    API                         WorkOS
   │                         │                            │
   │ POST /auth/login        │                            │
   │ {email, password}       │                            │
   │────────────────────────>│                            │
   │                         │ POST /user_management/     │
   │                         │   authenticate             │
   │                         │ {grant_type: "password"}   │
   │                         │───────────────────────────>│
   │                         │                            │
   │                         │ {access_token,             │
   │                         │  refresh_token, user}      │
   │                         │<───────────────────────────│
   │                         │                            │
   │                         │ Upsert user in local DB    │
   │                         │ Store refresh_token hash   │
   │                         │ Set HttpOnly cookies       │
   │                         │                            │
   │ {user, access_token,    │                            │
   │  refresh_token,         │                            │
   │  subdomain_url}         │                            │
   │<────────────────────────│                            │
```

### JWT Validation (Auth Middleware)

```
Frontend                    API Middleware               WorkOS
   │                         │                            │
   │ GET /auth/me            │                            │
   │ Bearer: <JWT>           │                            │
   │────────────────────────>│                            │
   │                         │ Decode JWT header → kid    │
   │                         │ Lookup kid in JWKS cache   │
   │                         │                            │
   │                         │ (cache miss or expired?)   │
   │                         │ GET /sso/jwks/{client_id}  │
   │                         │───────────────────────────>│
   │                         │ {keys: [...]}              │
   │                         │<───────────────────────────│
   │                         │                            │
   │                         │ Validate JWT signature     │
   │                         │ Check exp, iss             │
   │                         │ Extract: sub, org_id, role │
   │                         │ Inject CurrentUser         │
   │                         │                            │
   │                         │───> Handler executes       │
```

**JWKS caching:**
- Cached for 1 hour
- Thundering herd prevention: only one task fetches at a time; others use stale data
- On fetch failure: returns stale cached keys if available
- Force refresh: if JWT has unknown `kid`, triggers immediate re-fetch

---

## Multi-Tenant Architecture

Each school is an "organization" with its own subdomain:

```
schoolnify.com             → Main marketing site / login
springfield.schoolnify.com → Springfield High School dashboard
greendale.schoolnify.com   → Greendale Community College dashboard
api.schoolnify.com         → Shared API (all schools)
```

**In development:**
```
localhost:3000                       → Main site
springfield.localhost:3000           → Springfield subdomain
api: localhost:8080                  → Shared API
```

### How Organizations Work

1. Admin signs up → WorkOS user created → email verified
2. Admin creates org → WorkOS org + membership created → local DB org created
3. Org gets a slug (e.g. `springfield-high-school`) derived from the school name
4. Slug becomes the subdomain: `springfield-high-school.schoolnify.com`
5. All users in that org access data through that subdomain

### Slug Generation

- `"Springfield High School"` → `springfield-high-school`
- Non-alphanumeric characters become hyphens
- Consecutive hyphens collapsed
- If slug exists, appends `-2`, `-3`, etc.
- Checked and retried atomically (handles race conditions)

---

## Request Pipeline

```
Request
  │
  ▼
CORS Layer (dynamic: static origins + subdomain matching)
  │
  ▼
Trace Layer (request/response logging)
  │
  ▼
Timeout Layer (30s default, returns 408)
  │
  ▼
Compression Layer (gzip/deflate/br)
  │
  ▼
Body Limit Layer (1 MB max)
  │
  ▼
Router → Route → Handler
```

Protected routes add an additional auth middleware layer that validates the JWT and injects `CurrentUser` into the request extensions.
