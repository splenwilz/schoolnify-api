# School Endpoints

All school endpoints are under `/api/v1/schools/`.

For the full setup wizard integration guide (sections, presets, auto-save patterns), see [SCHOOL_SETUP.md](../SCHOOL_SETUP.md).

---

## `GET /api/v1/schools/setup`

Get the saved school setup state and section completion metadata for the authenticated user's organization.

**Auth:** Required (any org member)

**Response `200` (setup exists):**
```json
{
  "data": {
    "identity": {
      "school_type": "secondary",
      "motto": "Excellence in Education",
      "founded_year": "1995",
      "accreditation_number": "SCH/2024/001",
      "logo_url": "https://example.com/logo.png"
    },
    "branding": {
      "primary_color": "#0891B2",
      "secondary_color": "#10B981"
    },
    "location": {
      "country": "Nigeria",
      "state_region": "Lagos",
      "city": "Ikeja",
      "timezone": "Africa/Lagos"
    }
  },
  "completion": {
    "total_sections": 12,
    "completed_sections": 3,
    "sections": [
      {
        "name": "identity",
        "complete": true,
        "required_fields": ["school_type", "motto"],
        "missing_fields": []
      },
      {
        "name": "branding",
        "complete": true,
        "required_fields": ["primary_color", "secondary_color"],
        "missing_fields": []
      },
      {
        "name": "grading",
        "complete": false,
        "required_fields": ["grading_scale", "ca_weight", "exam_weight", "passmark"],
        "missing_fields": ["grading_scale", "ca_weight", "exam_weight", "passmark"]
      }
    ]
  },
  "updated_at": "2026-04-03T14:30:00Z"
}
```

**Response `200` (no setup saved yet):**
```json
{
  "data": null,
  "completion": {
    "total_sections": 12,
    "completed_sections": 0,
    "sections": [
      { "name": "identity", "complete": false, "required_fields": ["school_type", "motto"], "missing_fields": ["school_type", "motto"] }
    ]
  }
}
```

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| No organization | `400` | User is not part of an org |

---

## `PATCH /api/v1/schools/setup`

Save partial school setup data. Merges incoming top-level keys with existing data using PostgreSQL JSONB merge. All fields are optional. The frontend can call this on every field change (debounced) — it is idempotent.

**Auth:** Required (admin only)

**Request:**
```json
{
  "identity": {
    "school_type": "secondary",
    "motto": "Excellence in Education",
    "founded_year": "1995",
    "accreditation_number": "SCH/2024/001",
    "logo_url": "https://example.com/logo.png"
  }
}
```

Only include the sections you want to update. Sections not included are preserved unchanged. Sending `{"branding": {"primary_color": "#000"}}` does NOT delete the existing `identity` section.

**Important:** Each top-level key (section) is replaced entirely. If you send `{"identity": {"motto": "New"}}`, the entire `identity` section becomes `{"motto": "New"}` — any previous `school_type` in that section is lost. Always send the full section object.

**Response `200`:**
```json
{
  "data": {
    "identity": { "school_type": "secondary", "motto": "Excellence in Education" }
  },
  "completion": {
    "total_sections": 12,
    "completed_sections": 1,
    "sections": [
      { "name": "identity", "complete": true, "required_fields": ["school_type", "motto"], "missing_fields": [] }
    ]
  },
  "updated_at": "2026-04-03T14:30:00Z"
}
```

| Error | Status | When |
|-------|--------|------|
| Not authenticated | `401` | Missing or invalid token |
| Not admin | `403` | User role is not `"admin"` |
| Invalid body | `400` | Body is not a JSON object |
| No organization | `400` | User is not part of an org |

---

## `GET /api/v1/schools/{slug}/public`

Get public branding info for a school. **No authentication required.** Used by the subdomain login page to display school branding before the user logs in.

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `slug` | string | Organization URL slug (e.g. `springfield-high-school`) |

**Response `200`:**
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

Fields (`logo_url`, `motto`, `primary_color`, `secondary_color`) are omitted if not set in the school setup.

| Error | Status | When |
|-------|--------|------|
| School not found | `404` | Slug doesn't match any active organization |
