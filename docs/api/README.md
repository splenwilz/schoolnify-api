# Schoolnify API Reference

Base URL: `http://localhost:8080` (development) | `https://api.schoolnify.com` (production)

Interactive Swagger docs: `http://localhost:8080/docs`

---

## Endpoint Groups

| File | Prefix | Description |
|------|--------|-------------|
| [auth.md](auth.md) | `/api/v1/auth/*` | Signup, login, logout, session management, OAuth |
| [schools.md](schools.md) | `/api/v1/schools/*` | School setup wizard, public branding |
| [students.md](students.md) | `/api/v1/students/*` | Student CRUD, status/class changes, promotion, CSV import/export |
| [health.md](health.md) | `/health` | Health check |
| [types.md](types.md) | — | Shared response types (UserResponse, etc.) |

---

## Authentication

Protected endpoints require one of:

1. **Bearer token** (recommended for frontend with proxy):
   ```text
   Authorization: Bearer <access_token>
   ```

2. **Session cookie** (set automatically by auth endpoints):
   ```text
   Cookie: session_token=<jwt>
   ```

The `access_token` is a short-lived JWT (15 minutes by default). Use the `/refresh` endpoint or the `refresh_token` from login responses to obtain new access tokens.

---

## Error Format

All errors follow a consistent structure:

```json
{
  "error": {
    "type": "ERROR_TYPE",
    "message": "Human-readable error description"
  }
}
```

| HTTP Status | Error Type | Description |
|-------------|-----------|-------------|
| 400 | `BAD_REQUEST` | Invalid request body or parameters |
| 401 | `UNAUTHORIZED` | Missing, invalid, or expired token |
| 403 | `FORBIDDEN` | Authenticated but not permitted |
| 404 | `NOT_FOUND` | Resource does not exist |
| 409 | `CONFLICT` | Resource already exists (e.g. duplicate email) |
| 500 | `INTERNAL_ERROR` | Server error (details redacted) |
| 502 | `EXTERNAL_SERVICE_ERROR` | Upstream service failure (WorkOS) |
| 503 | — | Service unhealthy (health check) |
