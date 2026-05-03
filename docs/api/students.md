# Student Endpoints

All endpoints are under `/api/v1/students`. Every endpoint requires authentication; the user's school is resolved from their session (cookie or Bearer JWT) — no `org_id` query parameter needed.

A student is **always scoped to one school**. Cross-tenant requests return `404` (not `403`) to avoid leaking that the resource exists in another school.

For the rationale behind which endpoints are deferred (fees, attendance, grades, report card), see [Deferred Endpoints](#deferred-endpoints) at the bottom.

---

## `GET /api/v1/students`

List students with filters, pagination, and a whole-school summary.

**Auth:** Required (any org member)

**Query parameters:**

| Param | Type | Default | Notes |
|-------|------|---------|-------|
| `search` | string? | — | Matches first/last name, admission_number, or guardian name/phone/email (case-insensitive) |
| `grade_level` | string? | — | Exact match |
| `section` | string? | — | Exact match |
| `status` | string? | `active` | One of `active`, `inactive`, `suspended`, `graduated`, `withdrawn`, `transferred`. Pass `all` to disable the filter. |
| `gender` | string? | — | `male` or `female` |
| `boarding_status` | string? | — | `day`, `boarding`, `weekly_boarding` |
| `page` | int? | `1` | 1-indexed |
| `page_size` | int? | `25` | Max `100` |
| `sort` | string? | `last_name` | One of `last_name`, `first_name`, `admission_number`, `enrollment_date`, `created_at` |
| `order` | string? | `asc` | `asc` or `desc` |

**Response `200`:**
```json
{
  "data": [
    { "id": "std_a1...", "admission_number": "INF/2026/001", "first_name": "Chidera", "last_name": "Okonkwo", "...": "..." }
  ],
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

The `summary` is computed across the entire school and **ignores list filters**. `average_gpa` and `average_attendance` are `null` until the grades and attendance modules ship.

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| No organization | `400` | User is not part of an org |

---

## `POST /api/v1/students`

Create a single student.

**Auth:** Required (any org member)

**Required fields:** `first_name`, `last_name`, `date_of_birth`, `gender`, `grade_level`. Everything else is optional.

**Request:**
```json
{
  "first_name": "Chidera",
  "middle_name": "Grace",
  "last_name": "Okonkwo",
  "date_of_birth": "2017-03-15",
  "gender": "female",
  "grade_level": "Primary 1",
  "section": "A",
  "stream": null,
  "admission_number": null,
  "enrollment_date": "2024-09-09",
  "boarding_status": "day",
  "phone": null,
  "email": null,
  "address": "23 Adeola Odeku Street",
  "city": "Lagos",
  "state": "Lagos",
  "postal_code": "101241",
  "blood_group": "O+",
  "genotype": "AA",
  "allergies": "Peanuts",
  "medical_conditions": null,
  "previous_school": "Sunrise Pre-school",
  "state_of_origin": "Anambra",
  "lga": "Onitsha North",
  "religion": "Christianity",
  "tribe": "Igbo",
  "avatar_url": null,
  "guardians": [
    {
      "first_name": "Emeka",
      "last_name": "Okonkwo",
      "phone": "+2348012345678",
      "email": "emeka.o@example.com",
      "relationship": "Father",
      "occupation": "Engineer",
      "is_primary": true
    }
  ]
}
```

**Auto-generation:**
- If `admission_number` is omitted, the server generates one using the configured `admission_number_prefix` (set via `PATCH /api/v1/schools/setup` `identity` section). Falls back to the school's slug (uppercased) when no prefix is set. Format: `{prefix}/{year}/{seq:03}` (e.g. `INF/2026/001`). The sequence resets per year.
- If `enrollment_date` is omitted, defaults to today.

**Guardians:**
- Up to 3 guardians per student.
- Exactly one guardian may have `is_primary: true`. If none is marked primary, the first guardian in the array becomes primary by convention.

**Response `201`:** Full [Student](#student-object) object including `id`, `admission_number`, `created_at`, `updated_at`.

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Bad request | `400` | Invalid `gender`/`boarding_status`/`grade_level` (must be configured for the school), more than one primary guardian, more than 3 guardians |
| Duplicate admission number | `409` | Admin-supplied `admission_number` already exists in this school |

---

## `GET /api/v1/students/{id}`

Get a single student by id, with optional related data.

**Auth:** Required (any org member)

**Path parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | UUID | Student id |

**Query parameters:**

| Param | Type | Notes |
|-------|------|-------|
| `include` | string? | Comma-separated list. Recognized values: `recent_payments`, `recent_attendance`. |

**Response `200`:** [Student](#student-object) object.

When `include=recent_payments` is set, the response adds `"recent_payments": []` (always empty until the fees module ships). Same for `recent_attendance`.

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Not found | `404` | No student with that id in this school (also returned for cross-tenant access) |

---

## `PATCH /api/v1/students/{id}`

Update student fields.

**Auth:** Required (any org member)

Send only the fields you want to change. Each field uses a `COALESCE` pattern: passing `null` keeps the existing value. Empty string `""` will overwrite the field with empty.

**Cannot be changed via this endpoint:**
- `id`, `admission_number`, `enrollment_date`, `created_at`, `updated_at` — immutable
- `status` — use `PATCH /api/v1/students/{id}/status` (writes audit history)
- `grade_level`, `section` — use `PATCH /api/v1/students/{id}/class` (writes audit history)

**Request (subset):**
```json
{
  "phone": "+2348099999999",
  "address": "New address line",
  "guardians": [
    { "first_name": "Emeka", "last_name": "Okonkwo", "phone": "+2348012345678", "is_primary": true }
  ]
}
```

If `guardians` is included, the **entire** guardian set is replaced — the server deletes existing guardians and inserts the new array. Omit the field to leave guardians unchanged.

**Response `200`:** Updated [Student](#student-object) object.

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Bad request | `400` | Invalid field value (gender, boarding_status, etc.) |
| Not found | `404` | No student with that id in this school |

---

## `DELETE /api/v1/students/{id}`

Soft-delete: marks the student as `withdrawn`, sets `withdrawn_at`, and writes a `student_status_history` audit row (with reason `"deleted via API"`). The record is never hard-deleted.

After deletion the student is excluded from the default list view (which filters to `status=active`). They reappear when querying `?status=withdrawn` or `?status=all`.

**Auth:** Required (any org member)

**Response `204`** (no body).

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Not found | `404` | No student with that id in this school |
| Bad request | `400` | Student already has `status=withdrawn` |

---

## `PATCH /api/v1/students/{id}/status`

Change a student's enrollment status with a reason. Records audit history.

**Auth:** Required (any org member)

**Request:**
```json
{
  "status": "transferred",
  "reason": "Family relocated to UK",
  "effective_date": "2025-05-15"
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `status` | string | yes | One of `active`, `inactive`, `suspended`, `graduated`, `withdrawn`, `transferred` |
| `reason` | string? | no | |
| `effective_date` | date? | no | Defaults to today |

When transitioning to `graduated`, `graduation_date` is set (preserves the existing value if already set). When transitioning to `withdrawn`, `withdrawn_at` is set similarly.

**Response `200`:**
```json
{
  "student": { "id": "std_...", "status": "transferred", "...": "..." },
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

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Bad request | `400` | Invalid `status` value, or status unchanged |
| Not found | `404` | No student with that id in this school |

---

## `PATCH /api/v1/students/{id}/class`

Change a student's `grade_level` and/or `section`. Preserves all academic and attendance history; writes a `student_class_history` audit row with `change_kind: "manual"`.

**Auth:** Required (any org member)

**Request:**
```json
{
  "grade_level": "JSS 2",
  "section": "B",
  "stream": null,
  "effective_date": "2025-09-09",
  "reason": null
}
```

`grade_level` is required and must match a value configured in the school's `grade_levels`. `section` and `stream` are optional and use `COALESCE` (null preserves existing).

**Response `200`:** Updated [Student](#student-object) object.

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Bad request | `400` | `grade_level` not configured for this school |
| Not found | `404` | No student with that id in this school |

---

## `POST /api/v1/students/promote`

Bulk promotion at end of academic session. The most critical operation in any SIS — wrapped in a single transaction. If any decision fails, **all** changes roll back.

Each decision writes one `student_class_history` row sharing a server-generated `promotion_batch_id`, so you can later query a batch's results.

**Auth:** Required (any org member)

**Request:**
```json
{
  "decisions": [
    { "student_id": "std_001", "action": "promote",  "to_grade": "Primary 2", "to_section": "A" },
    { "student_id": "std_002", "action": "promote",  "to_grade": "Primary 2", "to_section": "B" },
    { "student_id": "std_003", "action": "retain",   "reason": "Failed core subjects" },
    { "student_id": "std_004", "action": "graduate" }
  ],
  "academic_year": "2025/2026",
  "effective_date": "2026-09-01"
}
```

| Action | Effect |
|--------|--------|
| `promote` | Updates `grade_level` (required: `to_grade`) and optionally `section`. Writes audit row. |
| `retain` | Leaves `grade_level` unchanged. Writes audit row with the unchanged grade. |
| `graduate` | Sets `status='graduated'` and `graduation_date`. Writes both a status_history and class_history row. |

**Response `200`:**
```json
{
  "promoted": 28,
  "retained": 1,
  "graduated": 1,
  "batch_id": "550e8400-e29b-41d4-a716-446655440000",
  "errors": []
}
```

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Bad request | `400` | Empty `decisions`, invalid `action`, or `promote` missing `to_grade`, or `to_grade` not configured for this school |
| Not found | `404` | One or more `student_id`s do not belong to this school. **Entire batch rolls back.** |

---

## `POST /api/v1/students/bulk-import`

Bulk-create students from a CSV upload.

**Auth:** Required (any org member)

**Request:** `multipart/form-data` with these parts:

| Part | Type | Required | Notes |
|------|------|----------|-------|
| `file` | CSV bytes | yes | Up to **5000 rows**. Headers required. |
| `mapping` | JSON string | yes | Maps each CSV header to a field key (see below). |
| `skip_invalid` | string | no | `"true"` or `"1"` to import valid rows even when some fail. Default: `false` (any error → `422`, no inserts). |

**Field keys recognized in `mapping`:**

`first_name`, `middle_name`, `last_name`, `date_of_birth`, `gender`, `grade_level`, `section`, `stream`, `admission_number`, `enrollment_date`, `boarding_status`, `phone`, `email`, `address`, `city`, `state`, `postal_code`, `blood_group`, `genotype`, `allergies`, `medical_conditions`, `previous_school`, `state_of_origin`, `lga`, `religion`, `tribe`, `avatar_url`.

For guardians, prefix with `guardianN_` where N is 1–3:

`guardian1_first_name`, `guardian1_last_name`, `guardian1_phone`, `guardian1_email`, `guardian1_relationship`, `guardian1_occupation`, `guardian2_first_name`, etc.

**`guardian1` is treated as the primary guardian by convention.**

**Required per row:** `first_name`, `last_name`, `date_of_birth` (must be `YYYY-MM-DD`), `gender` (`male`/`female`), `grade_level`.

**Example mapping:**
```json
{
  "Student First Name": "first_name",
  "Surname": "last_name",
  "DOB": "date_of_birth",
  "Gender": "gender",
  "Class": "grade_level",
  "Father Name": "guardian1_first_name",
  "Father Phone": "guardian1_phone"
}
```

**Response `200`** (success or `skip_invalid=true` with row errors):
```json
{
  "imported": 27,
  "skipped": 3,
  "errors": [
    { "row": 5, "field": "date_of_birth", "message": "must be YYYY-MM-DD" },
    { "row": 12, "field": "grade_level", "message": "grade_level 'Senior High' not configured for this school" },
    { "row": 18, "field": "gender", "message": "must be male or female" }
  ],
  "imported_students": [
    { "id": "std_xxx", "admission_number": "INF/2026/028", "first_name": "Ada", "last_name": "Lovelace" }
  ]
}
```

**Response `422`** (errors and `skip_invalid != true`): same body shape, `imported: 0`, `imported_students: []`. **No rows inserted.**

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Bad request | `400` | Missing `file` or `mapping`, malformed multipart, invalid mapping JSON |
| Validation failed | `422` | At least one row has errors and `skip_invalid != true` |

---

## `GET /api/v1/students/export`

Export filtered student list as CSV. Same query parameters as `GET /api/v1/students`. No pagination — the entire filtered set is returned.

**Auth:** Required (any org member)

**Response `200`:**

```text
Content-Type: text/csv; charset=utf-8
Content-Disposition: attachment; filename="students_2026-05-03.csv"
Cache-Control: no-store
```

CSV columns (in order):

```text
Admission No, First Name, Last Name, Middle Name, Grade, Section, Gender, DOB, Status, Boarding, Fee Status, Guardian Name, Guardian Phone, Guardian Email
```

`Fee Status` is currently `"unknown"` for every row (the fees module doesn't exist yet). `Guardian *` columns reflect the primary guardian; empty if none.

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |

---

## Student object

The full canonical Student response shape.

```json
{
  "id": "std_a1b2c3",
  "admission_number": "INF/2026/001",
  "first_name": "Chidera",
  "middle_name": "Grace",
  "last_name": "Okonkwo",
  "date_of_birth": "2017-03-15",
  "gender": "female",
  "grade_level": "Primary 1",
  "section": "A",
  "stream": null,

  "enrollment_date": "2024-09-09",
  "status": "active",
  "boarding_status": "day",

  "phone": null,
  "email": null,
  "address": "23 Adeola Odeku Street",
  "city": "Lagos",
  "state": "Lagos",
  "postal_code": "101241",

  "guardians": [
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
  ],

  "blood_group": "O+",
  "genotype": "AA",
  "allergies": "Peanuts",
  "medical_conditions": null,
  "previous_school": "Sunrise Pre-school",
  "state_of_origin": "Anambra",
  "lga": "Onitsha North",
  "religion": "Christianity",
  "tribe": "Igbo",
  "avatar_url": null,

  "gpa": null,
  "attendance_rate": null,
  "fee_status": "unknown",

  "created_at": "2024-09-09T08:30:00Z",
  "updated_at": "2025-04-15T14:22:00Z"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Server-generated |
| `admission_number` | string | Unique per school. Format `{prefix}/{year}/{seq:03}`. |
| `gender` | enum | `male` or `female` |
| `status` | enum | `active`, `inactive`, `suspended`, `graduated`, `withdrawn`, `transferred` |
| `boarding_status` | enum? | `day`, `boarding`, `weekly_boarding` |
| `gpa` | float? | **Always `null` until grades module ships.** |
| `attendance_rate` | float? | **Always `null` until attendance module ships.** |
| `fee_status` | string | **Always `"unknown"` until fees module ships.** |
| `guardians` | array | Up to 3, exactly one with `is_primary: true` |
| `created_at`, `updated_at` | ISO 8601 | |

Optional `string?` fields are omitted from the JSON when null.

When `?include=recent_payments` is passed on detail requests, the response includes `"recent_payments": []`. Same for `?include=recent_attendance`.

---

## Admission number configuration

Admission numbers are auto-generated from configuration on `school_configs`. Three columns drive this:

| Column | Type | Set by | Notes |
|--------|------|--------|-------|
| `admission_number_prefix` | string? | Admin via `PATCH /api/v1/schools/setup` (`identity` section) | Falls back to the school slug (uppercased) when null |
| `admission_number_seq_year` | smallint | Server | Year of the current sequence |
| `admission_number_next_seq` | int | Server | Next number to assign |

The frontend's school setup wizard should expose `admission_number_prefix` as a field in the `identity` section. The other two are internal counters managed automatically by `POST /api/v1/students` and `POST /api/v1/students/bulk-import`.

To set a school's prefix:
```json
PATCH /api/v1/schools/setup
{
  "identity": {
    "school_type": "secondary",
    "motto": "Excellence",
    "admission_number_prefix": "INF"
  }
}
```

Sequences reset on year boundary automatically — no manual intervention.

---

## Deferred Endpoints

The following endpoints in the original spec are **not implemented** in this release. Their underlying modules don't exist yet on the backend, and stubbing them would create migration debt when the real schemas land.

| Endpoint | Reason | Tracking |
|----------|--------|----------|
| `GET /api/v1/students/{id}/fees` | No fees ledger / payment table yet | Needs Fees module spec |
| `POST /api/v1/students/{id}/fees/payments` | Same | Same |
| `GET /api/v1/students/{id}/attendance` | No attendance-taking module | Needs Attendance module spec |
| `GET /api/v1/students/{id}/grades` | No gradebook | Needs Grades module spec |
| `GET /api/v1/students/{id}/report-card` | Depends on grades + attendance + report templates | Comes after the above three |

The frontend should keep using demo data for the views these endpoints would power until the corresponding modules ship. The `Student` response intentionally returns `gpa: null`, `attendance_rate: null`, `fee_status: "unknown"` so list/detail pages can still render without these endpoints.

When the modules do ship, the `recent_payments` / `recent_attendance` `?include=` keys on `GET /api/v1/students/{id}` will start returning real data — no breaking change.
