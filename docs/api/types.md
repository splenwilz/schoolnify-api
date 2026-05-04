# Shared Response Types

These types are used across multiple endpoints.

---

## UserResponse

Returned in auth responses and user profile endpoints.

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
| `organization_id` | UUID? | Omitted if user has no school |
| `role` | string | `"user"`, `"admin"`, `"teacher"`, etc. |
| `created_at` | ISO 8601 | |

---

## OrganizationResponse

Returned in admin signup and create-organization responses.

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440000",
  "name": "Springfield High School",
  "slug": "springfield-high-school",
  "created_at": "2026-03-19T10:00:00Z"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Internal org ID |
| `name` | string | Display name |
| `slug` | string | URL-safe identifier (used in subdomains) |
| `domain` | string? | Omitted if not configured |
| `created_at` | ISO 8601 | |

---

## AuthResponse

Returned by login, signup, verify-email, and establish-session.

```json
{
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "john@example.com",
    "first_name": "John",
    "last_name": "Doe",
    "email_verified": true,
    "profile_picture_url": null,
    "role": "user",
    "created_at": "2026-03-20T10:00:00Z"
  },
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
| `subdomain_url` | string? | Omitted if user has no organization |

---

## AdminSignupResponse

Returned by admin-signup (success) and create-organization.

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
    "created_at": "2026-03-20T10:00:00Z"
  },
  "organization": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "name": "Springfield High School",
    "slug": "springfield-high-school",
    "created_at": "2026-03-20T10:00:00Z"
  },
  "message": "School admin account created successfully",
  "access_token": "eyJ...",
  "subdomain_url": "http://springfield-high-school.localhost:3000"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `user` | UserResponse | |
| `organization` | OrganizationResponse | |
| `message` | string | |
| `access_token` | string? | Only in dev |
| `subdomain_url` | string | Subdomain URL for the new school |

---

## SchoolSetupResponse

Returned by GET and PATCH `/api/v1/schools/setup`.

```json
{
  "data": { "identity": { "..." : "..." }, "branding": { "..." : "..." } },
  "completion": {
    "total_sections": 12,
    "completed_sections": 2,
    "sections": [
      { "name": "identity", "complete": true, "required_fields": ["school_type", "motto"], "missing_fields": [] },
      { "name": "grading", "complete": false, "required_fields": ["grading_scale", "ca_weight", "exam_weight", "passmark"], "missing_fields": ["grading_scale"] }
    ]
  },
  "updated_at": "2026-04-03T14:30:00Z"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `data` | object? | `null` if no setup saved |
| `completion` | SetupCompletion | Section completion metadata |
| `updated_at` | ISO 8601? | Omitted if never saved |

---

## PublicBrandingResponse

Returned by GET `/api/v1/schools/{slug}/public`.

```json
{
  "name": "Springfield High School",
  "slug": "springfield-high-school",
  "logo_url": "https://example.com/logo.png",
  "motto": "Excellence in Education",
  "primary_color": "#0891B2",
  "secondary_color": "#10B981"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | School name |
| `slug` | string | URL slug |
| `logo_url` | string? | Omitted if not set |
| `motto` | string? | Omitted if not set |
| `primary_color` | string? | Omitted if not set |
| `secondary_color` | string? | Omitted if not set |

---

## StudentResponse

Returned by `GET /api/v1/students/{id}`, in the `data` array of `GET /api/v1/students`, and in many other student endpoints. See [students.md → Student object](students.md#student-object) for the canonical shape and field-level notes.

Key callouts:

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Server-generated |
| `admission_number` | string | Unique per school. Auto-generated values follow `{prefix}/{year}/{seq:03}`; admin-supplied values are accepted as-is. |
| `gender` | enum | `male` or `female` |
| `status` | enum | `active`, `inactive`, `suspended`, `graduated`, `withdrawn`, `transferred` |
| `gpa` | float? | **Always `null` until grades module ships** |
| `attendance_rate` | float? | **Always `null` until attendance module ships** |
| `fee_status` | string | **Always `"unknown"` until fees module ships** |
| `guardians` | GuardianResponse[] | Up to 3 |

---

## GuardianResponse

```json
{
  "id": "grd_xyz789",
  "first_name": "Emeka",
  "last_name": "Okonkwo",
  "phone": "+2348012345678",
  "email": "emeka.o@example.com",
  "relationship": "Father",
  "occupation": "Engineer",
  "is_primary": true
}
```

Optional fields (`phone`, `email`, `relationship`, `occupation`) are omitted when null.

At most one guardian per student has `is_primary: true`.

---

## StudentListResponse

Returned by `GET /api/v1/students`.

```json
{
  "data": [/* StudentResponse[] */],
  "pagination": {
    "page": 1,
    "page_size": 25,
    "total": 30,
    "total_pages": 2
  },
  "summary": {
    "total_students": 30,
    "active": 29,
    "average_gpa": null,
    "average_attendance": null
  }
}
```

`summary` is whole-school stats and **ignores list filters**.

---

## StatusChangeResponse

Returned by `PATCH /api/v1/students/{id}/status`.

```json
{
  "student": { /* StudentResponse */ },
  "status_change": {
    "id": "stchg_...",
    "from_status": "active",
    "to_status": "transferred",
    "reason": "Family relocated to UK",
    "effective_date": "2025-05-15",
    "changed_by": "550e8400-...",
    "changed_at": "2026-05-03T10:30:00Z"
  }
}
```

---

## PromoteSummary

Returned by `POST /api/v1/students/promote`.

```json
{
  "promoted": 28,
  "retained": 1,
  "graduated": 1,
  "batch_id": "550e8400-e29b-41d4-a716-446655440000",
  "errors": []
}
```

`batch_id` is a server-generated UUID shared across all `student_class_history` rows from this call — useful for "show me the results of last September's promotion".

---

## BulkImportResponse

Returned by `POST /api/v1/students/bulk-import` (status `200` on success or partial-with-skip, `422` when errors are present and `skip_invalid != true`).

```json
{
  "imported": 27,
  "skipped": 3,
  "errors": [
    { "row": 5, "field": "date_of_birth", "message": "must be YYYY-MM-DD" }
  ],
  "imported_students": [
    { "id": "std_xxx", "admission_number": "INF/2026/028", "first_name": "Ada", "last_name": "Lovelace" }
  ]
}
```

On `422`: `imported: 0`, `imported_students: []`. **No rows are inserted in this case.**

---

## ErrorResponse

All errors follow this structure.

```json
{
  "error": {
    "type": "UNAUTHORIZED",
    "message": "No authentication token provided"
  }
}
```

| Field | Type | Notes |
|-------|------|-------|
| `error.type` | string | Machine-readable error code |
| `error.message` | string | Human-readable description |
