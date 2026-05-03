use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ── DB Row Models ──────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct StudentRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub admission_number: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: String,
    pub grade_level: String,
    pub section: Option<String>,
    pub stream: Option<String>,
    pub enrollment_date: NaiveDate,
    pub status: String,
    pub boarding_status: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub blood_group: Option<String>,
    pub genotype: Option<String>,
    pub allergies: Option<String>,
    pub medical_conditions: Option<String>,
    pub previous_school: Option<String>,
    pub state_of_origin: Option<String>,
    pub lga: Option<String>,
    pub religion: Option<String>,
    pub tribe: Option<String>,
    pub avatar_url: Option<String>,
    pub graduation_date: Option<NaiveDate>,
    pub withdrawn_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct StudentGuardianRow {
    pub id: Uuid,
    pub student_id: Uuid,
    pub org_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub relationship: Option<String>,
    pub occupation: Option<String>,
    pub is_primary: bool,
    pub position: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct StudentStatusHistoryRow {
    pub id: Uuid,
    pub student_id: Uuid,
    pub org_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub reason: Option<String>,
    pub effective_date: NaiveDate,
    pub changed_by_user_id: Option<Uuid>,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct StudentClassHistoryRow {
    pub id: Uuid,
    pub student_id: Uuid,
    pub org_id: Uuid,
    pub from_grade_level: Option<String>,
    pub from_section: Option<String>,
    pub to_grade_level: Option<String>,
    pub to_section: Option<String>,
    pub change_kind: String,
    pub reason: Option<String>,
    pub effective_date: NaiveDate,
    pub changed_by_user_id: Option<Uuid>,
    pub promotion_batch_id: Option<Uuid>,
    pub changed_at: DateTime<Utc>,
}

// ── Request DTOs ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct GuardianInput {
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub relationship: Option<String>,
    #[serde(default)]
    pub occupation: Option<String>,
    #[serde(default)]
    pub is_primary: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateStudentRequest {
    pub first_name: String,
    #[serde(default)]
    pub middle_name: Option<String>,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: String,
    pub grade_level: String,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub stream: Option<String>,
    #[serde(default)]
    pub admission_number: Option<String>,
    #[serde(default)]
    pub enrollment_date: Option<NaiveDate>,
    #[serde(default)]
    pub boarding_status: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub blood_group: Option<String>,
    #[serde(default)]
    pub genotype: Option<String>,
    #[serde(default)]
    pub allergies: Option<String>,
    #[serde(default)]
    pub medical_conditions: Option<String>,
    #[serde(default)]
    pub previous_school: Option<String>,
    #[serde(default)]
    pub state_of_origin: Option<String>,
    #[serde(default)]
    pub lga: Option<String>,
    #[serde(default)]
    pub religion: Option<String>,
    #[serde(default)]
    pub tribe: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub guardians: Vec<GuardianInput>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateStudentRequest {
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub middle_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub date_of_birth: Option<NaiveDate>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub stream: Option<String>,
    #[serde(default)]
    pub boarding_status: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub blood_group: Option<String>,
    #[serde(default)]
    pub genotype: Option<String>,
    #[serde(default)]
    pub allergies: Option<String>,
    #[serde(default)]
    pub medical_conditions: Option<String>,
    #[serde(default)]
    pub previous_school: Option<String>,
    #[serde(default)]
    pub state_of_origin: Option<String>,
    #[serde(default)]
    pub lga: Option<String>,
    #[serde(default)]
    pub religion: Option<String>,
    #[serde(default)]
    pub tribe: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    /// If provided, replaces the full guardian set for this student.
    #[serde(default)]
    pub guardians: Option<Vec<GuardianInput>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeStatusRequest {
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub effective_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeClassRequest {
    pub grade_level: String,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub stream: Option<String>,
    #[serde(default)]
    pub effective_date: Option<NaiveDate>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PromoteDecision {
    pub student_id: Uuid,
    pub action: String,
    #[serde(default)]
    pub to_grade: Option<String>,
    #[serde(default)]
    pub to_section: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PromoteRequest {
    pub decisions: Vec<PromoteDecision>,
    #[serde(default)]
    pub academic_year: Option<String>,
    #[serde(default)]
    pub effective_date: Option<NaiveDate>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct StudentListQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub grade_level: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub boarding_status: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub include: Option<String>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct StudentDetailQuery {
    #[serde(default)]
    pub include: Option<String>,
}

// ── Response DTOs ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct GuardianResponse {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occupation: Option<String>,
    pub is_primary: bool,
}

impl From<StudentGuardianRow> for GuardianResponse {
    fn from(g: StudentGuardianRow) -> Self {
        Self {
            id: g.id,
            first_name: g.first_name,
            last_name: g.last_name,
            phone: g.phone,
            email: g.email,
            relationship: g.relationship,
            occupation: g.occupation,
            is_primary: g.is_primary,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StudentResponse {
    pub id: Uuid,
    pub admission_number: String,
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: String,
    pub grade_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    pub enrollment_date: NaiveDate,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boarding_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    pub guardians: Vec<GuardianResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blood_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genotype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allergies: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medical_conditions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_school: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_of_origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lga: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub religion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tribe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Computed; null until grades module exists.
    pub gpa: Option<f64>,
    /// Computed; null until attendance module exists.
    pub attendance_rate: Option<f64>,
    /// Derived; "unknown" until fees module exists.
    pub fee_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Empty array until fees module exists. Populated when ?include=recent_payments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_payments: Option<Vec<serde_json::Value>>,
    /// Empty array until attendance module exists. Populated when ?include=recent_attendance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_attendance: Option<Vec<serde_json::Value>>,
}

impl StudentResponse {
    pub fn from_row(s: StudentRow, guardians: Vec<StudentGuardianRow>) -> Self {
        Self {
            id: s.id,
            admission_number: s.admission_number,
            first_name: s.first_name,
            middle_name: s.middle_name,
            last_name: s.last_name,
            date_of_birth: s.date_of_birth,
            gender: s.gender,
            grade_level: s.grade_level,
            section: s.section,
            stream: s.stream,
            enrollment_date: s.enrollment_date,
            status: s.status,
            boarding_status: s.boarding_status,
            phone: s.phone,
            email: s.email,
            address: s.address,
            city: s.city,
            state: s.state,
            postal_code: s.postal_code,
            guardians: guardians.into_iter().map(GuardianResponse::from).collect(),
            blood_group: s.blood_group,
            genotype: s.genotype,
            allergies: s.allergies,
            medical_conditions: s.medical_conditions,
            previous_school: s.previous_school,
            state_of_origin: s.state_of_origin,
            lga: s.lga,
            religion: s.religion,
            tribe: s.tribe,
            avatar_url: s.avatar_url,
            gpa: None,
            attendance_rate: None,
            fee_status: "unknown".into(),
            created_at: s.created_at,
            updated_at: s.updated_at,
            recent_payments: None,
            recent_attendance: None,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StudentSummary {
    pub total_students: i64,
    pub active: i64,
    pub average_gpa: Option<f64>,
    pub average_attendance: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StudentListResponse {
    pub data: Vec<StudentResponse>,
    pub pagination: PaginationInfo,
    pub summary: StudentSummary,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatusChangeRecord {
    pub id: Uuid,
    pub from_status: String,
    pub to_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub effective_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_by: Option<Uuid>,
    pub changed_at: DateTime<Utc>,
}

impl From<StudentStatusHistoryRow> for StatusChangeRecord {
    fn from(r: StudentStatusHistoryRow) -> Self {
        Self {
            id: r.id,
            from_status: r.from_status,
            to_status: r.to_status,
            reason: r.reason,
            effective_date: r.effective_date,
            changed_by: r.changed_by_user_id,
            changed_at: r.changed_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatusChangeResponse {
    pub student: StudentResponse,
    pub status_change: StatusChangeRecord,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PromoteSummary {
    pub promoted: i64,
    pub retained: i64,
    pub graduated: i64,
    pub batch_id: Uuid,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportRowError {
    pub row: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportedStudent {
    pub id: Uuid,
    pub admission_number: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkImportResponse {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<ImportRowError>,
    pub imported_students: Vec<ImportedStudent>,
}
