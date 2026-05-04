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
| `admission_number_prefix` | TEXT | yes | | identity section. Used to generate student admission numbers (`{prefix}/{year}/{seq:03}`) |
| `admission_number_seq_year` | SMALLINT | yes | | Internal counter — current year of the sequence |
| `admission_number_next_seq` | INTEGER | no | `1` | Internal counter — next number to assign, resets per year |
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

### `students`

Student records, scoped to an organization. Soft-delete only (`status='withdrawn'`); records are never hard-deleted.

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `org_id` | UUID | no | — | FK → `organizations(id)` **ON DELETE CASCADE** |
| `admission_number` | TEXT | no | — | UNIQUE per `org_id`. Auto-generated unless supplied. |
| `first_name`, `last_name` | TEXT | no | — | |
| `middle_name` | TEXT | yes | | |
| `date_of_birth` | DATE | no | — | |
| `gender` | TEXT | no | — | CHECK: `male`, `female` |
| `grade_level` | TEXT | no | — | Validated against `school_grade_levels` |
| `section`, `stream` | TEXT | yes | | |
| `enrollment_date` | DATE | no | `CURRENT_DATE` | |
| `status` | TEXT | no | `active` | CHECK: `active`, `inactive`, `suspended`, `graduated`, `withdrawn`, `transferred` |
| `boarding_status` | TEXT | yes | | CHECK: `day`, `boarding`, `weekly_boarding` |
| `phone`, `email` | TEXT | yes | | |
| `address`, `city`, `state`, `postal_code` | TEXT | yes | | |
| `blood_group`, `genotype`, `allergies`, `medical_conditions` | TEXT | yes | | |
| `previous_school`, `state_of_origin`, `lga`, `religion`, `tribe` | TEXT | yes | | |
| `avatar_url` | TEXT | yes | | |
| `graduation_date` | DATE | yes | | Required when `status='graduated'` (CHECK constraint) |
| `withdrawn_at` | TIMESTAMPTZ | yes | | Required when `status='withdrawn'` (CHECK constraint) |
| `created_at` | TIMESTAMPTZ | no | `NOW()` | |
| `updated_at` | TIMESTAMPTZ | no | `NOW()` | Auto-updated by trigger |

**Indexes:** `(org_id)`, `(org_id, status)`, `(org_id, grade_level, section)`, `(org_id, last_name)`, `(org_id, enrollment_date DESC)`. Unique on `(org_id, admission_number)`.

`gpa`, `attendance_rate`, and `fee_status` are **not stored** — they're computed by the future grades, attendance, and fees modules and currently returned as null/`"unknown"` by the API.

---

### `student_guardians`

1-to-many from `students`. Up to 3 guardians per student (enforced in the service layer); at most one primary (enforced via partial unique index).

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `student_id` | UUID | no | — | FK → `students(id)` **ON DELETE CASCADE** |
| `org_id` | UUID | no | — | FK → `organizations(id)` (denormalized for scoping) |
| `first_name`, `last_name` | TEXT | no | — | |
| `phone`, `email` | TEXT | yes | | |
| `relationship`, `occupation` | TEXT | yes | | |
| `is_primary` | BOOLEAN | no | `FALSE` | At most one TRUE per student (partial unique index) |
| `position` | SMALLINT | no | `0` | Display order |
| `created_at`, `updated_at` | TIMESTAMPTZ | no | `NOW()` | |

**Indexes:** `(student_id)`, `(org_id)`, partial unique `(student_id) WHERE is_primary`.

---

### `student_status_history`

Audit log for status changes. One row per `PATCH /api/v1/students/{id}/status` call and per `DELETE /api/v1/students/{id}` (soft-delete writes a row with reason `"deleted via API"`).

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `student_id` | UUID | no | — | FK → `students(id)` **ON DELETE CASCADE** |
| `org_id` | UUID | no | — | FK → `organizations(id)` |
| `from_status` | TEXT | no | — | |
| `to_status` | TEXT | no | — | |
| `reason` | TEXT | yes | | |
| `effective_date` | DATE | no | `CURRENT_DATE` | |
| `changed_by_user_id` | UUID | yes | | FK → `users(id)` **ON DELETE SET NULL** |
| `changed_at` | TIMESTAMPTZ | no | `NOW()` | |

**Indexes:** `(student_id, changed_at DESC)`, `(org_id, changed_at DESC)`.

---

### `student_class_history`

Audit log for grade/section changes. Written by `PATCH /api/v1/students/{id}/class` (`change_kind = 'manual'`) and by `POST /api/v1/students/promote` (`change_kind` ∈ `promote`, `retain`, `graduate`, sharing a `promotion_batch_id`).

| Column | Type | Nullable | Default | Notes |
|--------|------|----------|---------|-------|
| `id` | UUID | no | `gen_random_uuid()` | Primary key |
| `student_id` | UUID | no | — | FK → `students(id)` **ON DELETE CASCADE** |
| `org_id` | UUID | no | — | FK → `organizations(id)` |
| `from_grade_level`, `from_section`, `from_stream` | TEXT | yes | | |
| `to_grade_level`, `to_section`, `to_stream` | TEXT | yes | | |
| `change_kind` | TEXT | no | — | CHECK: `promote`, `retain`, `graduate`, `manual` |
| `reason` | TEXT | yes | | |
| `effective_date` | DATE | no | `CURRENT_DATE` | |
| `changed_by_user_id` | UUID | yes | | FK → `users(id)` **ON DELETE SET NULL** |
| `promotion_batch_id` | UUID | yes | | All rows from one `POST /promote` call share this id |
| `changed_at` | TIMESTAMPTZ | no | `NOW()` | |

**Indexes:** `(student_id, changed_at DESC)`, `(org_id, changed_at DESC)`, partial `(promotion_batch_id) WHERE promotion_batch_id IS NOT NULL`.

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
| `20260503000001_create_students.sql` | students, student_guardians, student_status_history, student_class_history |
| `20260503000002_add_admission_number_config.sql` | Add `admission_number_prefix`, `admission_number_seq_year`, `admission_number_next_seq` to school_configs |
| `20260503000003_align_students_schema.sql` | Bidirectional consistency CHECKs on students, composite `(student_id, org_id)` FKs on guardian/history tables, status-history enum CHECKs, `from_stream`/`to_stream` columns on `student_class_history` |

### Running Migrations

```bash
# sqlx-cli requires DATABASE_URL (not APP__DATABASE__URL)
export DATABASE_URL=postgresql://postgres:password@localhost:5432/schoolnify

sqlx migrate run        # Apply pending migrations
sqlx migrate revert     # Rollback last migration
sqlx migrate info       # Show migration status
```
