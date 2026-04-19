use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde_json::json;

use crate::errors::AppError;
use crate::models::auth::{CurrentUser, ErrorResponse};
use crate::models::school_setup::{PublicBrandingResponse, SchoolSetupData, SchoolSetupResponse};
use crate::services::school_setup::SchoolSetupService;
use crate::state::AppState;

/// Get school setup state
#[utoipa::path(
    get,
    path = "/api/v1/schools/setup",
    tag = "Schools",
    security(("session_cookie" = []), ("bearer_token" = [])),
    responses(
        (status = 200, description = "Setup data with completion metadata", body = SchoolSetupResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 400, description = "User has no organization", body = ErrorResponse),
    )
)]
pub async fn get_setup(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .user_service
        .find_by_workos_id(&current_user.workos_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let org_id = user
        .org_id
        .ok_or_else(|| AppError::BadRequest("User is not part of an organization".into()))?;

    let setup = state.school_setup_service.get_by_org_id(org_id).await?;

    match setup {
        Some(data) => {
            let completion = SchoolSetupService::compute_completion(&data);
            let updated_at = data.updated_at;
            Ok(Json(SchoolSetupResponse {
                data: Some(assemble_json(&data)),
                completion,
                updated_at,
            }))
        }
        None => {
            let empty = SchoolSetupData::empty();
            let completion = SchoolSetupService::compute_completion(&empty);
            Ok(Json(SchoolSetupResponse {
                data: None,
                completion,
                updated_at: None,
            }))
        }
    }
}

/// Update school setup (partial merge)
#[utoipa::path(
    patch,
    path = "/api/v1/schools/setup",
    tag = "Schools",
    security(("session_cookie" = []), ("bearer_token" = [])),
    responses(
        (status = 200, description = "Setup saved", body = SchoolSetupResponse),
        (status = 400, description = "Invalid request body", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Admin role required", body = ErrorResponse),
    )
)]
pub async fn patch_setup(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AppError> {
    if !payload.is_object() {
        return Err(AppError::BadRequest(
            "Request body must be a JSON object".into(),
        ));
    }

    let user = state
        .user_service
        .find_by_workos_id(&current_user.workos_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if user.role != "admin" {
        return Err(AppError::Forbidden(
            "Only admins can modify school setup".into(),
        ));
    }

    let org_id = user
        .org_id
        .ok_or_else(|| AppError::BadRequest("User is not part of an organization".into()))?;

    let data = state
        .school_setup_service
        .upsert_merge(org_id, &payload)
        .await?;

    let completion = SchoolSetupService::compute_completion(&data);
    let updated_at = data.updated_at;

    Ok((
        StatusCode::OK,
        Json(SchoolSetupResponse {
            data: Some(assemble_json(&data)),
            completion,
            updated_at,
        }),
    ))
}

/// Get public school branding
#[utoipa::path(
    get,
    path = "/api/v1/schools/{slug}/public",
    tag = "Schools",
    params(("slug" = String, Path, description = "Organization URL slug")),
    responses(
        (status = 200, description = "Public branding info", body = PublicBrandingResponse),
        (status = 404, description = "School not found", body = ErrorResponse),
    )
)]
pub async fn get_public_branding(
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let branding = state
        .school_setup_service
        .get_public_branding(&slug)
        .await?
        .ok_or_else(|| AppError::NotFound("School not found".into()))?;

    Ok(Json(branding))
}

// ── JSON Assembly ──────────────────────────────────────────────────────

/// Convert relational SchoolSetupData into the JSON shape the frontend expects.
fn assemble_json(data: &SchoolSetupData) -> serde_json::Value {
    let mut root = serde_json::Map::new();

    let Some(c) = &data.config else {
        return serde_json::Value::Object(root);
    };

    // Identity
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "school_type", &c.school_type);
    insert_str(&mut sec, "ownership_type", &c.ownership_type);
    insert_str(&mut sec, "motto", &c.motto);
    insert_str(&mut sec, "founded_year", &c.founded_year);
    insert_str(&mut sec, "accreditation_number", &c.accreditation_number);
    if !sec.is_empty() { root.insert("identity".into(), json!(sec)); }

    // Branding
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "logo_url", &c.logo_url);
    insert_str(&mut sec, "primary_color", &c.primary_color);
    insert_str(&mut sec, "secondary_color", &c.secondary_color);
    if !sec.is_empty() { root.insert("branding".into(), json!(sec)); }

    // Location
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "country", &c.country);
    insert_str(&mut sec, "state_region", &c.state_region);
    insert_str(&mut sec, "city", &c.city);
    insert_str(&mut sec, "timezone", &c.timezone);
    if !sec.is_empty() { root.insert("location".into(), json!(sec)); }

    // Localization
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "currency", &c.currency);
    insert_str(&mut sec, "date_format", &c.date_format);
    insert_str(&mut sec, "language", &c.language);
    if !sec.is_empty() { root.insert("localization".into(), json!(sec)); }

    // Academic Calendar
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "calendar_type", &c.calendar_type);
    insert_str(&mut sec, "current_academic_year", &c.current_academic_year);
    if !data.terms.is_empty() {
        let terms: Vec<serde_json::Value> = data.terms.iter().map(|t| {
            let mut m = serde_json::Map::new();
            m.insert("name".into(), json!(t.name));
            if let Some(ref d) = t.start_date { m.insert("start_date".into(), json!(d)); }
            if let Some(ref d) = t.end_date { m.insert("end_date".into(), json!(d)); }
            json!(m)
        }).collect();
        sec.insert("terms".into(), json!(terms));
    }
    if !sec.is_empty() { root.insert("academic_calendar".into(), json!(sec)); }

    // Grade Levels
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "grade_level_structure_id", &c.grade_level_structure_id);
    if !data.grade_levels.is_empty() {
        let names: Vec<&str> = data.grade_levels.iter().map(|l| l.name.as_str()).collect();
        sec.insert("grade_levels".into(), json!(names));
    }
    insert_json(&mut sec, "group_sections", &c.group_sections);
    insert_json(&mut sec, "custom_group_levels", &c.custom_group_levels);
    if !sec.is_empty() { root.insert("grade_levels".into(), json!(sec)); }

    // Grading
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "grading_preset_id", &c.grading_preset_id);
    insert_str(&mut sec, "ca_weight", &c.ca_weight);
    insert_str(&mut sec, "exam_weight", &c.exam_weight);
    insert_str(&mut sec, "passmark", &c.passmark);
    insert_bool(&mut sec, "gpa_enabled", &c.gpa_enabled);
    insert_str(&mut sec, "assignment_weight", &c.assignment_weight);
    insert_str(&mut sec, "test_weight", &c.test_weight);
    insert_str(&mut sec, "project_weight", &c.project_weight);
    if !data.grading_scales.is_empty() {
        let scales: Vec<serde_json::Value> = data.grading_scales.iter().map(|s| {
            let mut m = serde_json::Map::new();
            m.insert("grade".into(), json!(s.grade));
            m.insert("min_score".into(), json!(s.min_score));
            m.insert("max_score".into(), json!(s.max_score));
            if let Some(ref d) = s.descriptor { m.insert("descriptor".into(), json!(d)); }
            if let Some(ref g) = s.gpa_points { m.insert("gpa_points".into(), json!(g)); }
            json!(m)
        }).collect();
        sec.insert("grading_scale".into(), json!(scales));
    }
    if !sec.is_empty() { root.insert("grading".into(), json!(sec)); }

    // Schedule
    if !data.schedule_groups.is_empty() {
        let mut schedules = serde_json::Map::new();
        for group in &data.schedule_groups {
            let periods: Vec<serde_json::Value> = data.schedule_periods.iter()
                .filter(|p| p.group_id == group.id)
                .map(|p| {
                    let mut m = serde_json::Map::new();
                    m.insert("label".into(), json!(p.label));
                    if let Some(ref t) = p.start_time { m.insert("start_time".into(), json!(t)); }
                    if let Some(ref t) = p.end_time { m.insert("end_time".into(), json!(t)); }
                    m.insert("is_break".into(), json!(p.is_break));
                    json!(m)
                }).collect();
            let mut g = serde_json::Map::new();
            if let Some(ref t) = group.start_time { g.insert("start_time".into(), json!(t)); }
            if let Some(ref t) = group.end_time { g.insert("end_time".into(), json!(t)); }
            if let Some(ref d) = group.period_duration { g.insert("period_duration".into(), json!(d)); }
            g.insert("periods".into(), json!(periods));
            schedules.insert(group.group_name.clone(), json!(g));
        }
        root.insert("schedule".into(), json!({"schedules": schedules}));
    }

    // Subjects
    let mut sec = serde_json::Map::new();
    if !data.subjects.is_empty() {
        let names: Vec<&str> = data.subjects.iter().map(|s| s.name.as_str()).collect();
        sec.insert("subjects".into(), json!(names));
    }
    insert_json(&mut sec, "subject_departments", &c.subject_departments);
    if !sec.is_empty() { root.insert("subjects".into(), json!(sec)); }

    // Fees
    let mut sec = serde_json::Map::new();
    if !data.fee_categories.is_empty() {
        let cats: Vec<serde_json::Value> = data.fee_categories.iter().map(|f| {
            json!({
                "name": f.name,
                "mandatory": f.mandatory,
                "frequency": f.frequency,
                "fee_type": f.fee_type,
                "applies_to": f.applies_to,
                "grade_levels": f.grade_levels,
                "amounts": f.amounts,
            })
        }).collect();
        sec.insert("fee_categories".into(), json!(cats));
    }
    insert_str(&mut sec, "fee_payment_schedule", &c.fee_payment_schedule);
    insert_str(&mut sec, "fee_payment_due_day", &c.fee_payment_due_day);
    insert_str(&mut sec, "late_fee_percentage", &c.late_fee_percentage);
    insert_str(&mut sec, "late_fee_grace_days", &c.late_fee_grace_days);
    if !data.fee_discounts.is_empty() {
        let discs: Vec<serde_json::Value> = data.fee_discounts.iter().map(|d| {
            json!({"name": d.name, "percentage": d.percentage, "applies_to": d.applies_to})
        }).collect();
        sec.insert("discount_types".into(), json!(discs));
    }
    if !sec.is_empty() { root.insert("fees".into(), json!(sec)); }

    // Report Card
    let mut sec = serde_json::Map::new();
    insert_str(&mut sec, "report_template", &c.report_template);
    insert_bool(&mut sec, "show_assessment_breakdown", &c.show_assessment_breakdown);
    insert_bool(&mut sec, "show_class_average", &c.show_class_average);
    insert_bool(&mut sec, "show_highest_lowest", &c.show_highest_lowest);
    insert_bool(&mut sec, "show_grading_legend", &c.show_grading_legend);
    insert_bool(&mut sec, "show_position", &c.show_position);
    insert_bool(&mut sec, "show_gpa", &c.show_gpa);
    insert_bool(&mut sec, "show_effort_grades", &c.show_effort_grades);
    insert_bool(&mut sec, "show_behavior_rating", &c.show_behavior_rating);
    insert_bool(&mut sec, "show_psychomotor", &c.show_psychomotor);
    insert_json(&mut sec, "psychomotor_traits", &c.psychomotor_traits);
    insert_bool(&mut sec, "show_affective", &c.show_affective);
    insert_json(&mut sec, "affective_traits", &c.affective_traits);
    insert_bool(&mut sec, "show_teacher_comments", &c.show_teacher_comments);
    insert_bool(&mut sec, "show_class_teacher_comment", &c.show_class_teacher_comment);
    insert_bool(&mut sec, "show_principal_signature", &c.show_principal_signature);
    insert_bool(&mut sec, "show_subject_teacher_signature", &c.show_subject_teacher_signature);
    insert_str(&mut sec, "comment_char_limit", &c.comment_char_limit);
    insert_bool(&mut sec, "show_attendance_summary", &c.show_attendance_summary);
    insert_bool(&mut sec, "show_next_term_dates", &c.show_next_term_dates);
    insert_bool(&mut sec, "show_co_curricular", &c.show_co_curricular);
    if !sec.is_empty() { root.insert("report_card".into(), json!(sec)); }

    // Policies
    let mut sec = serde_json::Map::new();
    insert_json(&mut sec, "attendance_tracking_methods", &c.attendance_tracking_methods);
    insert_str(&mut sec, "late_grace_period", &c.late_grace_period);
    insert_str(&mut sec, "attendance_threshold", &c.attendance_threshold);
    insert_str(&mut sec, "tardies_to_absence", &c.tardies_to_absence);
    insert_str(&mut sec, "consecutive_absence_alert", &c.consecutive_absence_alert);
    insert_json(&mut sec, "absence_categories", &c.absence_categories);
    insert_str(&mut sec, "promotion_criteria", &c.promotion_criteria);
    insert_json(&mut sec, "promotion_rules", &c.promotion_rules);
    insert_str(&mut sec, "discipline_framework", &c.discipline_framework);
    insert_json(&mut sec, "offense_categories", &c.offense_categories);
    insert_json(&mut sec, "consequence_ladder", &c.consequence_ladder);
    insert_str(&mut sec, "point_reset_period", &c.point_reset_period);
    insert_bool(&mut sec, "parent_portal", &c.parent_portal);
    insert_bool(&mut sec, "report_comments", &c.report_comments);
    insert_bool(&mut sec, "attendance_alerts", &c.attendance_alerts);
    insert_bool(&mut sec, "fee_reminders", &c.fee_reminders);
    insert_bool(&mut sec, "exam_result_notify", &c.exam_result_notify);
    insert_bool(&mut sec, "behavior_alerts", &c.behavior_alerts);
    insert_bool(&mut sec, "homework_alerts", &c.homework_alerts);
    insert_json(&mut sec, "notification_channels", &c.notification_channels);
    if !sec.is_empty() { root.insert("policies".into(), json!(sec)); }

    serde_json::Value::Object(root)
}

// ── Helpers ────────────────────────────────────────────────────────────

fn insert_str(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, val: &Option<String>) {
    if let Some(v) = val {
        map.insert(key.into(), json!(v));
    }
}

fn insert_bool(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, val: &Option<bool>) {
    if let Some(v) = val {
        map.insert(key.into(), json!(v));
    }
}

fn insert_json(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, val: &serde_json::Value) {
    if !val.is_null() && val != &json!({}) && val != &json!([]) {
        map.insert(key.into(), val.clone());
    }
}
