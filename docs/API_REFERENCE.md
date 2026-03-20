# Schoolnify API Reference

Base URL: `http://localhost:8080` (development) | `https://api.schoolnify.com` (production)

Interactive Swagger docs: `http://localhost:8080/docs`

---

## Table of Contents

- [Authentication](#authentication)
- [Error Format](#error-format)
- [Endpoints](#endpoints)
  - [Health](#health)
  - [Signup](#post-apiv1authsignup)
  - [Admin Signup](#post-apiv1authadmin-signup)
  - [Verify Email](#post-apiv1authverify-email)
  - [Resend Verification](#post-apiv1authresend-verification)
  - [Login](#post-apiv1authlogin)
  - [Get Current User](#get-apiv1authme)
  - [Delete Account](#delete-apiv1authme)
  - [Establish Session](#post-apiv1authestablish-session)
  - [Create Organization](#post-apiv1authcreate-organization)
  - [Refresh Token](#post-apiv1authrefresh)
  - [Logout](#post-apiv1authlogout)
  - [OAuth Authorize](#get-apiv1authauthorize)
  - [OAuth Callback](#get-apiv1authcallback)
- [Response Types](#response-types)

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

---

## Endpoints

### Health

#### `GET /health`

Check API and database health.

**Response `200`:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "checks": {
    "database": "up"
  }
}
```

**Response `503`:** Same structure with `"unhealthy"` / `"down"`.

---

### `POST /api/v1/auth/signup`

Create a regular user account (no school/organization).

**Request:**
```json
{
  "email": "john@example.com",
  "password": "SecurePass123!",
  "first_name": "John",
  "last_name": "Doe"
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `email` | string | yes | Must be unique |
| `password` | string | yes | Min 8 characters |
| `first_name` | string | no | |
| `last_name` | string | no | |

**Response `201` (email verification required):**
```json
{
  "message": "Account created. Please check your email for a verification code.",
  "pending_authentication_token": "JEINf3ozYj2soOa2xi2xzaEIS",
  "user_id": "user_01HXYZ..."
}
```

**Response `201` (verification disabled, auto-authenticated):**
```json
{
  "user": { "id": "...", "email": "...", "role": "user", "..." : "..." },
  "message": "Account created successfully",
  "access_token": "eyJ...",
  "refresh_token": "6sVQ...",
  "subdomain_url": null
}
```

| Error | Status | When |
|-------|--------|------|
| Email already exists | `409` | Duplicate email address |
| WorkOS error | `502` | Upstream service failure |

---

### `POST /api/v1/auth/admin-signup`

Create a school admin account with a school organization. This is the primary signup flow for school administrators.

**Request:**
```json
{
  "email": "admin@springfield-high.edu",
  "password": "SecurePass123!",
  "first_name": "Jane",
  "last_name": "Smith",
  "school_name": "Springfield High School"
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `email` | string | yes | Must be unique |
| `password` | string | yes | Min 8 characters |
| `first_name` | string | no | |
| `last_name` | string | no | |
| `school_name` | string | yes | Used to generate org slug |

**Response `202` (email verification required):**
```json
{
  "message": "Account created. Verify your email, then complete school setup.",
  "pending_authentication_token": "JEINf3ozYj2soOa2xi2xzaEIS",
  "school_name": "Springfield High School",
  "user_id": "user_01HXYZ..."
}
```
Save `pending_authentication_token`, `school_name`, and `user_id` — you need them for the next steps.

**Response `201` (verification disabled, fully created):**
```json
{
  "user": { "id": "...", "email": "...", "role": "user", "..." : "..." },
  "organization": { "id": "...", "name": "...", "slug": "...", "..." : "..." },
  "message": "School admin account created successfully",
  "access_token": "eyJ...",
  "subdomain_url": "http://springfield-high-school.localhost:3000"
}
```

---

### `POST /api/v1/auth/verify-email`

Complete email verification with the 6-digit code sent to the user's email.

**Request:**
```json
{
  "code": "123456",
  "pending_authentication_token": "JEINf3ozYj2soOa2xi2xzaEIS"
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `code` | string | yes | 6-digit code from email |
| `pending_authentication_token` | string | yes | From signup response |

**Response `200`:**
```json
{
  "user": { "id": "...", "email": "...", "role": "user", "..." : "..." },
  "message": "Email verified successfully",
  "access_token": "eyJ...",
  "refresh_token": "6sVQ...",
  "subdomain_url": null
}
```

Save `access_token` and `refresh_token` — pass them to `/create-organization` in the next step.

| Error | Status | When |
|-------|--------|------|
| Invalid code | `400` | Wrong or expired verification code |

---

### `POST /api/v1/auth/resend-verification`

Resend the email verification code.

**Request:**
```json
{
  "user_id": "user_01HXYZ..."
}
```

**Response `200`:**
```json
{
  "message": "Verification email sent"
}
```

---

### `POST /api/v1/auth/login`

Authenticate with email and password.

**Request:**
```json
{
  "email": "john@example.com",
  "password": "SecurePass123!"
}
```

**Response `200` (success):**
```json
{
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "admin@springfield-high.edu",
    "first_name": "Jane",
    "last_name": "Smith",
    "email_verified": true,
    "profile_picture_url": null,
    "organization_id": "660e8400-e29b-41d4-a716-446655440000",
    "role": "admin",
    "created_at": "2026-03-19T10:00:00Z"
  },
  "message": "Login successful",
  "access_token": "eyJ...",
  "refresh_token": "6sVQ...",
  "subdomain_url": "http://springfield-high-school.localhost:3000"
}
```

Key fields:
- `subdomain_url` — present if user belongs to an organization. Use for redirect.
- `subdomain_url` — `null` if user has no organization.
- `access_token` / `refresh_token` — present when `expose_token_in_response` is enabled (dev mode).

**Response `403` (email verification required):**
```json
{
  "message": "Email verification required. Please check your email for a verification code.",
  "pending_authentication_token": "JEINf3ozYj2soOa2xi2xzaEIS"
}
```

| Error | Status | When |
|-------|--------|------|
| Invalid credentials | `401` | Wrong email or password |
| Email not verified | `403` | Must verify email first |

---

### `GET /api/v1/auth/me`

Get the authenticated user's profile.

**Auth:** Required (Bearer token or session cookie)

**Response `200`:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "admin@springfield-high.edu",
  "first_name": "Jane",
  "last_name": "Smith",
  "email_verified": true,
  "profile_picture_url": null,
  "organization_id": "660e8400-e29b-41d4-a716-446655440000",
  "role": "admin",
  "created_at": "2026-03-19T10:00:00Z"
}
```

---

### `DELETE /api/v1/auth/me`

Permanently delete the authenticated user's account. If the user is the sole admin of an organization, the organization is also deleted (both locally and in WorkOS).

**Auth:** Required

**Response `200`:**
```json
{
  "message": "Account deleted successfully"
}
```

---

### `POST /api/v1/auth/establish-session`

Establish a session (set cookies) on a school subdomain. Used when redirecting a user from the main login page to their school's subdomain.

Verifies the user is a member of the organization matching the provided slug. Returns `403` if not.

**Auth:** Required (Bearer token)

**Request:**
```json
{
  "organization_slug": "springfield-high-school",
  "refresh_token": "6sVQ..."
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `organization_slug` | string | yes | Slug from subdomain hostname |
| `refresh_token` | string | no | If provided, sets refresh cookie too |

**Response `200`:**
```json
{
  "user": { "id": "...", "email": "...", "role": "user", "..." : "..." },
  "message": "Session established",
  "subdomain_url": "http://springfield-high-school.localhost:3000"
}
```

Sets `session_token` (and optionally `refresh_token`) as HttpOnly cookies for the subdomain.

| Error | Status | When |
|-------|--------|------|
| Org not found | `404` | Slug doesn't match any organization |
| Not a member | `403` | User doesn't belong to this organization |

---

### `POST /api/v1/auth/create-organization`

Create a school organization for the authenticated user. Used after email verification in the admin signup flow.

**Auth:** Required (Bearer token)

**Request:**
```json
{
  "school_name": "Springfield High School",
  "refresh_token": "6sVQ..."
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `school_name` | string | yes | Name of the school |
| `refresh_token` | string | no | Pass from verify-email response to avoid cookie issues |

**Response `201`:**
```json
{
  "user": { "id": "...", "email": "...", "role": "user", "..." : "..." },
  "organization": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "name": "Springfield High School",
    "slug": "springfield-high-school",
    "created_at": "2026-03-19T10:00:00Z"
  },
  "message": "School organization created successfully",
  "access_token": "eyJ...",
  "subdomain_url": "http://springfield-high-school.localhost:3000"
}
```

| Error | Status | When |
|-------|--------|------|
| Already in org | `409` | User already belongs to an organization |

---

### `POST /api/v1/auth/refresh`

Refresh the access token using a refresh token cookie.

**Request:** No body. Reads the `refresh_token` HttpOnly cookie.

**Response `200`:**
```json
{
  "message": "Token refreshed"
}
```

Sets new `session_token` and `refresh_token` cookies.

| Error | Status | When |
|-------|--------|------|
| No refresh token | `401` | Cookie missing or expired |
| Account deactivated | `403` | User account is disabled |

---

### `POST /api/v1/auth/logout`

Clear session cookies and revoke the refresh token.

**Response `200`:**
```json
{
  "message": "Logged out successfully"
}
```

---

### `GET /api/v1/auth/authorize`

Get an OAuth authorization URL for social login (Google, Microsoft, GitHub, Apple).

**Query Parameters:**

| Param | Type | Required | Notes |
|-------|------|----------|-------|
| `provider` | string | no | `GoogleOAuth`, `MicrosoftOAuth`, `GitHubOAuth`, `AppleOAuth` |
| `connection_id` | string | no | WorkOS connection ID for enterprise SSO |
| `organization_id` | string | no | WorkOS organization ID |

**Response `200`:**
```json
{
  "authorization_url": "https://accounts.google.com/o/oauth2/v2/auth?..."
}
```

Redirect the user to `authorization_url`. After authentication, they'll be redirected back to the callback URL.

---

### `GET /api/v1/auth/callback`

OAuth callback endpoint. WorkOS redirects here after OAuth/SSO authentication. Sets session cookies and redirects to the frontend's `post_login_redirect_url`.

**Query Parameters:** `code`, `state` (set automatically by WorkOS redirect)

**Response:** `302` redirect to frontend with cookies set.

---

## Response Types

### UserResponse

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "admin@springfield-high.edu",
  "first_name": "Jane",
  "last_name": "Smith",
  "email_verified": true,
  "profile_picture_url": "https://...",
  "organization_id": "660e8400-e29b-41d4-a716-446655440000",
  "role": "admin",
  "created_at": "2026-03-19T10:00:00Z"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Internal user ID |
| `email` | string | |
| `first_name` | string? | Nullable |
| `last_name` | string? | Nullable |
| `email_verified` | boolean | |
| `profile_picture_url` | string? | Nullable |
| `organization_id` | UUID? | Null if user has no school |
| `role` | string | `"user"`, `"admin"`, `"teacher"`, etc. |
| `created_at` | ISO 8601 | |

### OrganizationResponse

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440000",
  "name": "Springfield High School",
  "slug": "springfield-high-school",
  "domain": null,
  "created_at": "2026-03-19T10:00:00Z"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Internal org ID |
| `name` | string | Display name |
| `slug` | string | URL-safe identifier (used in subdomains) |
| `domain` | string? | Custom domain if configured |
| `created_at` | ISO 8601 | |

### AuthResponse

Returned by login, signup, verify-email, and establish-session.

```json
{
  "user": { "id": "...", "email": "...", "role": "user", "..." : "..." },
  "message": "Login successful",
  "access_token": "eyJ...",
  "refresh_token": "6sVQ...",
  "subdomain_url": "http://springfield-high-school.localhost:3000"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `user` | UserResponse | |
| `message` | string | Human-readable status |
| `access_token` | string? | JWT. Only in dev (`expose_token_in_response=true`) |
| `refresh_token` | string? | Only in dev |
| `subdomain_url` | string? | Null if user has no organization |

### AdminSignupResponse

Returned by admin-signup (success) and create-organization.

```json
{
  "user": { "id": "...", "email": "...", "role": "user", "..." : "..." },
  "organization": { "id": "...", "name": "...", "slug": "...", "..." : "..." },
  "message": "School admin account created successfully",
  "access_token": "eyJ...",
  "subdomain_url": "http://springfield-high-school.localhost:3000"
}
```
