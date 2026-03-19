# Frontend Integration Guide

This guide covers how to integrate a Next.js frontend with the Schoolnify API, including the multi-tenant subdomain architecture.

---

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Development Setup](#development-setup)
- [Authentication Flows](#authentication-flows)
  - [Admin Signup (New School)](#1-admin-signup-new-school)
  - [Login (Main Site → Subdomain)](#2-login-main-site--subdomain)
  - [Login (From Subdomain)](#3-login-directly-from-subdomain)
  - [Establish Session on Subdomain](#4-establish-session-on-subdomain)
  - [Token Refresh](#5-token-refresh)
  - [Logout](#6-logout)
- [API Client Setup](#api-client-setup)
- [Subdomain Routing](#subdomain-routing)
- [Production Considerations](#production-considerations)

---

## Architecture Overview

```text
                    ┌──────────────────────────────┐
                    │   Main Site (localhost:3000)  │
                    │   - Landing page             │
                    │   - Login / Signup forms      │
                    └──────────────┬───────────────┘
                                   │ login → gets subdomain_url
                                   │ redirect with tokens
                                   ▼
┌─────────────────────────────────────────────────────────────┐
│   School Subdomain (springfield.localhost:3000)              │
│   - Dashboard                                                │
│   - All school-specific pages                                │
│   - Next.js proxy: /api/v1/* → localhost:8080/api/v1/*       │
└──────────────────────────────┬──────────────────────────────┘
                               │ proxied API calls
                               ▼
                    ┌──────────────────────────────┐
                    │   Schoolnify API (:8080)     │
                    │   - Auth (WorkOS)            │
                    │   - User management          │
                    │   - Organization management  │
                    └──────────────────────────────┘
```

**Key concepts:**
- Each school gets a subdomain: `{slug}.localhost:3000` (dev) or `{slug}.schoolnify.com` (prod)
- The frontend proxies API calls so the browser sees same-origin requests
- Authentication uses Bearer tokens (from JSON responses), not cookies through the proxy
- The `/establish-session` endpoint sets cookies directly on the subdomain when needed

---

## Development Setup

### Backend
```bash
cd schoolnify-api
cp .env.example .env
# Edit .env with your WorkOS credentials

export DATABASE_URL=postgresql://postgres:password@localhost:5432/schoolnify
sqlx database create
sqlx migrate run

cargo run  # Starts on http://localhost:8080
```

### Frontend

In your Next.js `next.config.js`, add a rewrite to proxy API requests:

```javascript
// next.config.js
module.exports = {
  async rewrites() {
    return [
      {
        source: '/api/v1/:path*',
        destination: 'http://localhost:8080/api/v1/:path*',
      },
    ];
  },
};
```

For subdomain support in local development, `*.localhost` resolves automatically in modern browsers. No `/etc/hosts` changes needed.

---

## Authentication Flows

### 1. Admin Signup (New School)

This is a multi-step flow: signup → verify email → create organization → redirect to subdomain.

```javascript
// Step 1: Admin signup
const signupRes = await fetch('/api/v1/auth/admin-signup', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'admin@school.edu',
    password: 'SecurePass123!',
    first_name: 'Jane',
    last_name: 'Smith',
    school_name: 'Springfield Academy'
  })
});
const signup = await signupRes.json();
// signup.status === 202 → email verification required
// Save: signup.pending_authentication_token, signup.school_name, signup.user_id

// Step 2: Verify email (user enters 6-digit code from email)
const verifyRes = await fetch('/api/v1/auth/verify-email', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    code: '123456',
    pending_authentication_token: signup.pending_authentication_token
  })
});
const verified = await verifyRes.json();
// Save: verified.access_token, verified.refresh_token

// Step 3: Create the school organization
const orgRes = await fetch('/api/v1/auth/create-organization', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${verified.access_token}`
  },
  body: JSON.stringify({
    school_name: signup.school_name,
    refresh_token: verified.refresh_token   // pass directly, don't rely on cookies
  })
});
const org = await orgRes.json();
// org.subdomain_url = "http://springfield-academy.localhost:3000"
// org.access_token = new JWT with org_id claim

// Step 4: Redirect to the school subdomain
const url = new URL(org.subdomain_url);
url.pathname = '/auth/callback';
url.searchParams.set('token', org.access_token);
url.searchParams.set('rt', verified.refresh_token);
window.location.href = url.toString();
```

### 2. Login (Main Site → Subdomain)

User logs in from the main site and is redirected to their school subdomain.

```javascript
// On the main login page (localhost:3000/signin)
const res = await fetch('/api/v1/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ email, password })
});
const data = await res.json();

if (res.status === 200 && data.subdomain_url) {
  // User belongs to an organization → redirect to subdomain
  const url = new URL(data.subdomain_url);
  url.pathname = '/auth/callback';
  url.searchParams.set('token', data.access_token);
  url.searchParams.set('rt', data.refresh_token);
  window.location.href = url.toString();

} else if (res.status === 200 && !data.subdomain_url) {
  // User has no organization → show "create organization" page
  // Store access_token for subsequent API calls

} else if (res.status === 403) {
  // Email verification required
  // Show verification form, use data.pending_authentication_token
}
```

### 3. Login (Directly From Subdomain)

If the user is already on a subdomain (e.g. `springfield-academy.localhost:3000/signin`):

```javascript
const res = await fetch('/api/v1/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ email, password })
});
const data = await res.json();

if (res.ok) {
  // Store tokens in memory/state
  setAccessToken(data.access_token);
  setRefreshToken(data.refresh_token);

  // Use Bearer token for all subsequent requests
  router.push('/dashboard');
}
```

### 4. Establish Session on Subdomain

When a user lands on a subdomain via redirect (from main login or admin signup), the subdomain needs to establish its own session.

Create a page at `/auth/callback` on the frontend:

```javascript
// pages/auth/callback.js (or app/auth/callback/page.tsx)
'use client';

import { useEffect } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';

export default function AuthCallback() {
  const router = useRouter();
  const params = useSearchParams();

  useEffect(() => {
    const token = params.get('token');
    const rt = params.get('rt');

    if (!token) {
      router.push('/signin');
      return;
    }

    // Extract org slug from subdomain
    const slug = window.location.hostname.split('.')[0];

    // Establish session (sets cookies on this subdomain)
    fetch('/api/v1/auth/establish-session', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`
      },
      body: JSON.stringify({
        organization_slug: slug,
        refresh_token: rt || undefined
      })
    })
    .then(res => {
      if (res.ok) {
        // Session established — store token and go to dashboard
        // Clean URL by removing token params
        window.history.replaceState({}, '', '/dashboard');
        router.push('/dashboard');
      } else if (res.status === 403) {
        // User is not a member of this organization
        alert('You do not have access to this school.');
        window.location.href = 'http://localhost:3000/signin';
      } else {
        // Token expired or invalid
        window.location.href = 'http://localhost:3000/signin';
      }
    });
  }, []);

  return <div>Signing you in...</div>;
}
```

### 5. Token Refresh

Access tokens expire after 15 minutes. To refresh:

```javascript
// Option A: If cookies are working (direct subdomain login)
const res = await fetch('/api/v1/auth/refresh', { method: 'POST' });

// Option B: Using stored refresh token with Bearer
// (requires a custom refresh endpoint or re-login)
```

### 6. Logout

```javascript
await fetch('/api/v1/auth/logout', { method: 'POST' });
// Clear any stored tokens in frontend state
setAccessToken(null);
setRefreshToken(null);
router.push('/signin');
```

---

## API Client Setup

Create a reusable API client that handles auth headers:

```typescript
// lib/api.ts

let accessToken: string | null = null;

export function setAccessToken(token: string | null) {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
}

export async function api(path: string, options: RequestInit = {}) {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string>),
  };

  if (accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  const res = await fetch(`/api/v1${path}`, {
    ...options,
    headers,
  });

  // Auto-redirect on 401
  if (res.status === 401) {
    setAccessToken(null);
    window.location.href = '/signin';
    throw new Error('Unauthorized');
  }

  return res;
}

// Usage:
// const res = await api('/auth/me');
// const user = await res.json();
```

---

## Subdomain Routing

### Extracting the Organization Slug

```typescript
// lib/subdomain.ts

export function getOrgSlug(): string | null {
  const hostname = window.location.hostname;

  // Development: springfield-academy.localhost
  if (hostname.endsWith('.localhost')) {
    return hostname.replace('.localhost', '');
  }

  // Production: springfield-academy.schoolnify.com
  const baseDomain = 'schoolnify.com';
  if (hostname.endsWith(`.${baseDomain}`)) {
    return hostname.replace(`.${baseDomain}`, '');
  }

  return null; // Main site, no subdomain
}

export function isSubdomain(): boolean {
  return getOrgSlug() !== null;
}
```

### Next.js Middleware for Subdomain Routing

```typescript
// middleware.ts
import { NextRequest, NextResponse } from 'next/server';

export function middleware(request: NextRequest) {
  const hostname = request.headers.get('host') || '';
  const slug = hostname.split('.')[0];

  // If on a subdomain, rewrite to the school-specific pages
  if (hostname.includes('.localhost') && slug !== 'localhost') {
    // Rewrite /dashboard → /school/[slug]/dashboard
    const url = request.nextUrl.clone();
    url.pathname = `/school/${slug}${url.pathname}`;
    return NextResponse.rewrite(url);
  }

  return NextResponse.next();
}
```

---

## Production Considerations

### Environment Variables

For production, the backend uses these overrides:

```bash
# Cookie security
APP__AUTH__COOKIE_SECURE=true
APP__AUTH__COOKIE_SAME_SITE=lax
APP__AUTH__EXPOSE_TOKEN_IN_RESPONSE=false  # Never expose tokens in JSON

# CORS
APP__CORS__ALLOWED_ORIGINS=https://schoolnify.com
APP__CORS__BASE_DOMAIN=schoolnify.com

# Database
APP__DATABASE__URL=postgresql://user:pass@db-host:5432/schoolnify
```

### DNS Setup (Cloudflare)

1. **A record:** `schoolnify.com` → your server IP
2. **A record:** `api.schoolnify.com` → your API server IP
3. **Wildcard CNAME:** `*.schoolnify.com` → `schoolnify.com` (proxied)

### Cookie Behavior in Production

In production with `expose_token_in_response=false`:
- Tokens are only in HttpOnly cookies (not in JSON responses)
- The API sets `Secure=true` cookies over HTTPS
- Subdomains share cookies via `Domain=.schoolnify.com`
- The establish-session flow is not needed (cookies work across subdomains)

### Security Checklist

- [ ] `APP__AUTH__EXPOSE_TOKEN_IN_RESPONSE=false` in production
- [ ] `APP__AUTH__COOKIE_SECURE=true` in production
- [ ] HTTPS enforced on all endpoints
- [ ] WorkOS production API keys (not `sk_test_*`)
- [ ] `APP__CORS__ALLOWED_ORIGINS` set to production domain only
- [ ] Database credentials rotated from development defaults
