use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::{Extension, Json};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::auth::{CurrentUser, ErrorResponse};
use crate::models::students::{
    BulkImportResponse, ChangeClassRequest, ChangeStatusRequest, CreateStudentRequest,
    PromoteRequest, PromoteSummary, StatusChangeResponse, StudentDetailQuery, StudentListQuery,
    StudentListResponse, StudentResponse, UpdateStudentRequest,
};
use crate::state::AppState;

/// Resolve the requesting user's local id and org_id.
async fn resolve_user_and_org(
    state: &AppState,
    current_user: &CurrentUser,
) -> Result<(Uuid, Uuid), AppError> {
    let user = state
        .user_service
        .find_by_workos_id(&current_user.workos_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    let org_id = user
        .org_id
        .ok_or_else(|| AppError::BadRequest("User is not part of an organization".into()))?;
    Ok((user.id, org_id))
}

/// Same as [`resolve_user_and_org`] but additionally requires `role = 'admin'`.
/// All write endpoints in this module gate on this. Non-admin staff (teachers,
/// office) can still call read endpoints; tightening this further requires
/// proper school-staff roles, which don't exist yet.
async fn resolve_admin_and_org(
    state: &AppState,
    current_user: &CurrentUser,
) -> Result<(Uuid, Uuid), AppError> {
    let user = state
        .user_service
        .find_by_workos_id(&current_user.workos_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    if user.role != "admin" {
        return Err(AppError::Forbidden(
            "Only admins can modify student records".into(),
        ));
    }
    let org_id = user
        .org_id
        .ok_or_else(|| AppError::BadRequest("User is not part of an organization".into()))?;
    Ok((user.id, org_id))
}

/// List students with filters, pagination, and whole-school summary.
#[utoipa::path(
    get,
    path = "/api/v1/students",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    params(
        ("search" = Option<String>, Query, description = "Match name, admission_number, or guardian name/phone/email"),
        ("grade_level" = Option<String>, Query, description = "Exact grade level"),
        ("section" = Option<String>, Query, description = "Exact section"),
        ("status" = Option<String>, Query, description = "Default 'active'. Use 'all' to disable the filter"),
        ("gender" = Option<String>, Query, description = "male | female"),
        ("boarding_status" = Option<String>, Query, description = "day | boarding | weekly_boarding"),
        ("page" = Option<i64>, Query, description = "1-indexed page (default 1)"),
        ("page_size" = Option<i64>, Query, description = "Default 25, max 100"),
        ("sort" = Option<String>, Query, description = "last_name | first_name | admission_number | enrollment_date | created_at"),
        ("order" = Option<String>, Query, description = "asc | desc"),
    ),
    responses(
        (status = 200, description = "Page of students with summary", body = StudentListResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    )
)]
pub async fn list_students(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Query(q): Query<StudentListQuery>,
) -> Result<Json<StudentListResponse>, AppError> {
    let (_user_id, org_id) = resolve_user_and_org(&state, &current_user).await?;
    let response = state.students_service.list(org_id, q).await?;
    Ok(Json(response))
}

/// Create one student. Auto-generates admission_number if omitted.
#[utoipa::path(
    post,
    path = "/api/v1/students",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    request_body = CreateStudentRequest,
    responses(
        (status = 201, description = "Student created", body = StudentResponse),
        (status = 400, description = "Invalid grade_level / gender / boarding_status", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Forbidden — requires admin", body = ErrorResponse),
        (status = 409, description = "admission_number already exists for this school", body = ErrorResponse),
    )
)]
pub async fn create_student(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Json(req): Json<CreateStudentRequest>,
) -> Result<(StatusCode, Json<StudentResponse>), AppError> {
    let (_user_id, org_id) = resolve_admin_and_org(&state, &current_user).await?;
    let response = state.students_service.create(org_id, req).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Get a student by id. Use `?include=recent_payments,recent_attendance` for related data.
#[utoipa::path(
    get,
    path = "/api/v1/students/{id}",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    params(
        ("id" = uuid::Uuid, Path, description = "Student id"),
        ("include" = Option<String>, Query, description = "Comma-separated includes: recent_payments, recent_attendance"),
    ),
    responses(
        (status = 200, description = "Student detail", body = StudentResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Student not found in this school", body = ErrorResponse),
    )
)]
pub async fn get_student(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<StudentDetailQuery>,
) -> Result<Json<StudentResponse>, AppError> {
    let (_user_id, org_id) = resolve_user_and_org(&state, &current_user).await?;
    let include = q.include.unwrap_or_default();
    let response = state.students_service.get(org_id, id, &include).await?;
    Ok(Json(response))
}

/// Update student fields. NOT used for status, class, or admission_number — see dedicated endpoints.
/// Sending `guardians` replaces the full guardian set for the student.
#[utoipa::path(
    patch,
    path = "/api/v1/students/{id}",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    params(("id" = uuid::Uuid, Path, description = "Student id")),
    request_body = UpdateStudentRequest,
    responses(
        (status = 200, description = "Updated student", body = StudentResponse),
        (status = 400, description = "Invalid field value", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Forbidden — requires admin", body = ErrorResponse),
        (status = 404, description = "Student not found in this school", body = ErrorResponse),
    )
)]
pub async fn patch_student(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateStudentRequest>,
) -> Result<Json<StudentResponse>, AppError> {
    let (_user_id, org_id) = resolve_admin_and_org(&state, &current_user).await?;
    let response = state.students_service.patch(org_id, id, req).await?;
    Ok(Json(response))
}

/// Soft-delete: marks the student as `withdrawn` and writes a status_history row.
#[utoipa::path(
    delete,
    path = "/api/v1/students/{id}",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    params(("id" = uuid::Uuid, Path, description = "Student id")),
    responses(
        (status = 204, description = "Student soft-deleted (idempotent)"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Forbidden — requires admin", body = ErrorResponse),
        (status = 404, description = "Student not found in this school", body = ErrorResponse),
    )
)]
pub async fn delete_student(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let (user_id, org_id) = resolve_admin_and_org(&state, &current_user).await?;
    state.students_service.soft_delete(org_id, id, Some(user_id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Change a student's enrollment status with reason. Records audit history.
#[utoipa::path(
    patch,
    path = "/api/v1/students/{id}/status",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    params(("id" = uuid::Uuid, Path, description = "Student id")),
    request_body = ChangeStatusRequest,
    responses(
        (status = 200, description = "Status changed", body = StatusChangeResponse),
        (status = 400, description = "Invalid status or unchanged", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Forbidden — requires admin", body = ErrorResponse),
        (status = 404, description = "Student not found in this school", body = ErrorResponse),
    )
)]
pub async fn change_status(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeStatusRequest>,
) -> Result<Json<StatusChangeResponse>, AppError> {
    let (user_id, org_id) = resolve_admin_and_org(&state, &current_user).await?;
    let response = state
        .students_service
        .change_status(org_id, id, req, Some(user_id))
        .await?;
    Ok(Json(response))
}

/// Change a student's grade_level / section. Records audit history.
#[utoipa::path(
    patch,
    path = "/api/v1/students/{id}/class",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    params(("id" = uuid::Uuid, Path, description = "Student id")),
    request_body = ChangeClassRequest,
    responses(
        (status = 200, description = "Class changed", body = StudentResponse),
        (status = 400, description = "Invalid grade_level for this school", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Forbidden — requires admin", body = ErrorResponse),
        (status = 404, description = "Student not found in this school", body = ErrorResponse),
    )
)]
pub async fn change_class(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeClassRequest>,
) -> Result<Json<StudentResponse>, AppError> {
    let (user_id, org_id) = resolve_admin_and_org(&state, &current_user).await?;
    let response = state
        .students_service
        .change_class(org_id, id, req, Some(user_id))
        .await?;
    Ok(Json(response))
}

/// Bulk promote / retain / graduate students. All decisions in one transaction;
/// each writes a `student_class_history` row sharing a `promotion_batch_id`.
#[utoipa::path(
    post,
    path = "/api/v1/students/promote",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    request_body = PromoteRequest,
    responses(
        (status = 200, description = "Promotion summary", body = PromoteSummary),
        (status = 400, description = "Invalid action / missing to_grade for promote / duplicate student_id", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Forbidden — requires admin", body = ErrorResponse),
        (status = 404, description = "One or more student_ids not found", body = ErrorResponse),
    )
)]
pub async fn promote(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Json(req): Json<PromoteRequest>,
) -> Result<Json<PromoteSummary>, AppError> {
    let (user_id, org_id) = resolve_admin_and_org(&state, &current_user).await?;
    let response = state.students_service.promote(org_id, req, Some(user_id)).await?;
    Ok(Json(response))
}

/// Bulk import students from a CSV file. Multipart form fields:
/// - `file` (required): CSV bytes (up to 5000 rows)
/// - `mapping` (required): JSON object mapping CSV header → field key. Guardian fields use `guardian1_first_name` etc.
/// - `skip_invalid` (optional): "true" to import valid rows even with errors; otherwise 422 on any error.
///
/// `guardian1_*` is treated as the primary guardian. Returns 422 on validation errors when
/// `skip_invalid != true` (no rows inserted).
#[utoipa::path(
    post,
    path = "/api/v1/students/bulk-import",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Imported (with optional row errors)", body = BulkImportResponse),
        (status = 400, description = "Missing file or mapping / invalid mapping target", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Forbidden — requires admin", body = ErrorResponse),
        (status = 422, description = "Validation errors and skip_invalid=false", body = BulkImportResponse),
    )
)]
pub async fn bulk_import(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<BulkImportResponse>), AppError> {
    let (user_id, org_id) = resolve_admin_and_org(&state, &current_user).await?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mapping: Option<HashMap<String, String>> = None;
    let mut skip_invalid = false;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("failed to read file: {e}")))?;
                file_bytes = Some(bytes.to_vec());
            }
            "mapping" => {
                let s = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("failed to read mapping: {e}")))?;
                mapping = Some(
                    serde_json::from_str(&s)
                        .map_err(|e| AppError::BadRequest(format!("invalid mapping JSON: {e}")))?,
                );
            }
            "skip_invalid" => {
                let s = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("read skip_invalid: {e}")))?;
                skip_invalid = matches!(s.as_str(), "true" | "1");
            }
            _ => {}
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| AppError::BadRequest("missing 'file' field".into()))?;
    let mapping = mapping.ok_or_else(|| AppError::BadRequest("missing 'mapping' field".into()))?;

    let (response, ok) = state
        .students_service
        .bulk_import(org_id, &file_bytes, mapping, skip_invalid, Some(user_id))
        .await?;

    let status = if ok { StatusCode::OK } else { StatusCode::UNPROCESSABLE_ENTITY };
    Ok((status, Json(response)))
}

/// Export filtered student list as CSV. Same query parameters as `GET /api/v1/students`.
#[utoipa::path(
    get,
    path = "/api/v1/students/export",
    tag = "Students",
    security(("session_cookie" = []), ("bearer_token" = [])),
    responses(
        (status = 200, description = "CSV file (text/csv)", content_type = "text/csv"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    )
)]
pub async fn export(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Query(q): Query<StudentListQuery>,
) -> Result<Response, AppError> {
    let (_user_id, org_id) = resolve_user_and_org(&state, &current_user).await?;
    let bytes = state.students_service.export_csv(org_id, q).await?;
    let date = chrono::Utc::now().format("%Y-%m-%d");
    let filename = format!("students_{date}.csv");
    let disposition =
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|e| AppError::Internal(format!("invalid disposition header: {e}")))?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, disposition)
        // Sensitive PII; tell browsers and intermediaries not to cache.
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(format!("response build: {e}")))?;
    Ok(response)
}
