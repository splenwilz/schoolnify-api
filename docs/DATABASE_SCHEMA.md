# Database Schema

Schoolnify uses PostgreSQL with SQLx for compile-time checked queries. Migrations are in the `migrations/` directory.

---

## Tables

### `users`

Core user table. Each user has a corresponding record in WorkOS.

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `workos_user_id` | TEXT | no | — | **UNIQUE**. WorkOS user ID |
| `email` | TEXT | no | — | **UNIQUE**. User's email |
| `first_name` | TEXT | yes | `NULL` | |
| `last_name` | TEXT | yes | `NULL` | |
| `email_verified` | BOOLEAN | no | `FALSE` | |
| `profile_picture_url` | TEXT | yes | `NULL` | |
| `workos_metadata` | JSONB | no | `'{}'` | Arbitrary WorkOS metadata |
| `last_sign_in_at` | TIMESTAMPTZ | yes | `NULL` | Updated on each login |
| `org_id` | UUID | yes | `NULL` | FK → `organizations(id)` |
| `role` | TEXT | no | `'user'` | `user`, `admin`, `teacher`, etc. |
| `is_active` | BOOLEAN | no | `TRUE` | Soft-delete flag |
| `created_at` | TIMESTAMPTZ | no | `NOW()` | |
| `updated_at` | TIMESTAMPTZ | no | `NOW()` | Auto-updated by trigger |

**Indexes:**
- `users_pkey` — PRIMARY KEY on `id`
- `users_workos_user_id_key` — UNIQUE on `workos_user_id`
- `users_email_unique` — UNIQUE on `email`
- `idx_users_org_id` — on `org_id`
- `idx_users_role` — on `role`
- `idx_users_is_active` — partial index `WHERE is_active = TRUE`

**Trigger:** `update_users_updated_at` — sets `updated_at = NOW()` before each update.

---

### `organizations`

School organizations. Each org has a corresponding record in WorkOS.

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `workos_org_id` | TEXT | no | — | **UNIQUE**. WorkOS org ID |
| `name` | TEXT | no | — | Display name |
| `slug` | TEXT | no | — | **UNIQUE**. URL-safe identifier for subdomains |
| `domain` | TEXT | yes | `NULL` | Optional custom domain |
| `is_active` | BOOLEAN | no | `TRUE` | |
| `created_at` | TIMESTAMPTZ | no | `NOW()` | |
| `updated_at` | TIMESTAMPTZ | no | `NOW()` | Auto-updated by trigger |

**Indexes:**
- `organizations_pkey` — PRIMARY KEY on `id`
- `organizations_workos_org_id_key` — UNIQUE on `workos_org_id`
- `organizations_slug_key` — UNIQUE on `slug`

**Trigger:** `update_organizations_updated_at` — sets `updated_at = NOW()` before each update.

**Slug generation:** The slug is derived from the school name (e.g. "Springfield High School" → `springfield-high-school`). If a slug already exists, a numeric suffix is appended (`-2`, `-3`, etc.).

---

### `refresh_tokens`

Stores hashed refresh tokens for session management.

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `user_id` | UUID | no | — | FK → `users(id)` **ON DELETE CASCADE** |
| `token_hash` | TEXT | no | — | **UNIQUE**. SHA-256 hash of the raw token |
| `expires_at` | TIMESTAMPTZ | no | — | Token expiry (default 30 days) |
| `created_at` | TIMESTAMPTZ | no | `NOW()` | |
| `revoked_at` | TIMESTAMPTZ | yes | `NULL` | Set on logout or token rotation |

**Indexes:**
- `refresh_tokens_pkey` — PRIMARY KEY on `id`
- `refresh_tokens_token_hash_key` — UNIQUE on `token_hash`
- `idx_refresh_tokens_user_id` — on `user_id`
- `idx_refresh_tokens_token_hash` — partial index `WHERE revoked_at IS NULL`
- `idx_refresh_tokens_expires_at` — partial index `WHERE revoked_at IS NULL`

**Token lifecycle:**
1. **Created** on login/signup via `store_refresh_token`
2. **Rotated** on `/refresh` — old token revoked, new one stored
3. **Revoked** on `/logout` — `revoked_at` set to `NOW()`
4. **Cascade deleted** when user is deleted

Raw tokens are never stored — only SHA-256 hashes.

---

### `school_configs`

Parent configuration table (one row per org). Contains ~50 typed scalar columns for school setup preferences. Child tables store array data (grading scales, terms, subjects, etc.).

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `org_id` | UUID | no | — | **UNIQUE** FK → `organizations(id)` **ON DELETE CASCADE** |
| `school_type` | TEXT | yes | | identity section |
| `motto` | TEXT | yes | | identity section |
| `primary_color` | TEXT | yes | | branding section |
| `country` | TEXT | yes | | location section |
| `timezone` | TEXT | yes | | location section |
| `calendar_type` | TEXT | yes | | academic calendar |
| `report_template` | TEXT | yes | | report card |
| `promotion_criteria` | TEXT | yes | | policies |
| ... | ... | ... | | (~50 columns total) |
| `created_at` | TIMESTAMPTZ | no | `NOW()` | |
| `updated_at` | TIMESTAMPTZ | no | `NOW()` | Auto-updated by trigger |

### Child Tables (8 tables, all with `org_id` FK + `position` for ordering)

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `school_grading_scales` | Grading scale rows (A1-F9) | grade, min_score, max_score, descriptor, gpa_points |
| `school_terms` | Academic calendar terms | name, start_date, end_date |
| `school_subjects` | Subject list | name, department |
| `school_grade_levels` | Grade level list | name, group_name |
| `school_fee_categories` | Fee categories | name, mandatory, frequency, fee_type, amounts (JSONB) |
| `school_fee_discounts` | Fee discounts | name, percentage, applies_to |
| `school_schedule_groups` | Schedule groups (e.g. "Primary") | group_name, start_time, end_time, period_duration |
| `school_schedule_periods` | Periods within groups | label, start_time, end_time, is_break (FK → groups) |

All child tables have `ON DELETE CASCADE` from `organizations` (via `org_id` FK). Schedule periods cascade from schedule groups.

---

## Entity Relationship

```text
users                    organizations            school_configs
┌────────────────┐       ┌──────────────────┐     ┌──────────────────┐
│ id (PK)        │       │ id (PK) <────────┐     │ id (PK)          │
│ workos_user_id │       │ workos_org_id    │     │ org_id (FK/UQ)   │
│ email          │       │ name             │     │ school_type      │
│ org_id (FK) ───┼───────┘ slug             │     │ motto, ...       │
│ role           │       │ domain           │     │ (~50 columns)    │
│ ...            │       │ ...              │     └──────────────────┘
└───────┬────────┘       └──────────────────┘            │
        │                                          8 child tables
        │ 1:N (ON DELETE CASCADE)              (grading_scales, terms,
        │                                       subjects, grade_levels,
┌───────┴────────┐                              fee_categories, fee_discounts,
│ refresh_tokens │                              schedule_groups → periods)
│ id (PK)        │
│ user_id (FK)   │
│ token_hash     │
│ expires_at     │
│ revoked_at     │
└────────────────┘
```

---

## Migrations

| File | Description |
|------|-------------|
| `20260220000001_create_users.sql` | Users table, refresh_tokens table, triggers |
| `20260305000001_create_organizations.sql` | Organizations table, `org_id` column on users |
| `20260312000001_add_email_unique_constraint.sql` | UNIQUE constraint on `users.email` |
| `20260403000001_create_school_setups.sql` | Initial school setup (JSONB, later normalized) |
| `20260419000001_normalize_school_setups.sql` | Normalize to school_configs + 8 child tables, drop school_setups |

### Running Migrations

```bash
# sqlx-cli requires DATABASE_URL (not APP__DATABASE__URL)
export DATABASE_URL=postgresql://postgres:password@localhost:5432/schoolnify

sqlx migrate run        # Apply pending migrations
sqlx migrate revert     # Rollback last migration
sqlx migrate info       # Show migration status
```
