# School Setup — Frontend Integration Guide

This guide explains how to integrate the school setup wizard with the backend API.

---

## Overview

School setup is a 12-section configuration wizard that admins complete after creating their school. The backend stores setup data as a single JSONB document per organization, supporting partial saves so admins can complete it across multiple sessions.

```text
Admin creates school → Redirected to subdomain → Setup wizard loads
                                                        │
                                         ┌──────────────┴──────────────┐
                                         │  GET /api/v1/schools/setup  │
                                         │  Load existing draft        │
                                         └──────────────┬──────────────┘
                                                        │
                                         ┌──────────────▼──────────────┐
                                         │  Admin fills sections       │
                                         │  (identity, branding, ...)  │
                                         └──────────────┬──────────────┘
                                                        │ on change (debounced)
                                         ┌──────────────▼──────────────┐
                                         │ PATCH /api/v1/schools/setup │
                                         │ Save partial data           │
                                         └─────────────────────────────┘
```

---

## Endpoints

| Method | Path | Auth | Who | Purpose |
|--------|------|------|-----|---------|
| `GET` | `/api/v1/schools/setup` | Bearer token | Any org member | Load saved setup |
| `PATCH` | `/api/v1/schools/setup` | Bearer token | Admin only | Save partial setup |
| `GET` | `/api/v1/schools/{slug}/public` | None | Anyone | Get public branding |

---

## The 12 Sections

Each section is a top-level key in the setup JSON. The backend tracks which sections are "complete" based on required fields.

### 1. Identity
```json
{
  "identity": {
    "school_type": "secondary",
    "ownership_type": "private",
    "motto": "Excellence in Education",
    "founded_year": "1995",
    "accreditation_number": "SCH/2024/001"
  }
}
```
**Required fields:** `school_type`, `motto`

### 2. Branding
```json
{
  "branding": {
    "logo_url": "https://cdn.example.com/logo.png",
    "primary_color": "#0891B2",
    "secondary_color": "#10B981"
  }
}
```
**Required fields:** `primary_color`, `secondary_color`

### 3. Location
```json
{
  "location": {
    "country": "NG",
    "state_region": "Lagos",
    "city": "Ikeja",
    "timezone": "Africa/Lagos"
  }
}
```
**Required fields:** `country`, `timezone`

### 4. Localization
```json
{
  "localization": {
    "currency": "NGN",
    "date_format": "DD/MM/YYYY",
    "language": "en"
  }
}
```
**Required fields:** `currency`, `date_format`, `language`

### 5. Academic Calendar
```json
{
  "academic_calendar": {
    "calendar_type": "trimester",
    "current_academic_year": "2025/2026",
    "terms": [
      { "name": "First Term", "start_date": "2025-09-01", "end_date": "2025-12-15" },
      { "name": "Second Term", "start_date": "2026-01-10", "end_date": "2026-04-05" },
      { "name": "Third Term", "start_date": "2026-04-20", "end_date": "2026-07-15" }
    ]
  }
}
```
**Required fields:** `calendar_type`, `current_academic_year`

### 6. Grade Levels
```json
{
  "grade_levels": {
    "grade_level_structure_id": "ng_6334",
    "grade_levels": ["Primary 1", "Primary 2", "JSS 1", "JSS 2", "SSS 1", "SSS 2"],
    "group_sections": {
      "Primary": ["Section A", "Section B"],
      "Secondary": ["Arm A", "Arm B", "Arm C"]
    },
    "custom_group_levels": {
      "Primary": ["Primary 1", "Primary 2"],
      "Secondary": ["JSS 1", "JSS 2", "SSS 1", "SSS 2"]
    }
  }
}
```
**Required fields:** `grade_levels` (the array)

### 7. Grading
```json
{
  "grading": {
    "grading_preset_id": "waec",
    "grading_scale": [
      { "grade": "A1", "min_score": "75", "max_score": "100", "descriptor": "Excellent", "gpa_points": "4.0" },
      { "grade": "B2", "min_score": "70", "max_score": "74", "descriptor": "Very Good", "gpa_points": "3.7" },
      { "grade": "C4", "min_score": "60", "max_score": "64", "descriptor": "Credit", "gpa_points": "3.0" },
      { "grade": "F9", "min_score": "0", "max_score": "39", "descriptor": "Fail", "gpa_points": "0.0" }
    ],
    "ca_weight": "40",
    "exam_weight": "60",
    "passmark": "40",
    "gpa_enabled": false,
    "assignment_weight": "",
    "test_weight": "",
    "project_weight": ""
  }
}
```
**Required fields:** `grading_scale`, `ca_weight`, `exam_weight`, `passmark`

### 8. Schedule
```json
{
  "schedule": {
    "schedules": {
      "Primary": {
        "start_time": "08:00",
        "end_time": "15:00",
        "period_duration": "40",
        "periods": [
          { "label": "Period 1", "start_time": "08:00", "end_time": "08:40", "is_break": false },
          { "label": "Short Break", "start_time": "08:40", "end_time": "08:55", "is_break": true },
          { "label": "Period 2", "start_time": "08:55", "end_time": "09:35", "is_break": false }
        ]
      }
    }
  }
}
```
**Required fields:** `schedules` (the object, must be non-empty)

### 9. Subjects
```json
{
  "subjects": {
    "subjects": ["Mathematics", "English Language", "Physics", "Chemistry"],
    "subject_departments": {
      "Mathematics": "science",
      "English Language": "languages",
      "Physics": "science"
    }
  }
}
```
**Required fields:** `subjects` (the array)

### 10. Fees
```json
{
  "fees": {
    "fee_categories": [
      {
        "name": "Tuition",
        "mandatory": true,
        "frequency": "per_term",
        "fee_type": "tuition",
        "applies_to": "all",
        "grade_levels": [],
        "amounts": { "_flat": "50000" }
      },
      {
        "name": "Transport",
        "mandatory": false,
        "frequency": "per_term",
        "fee_type": "transport",
        "applies_to": "specific",
        "grade_levels": ["Primary 1", "Primary 2"],
        "amounts": { "_flat": "15000" }
      }
    ],
    "fee_payment_schedule": "start_of_term",
    "fee_payment_due_day": "5",
    "late_fee_percentage": "5",
    "late_fee_grace_days": "7",
    "discount_types": [
      { "name": "Sibling 2nd child", "percentage": "10", "applies_to": "tuition" }
    ]
  }
}
```
**Required fields:** `fee_categories` (the array)

**Fee type options:** `tuition`, `facility`, `boarding`, `transport`, `exam`, `admin`, `co_curricular`, `one_time_onboarding`

**Frequency options:** `per_term`, `quarterly`, `semi_annual`, `annual`, `one_time`, `monthly`

### 11. Report Card
```json
{
  "report_card": {
    "report_template": "standard",
    "show_assessment_breakdown": true,
    "show_class_average": true,
    "show_highest_lowest": true,
    "show_grading_legend": true,
    "show_position": true,
    "show_gpa": false,
    "show_effort_grades": false,
    "show_behavior_rating": false,
    "show_psychomotor": true,
    "psychomotor_traits": ["Handwriting", "Verbal Fluency", "Creativity", "Sports", "Musical Skills"],
    "show_affective": true,
    "affective_traits": ["Punctuality", "Neatness", "Attentiveness", "Honesty", "Politeness"],
    "show_teacher_comments": true,
    "show_class_teacher_comment": true,
    "show_principal_signature": true,
    "show_subject_teacher_signature": false,
    "comment_char_limit": "200",
    "show_attendance_summary": false,
    "show_next_term_dates": true,
    "show_co_curricular": false
  }
}
```
**Required fields:** `report_template`

**Template options:** `standard`, `detailed`, `minimal`, `uk_style`, `standards_based`, `descriptive`

### 12. Policies & Notifications
```json
{
  "policies": {
    "attendance_tracking_methods": {
      "Primary": "daily",
      "Secondary": "per_subject"
    },
    "late_grace_period": "15",
    "attendance_threshold": "75",
    "tardies_to_absence": "3",
    "consecutive_absence_alert": "3",
    "absence_categories": ["Excused", "Unexcused", "Medical", "School Activity", "Suspended"],

    "promotion_criteria": "automatic",
    "promotion_rules": {
      "min_subjects_to_pass": {
        "Primary": "5",
        "Secondary": "6"
      },
      "overall_pass_percentage": "40",
      "required_subjects": ["Mathematics", "English Language"],
      "allow_remedial": true,
      "max_subjects_for_supplementary": "2",
      "conditional_promotion": true,
      "max_repeats": "1"
    },

    "discipline_framework": "merit_demerit",
    "offense_categories": ["Minor", "Major", "Critical"],
    "consequence_ladder": ["Verbal Warning", "Written Warning", "Detention", "Suspension"],
    "point_reset_period": "per_term",

    "parent_portal": true,
    "report_comments": true,
    "attendance_alerts": true,
    "fee_reminders": true,
    "exam_result_notify": true,
    "behavior_alerts": true,
    "homework_alerts": false,
    "notification_channels": ["email", "sms"]
  }
}
```
**Required fields:** `promotion_criteria`, `discipline_framework`

**Discipline framework options:** `merit_demerit`, `behavior_levels`, `incident_logging`, `house_points`, `restorative`

**Promotion criteria options:** `automatic`, `manual`, `hybrid`

---

## How to Implement the Setup Wizard

### Loading Existing Data

On wizard mount, fetch existing setup:

```typescript
const res = await api('/schools/setup');
const { data, completion } = await res.json();

if (data) {
  // Pre-populate form with existing data
  setFormData(data);
}

// Use completion to show progress
setProgress(completion.completed_sections, completion.total_sections);

// Show checkmarks on completed sections
completion.sections.forEach(section => {
  setSectionComplete(section.name, section.complete);
});
```

### Auto-Saving on Change

Debounce saves to avoid excessive requests. Send the full section object on each save:

```typescript
import { debounce } from 'lodash';

const saveSetup = debounce(async (sectionName: string, sectionData: any) => {
  const res = await fetch('/api/v1/schools/setup', {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${accessToken}`
    },
    body: JSON.stringify({
      [sectionName]: sectionData
    })
  });

  if (res.ok) {
    const { completion } = await res.json();
    setProgress(completion.completed_sections, completion.total_sections);
  }
}, 1000); // 1 second debounce

// Example: when the identity form changes
function onIdentityChange(formValues) {
  saveSetup('identity', formValues);
}
```

### Section Navigation with Completion Tracking

```typescript
const SECTIONS = [
  { key: 'identity', label: 'School Identity' },
  { key: 'branding', label: 'Branding & Colors' },
  { key: 'location', label: 'Location' },
  { key: 'localization', label: 'Regional Settings' },
  { key: 'academic_calendar', label: 'Academic Calendar' },
  { key: 'grade_levels', label: 'Grade Levels' },
  { key: 'grading', label: 'Grading System' },
  { key: 'schedule', label: 'Class Schedule' },
  { key: 'subjects', label: 'Subjects' },
  { key: 'fees', label: 'Fee Structure' },
  { key: 'report_card', label: 'Report Card' },
  { key: 'policies', label: 'Policies & Notifications' },
];
```

### Public Branding (Subdomain Login Page)

On the subdomain login page (before the user is authenticated):

```typescript
const slug = window.location.hostname.split('.')[0];

const res = await fetch(`/api/v1/schools/${slug}/public`);
if (res.ok) {
  const branding = await res.json();
  document.title = branding.name;
  setLogo(branding.logo_url);
  setMotto(branding.motto);
  setCSSVariable('--primary-color', branding.primary_color);
  setCSSVariable('--secondary-color', branding.secondary_color);
}
```

---

## Merge Behavior

The PATCH endpoint uses **top-level key merge**:

```text
Existing data:
{
  "identity": { "school_type": "primary", "motto": "Old" },
  "branding": { "primary_color": "#000" }
}

PATCH body:
{
  "identity": { "school_type": "secondary", "motto": "New" }
}

Result:
{
  "identity": { "school_type": "secondary", "motto": "New" },   ← replaced entirely
  "branding": { "primary_color": "#000" }                        ← preserved
}
```

**Rule:** Always send the complete section object when saving. If you only send `{"identity": {"motto": "New"}}`, the `school_type` field within identity is lost.

**Safe pattern:**
```typescript
const currentIdentity = formData.identity || {};
const updatedIdentity = { ...currentIdentity, motto: newMotto };
saveSetup('identity', updatedIdentity);
```

---

## Completion Logic

A section is "complete" when all its required fields are:
- Present in the data
- Not `null`
- Not an empty string `""`
- Not an empty array `[]`
- Not an empty object `{}`

Numbers (including `0`) and booleans (including `false`) are considered filled.

---

## Common Presets

### Grade Level Structures
| ID | Label | Levels |
|----|-------|--------|
| `ng_6334` | Nigerian 6-3-3-4 | Primary 1-6, JSS 1-3, SSS 1-3 |
| `ng_primary` | Primary Only | Primary 1-6 |
| `ng_secondary` | Secondary Only | JSS 1-3, SSS 1-3 |
| `us_k12` | US K-12 | Kindergarten, Grade 1-12 |

### Grading Presets
| ID | Label | Scale |
|----|-------|-------|
| `waec` | WAEC/NECO (A1-F9) | 9 grades: A1(75-100) to F9(0-39) |
| `percentage` | Percentage-based | A(70-100), B(60-69), C(50-59), D(40-49), F(0-39) |

### Calendar Types
| ID | Label | Terms |
|----|-------|-------|
| `trimester` | 3 Terms | First, Second, Third Term |
| `semester` | 2 Semesters | First, Second Semester |
| `quarterly` | 4 Quarters | Q1, Q2, Q3, Q4 |

These presets are **frontend-only** — the backend stores whatever values you send.

---

## Error Handling

| Status | Meaning | Action |
|--------|---------|--------|
| `200` | Success | Update local state with response data |
| `400` | Invalid body or no org | Show error toast |
| `401` | Token expired | Redirect to login |
| `403` | Not an admin | Show "admin required" message, disable save |
| `404` | School not found (public endpoint) | Show default branding |
| `500` | Server error | Retry with exponential backoff |
