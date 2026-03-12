# CLAUDE.md - Schoolnify API

## Project Overview
Schoolnify API is a production-grade Rust REST API for a school management platform.
Built with Axum 0.8, PostgreSQL via SQLx, and WorkOS User Management for authentication.

## Build & Run

```bash
# Install sqlx-cli (for migrations)
cargo install sqlx-cli --no-default-features --features postgres

# Copy environment template
cp .env.example .env
# Edit .env with your actual values

# sqlx-cli requires DATABASE_URL (not APP__DATABASE__URL)
export DATABASE_URL=postgresql://postgres:password@localhost:5432/schoolnify

# Create database
sqlx database create

# Run migrations
sqlx migrate run

# Run in development
cargo run

# Run with release optimizations
cargo run --release

# Run tests
cargo test
```

## Project Structure
- `src/main.rs` - Entry point (tracing, config, DB pool, server startup)
- `src/lib.rs` - Router assembly with middleware layers
- `src/config.rs` - Configuration loading (default.toml + env vars with APP__ prefix)
- `src/state.rs` - AppState shared across all handlers
- `src/errors.rs` - Centralized AppError enum with IntoResponse
- `src/db.rs` - Database pool creation
- `src/routes/` - Route definitions (thin, just map paths to handlers)
- `src/handlers/` - Request handler functions (extract, delegate, respond)
- `src/services/` - Business logic and external API clients
- `src/models/` - Data types (DB models, API types, WorkOS types)
- `src/middleware/` - Axum middleware (JWT auth)
- `migrations/` - SQLx database migrations
- `config/` - TOML configuration files

## Architecture Rules
- NO file should exceed 700 lines
- Handlers extract data, call services, return responses - no business logic
- Services contain business logic and DB queries
- Models are data-only structs (no methods with side effects)
- All errors flow through AppError for consistent API responses
- WorkOS integration uses reqwest HTTP calls, not the workos crate

## Environment Variables
All config can be overridden via env vars prefixed with `APP__`:
- `APP__DATABASE__URL` - PostgreSQL connection string (required)
- `APP__WORKOS__API_KEY` - WorkOS API key (required)
- `APP__WORKOS__CLIENT_ID` - WorkOS client ID (required)
- `APP__WORKOS__CLIENT_SECRET` - WorkOS client secret (required)
- `APP__SERVER__PORT` - Server port (default: 8080)
- `APP__CORS__ALLOWED_ORIGINS` - Comma-separated allowed origins
- `RUN_ENV` - Environment name: development, staging, production

## Auth Flow (Custom UI + API Proxy)
1. Frontend sends POST /api/v1/auth/signup with { email, password, first_name, last_name }
2. Backend creates user in WorkOS, authenticates, upserts in local DB
3. Backend sets HttpOnly secure cookies (session_token + refresh_token)
4. Subsequent requests include JWT in cookie, validated by middleware using WorkOS JWKS
5. Token refresh via POST /api/v1/auth/refresh (reads refresh token cookie)

## Database
- PostgreSQL with SQLx (compile-time checked queries when using query_as!)
- Migrations in `migrations/` directory
- Run `sqlx migrate run` to apply, `sqlx migrate revert` to rollback

## Key Dependencies
- axum 0.8 - Web framework (uses {param} path syntax)
- sqlx 0.8 - Async PostgreSQL driver
- jsonwebtoken 10 - JWT validation via JWKS
- reqwest 0.12 - HTTP client for WorkOS API
- tower-http 0.6 - HTTP middleware (CORS, tracing, compression)
- thiserror 2 - Error derive macros
