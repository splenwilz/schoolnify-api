use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ── DB Row Models ──────────────────────────────────────────────────────

/// Single config row from `school_configs` table (one per org).
#[derive(Debug, Clone, FromRow)]
pub struct SchoolConfigRow {
    pub id: Uuid,
    pub org_id: Uuid,
    // Identity
    pub school_type: Option<String>,
    pub ownership_type: Option<String>,
    pub motto: Option<String>,
    pub founded_year: Option<String>,
    pub accreditation_number: Option<String>,
    pub logo_url: Option<String>,
    // Branding
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    // Location
    pub country: Option<String>,
    pub state_region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
    // Localization
    pub currency: Option<String>,
    pub date_format: Option<String>,
    pub language: Option<String>,
    // Academic Calendar
    pub calendar_type: Option<String>,
    pub current_academic_year: Option<String>,
    // Grade Levels scalars
    pub grade_level_structure_id: Option<String>,
    pub group_sections: serde_json::Value,
    pub custom_group_levels: serde_json::Value,
    // Grading scalars
    pub grading_preset_id: Option<String>,
    pub ca_weight: Option<String>,
    pub exam_weight: Option<String>,
    pub passmark: Option<String>,
    pub gpa_enabled: Option<bool>,
    pub assignment_weight: Option<String>,
    pub test_weight: Option<String>,
    pub project_weight: Option<String>,
    // Subjects scalar
    pub subject_departments: serde_json::Value,
    // Fees scalars
    pub fee_payment_schedule: Option<String>,
    pub fee_payment_due_day: Option<String>,
    pub late_fee_percentage: Option<String>,
    pub late_fee_grace_days: Option<String>,
    // Report Card
    pub report_template: Option<String>,
    pub show_assessment_breakdown: Option<bool>,
    pub show_class_average: Option<bool>,
    pub show_highest_lowest: Option<bool>,
    pub show_grading_legend: Option<bool>,
    pub show_position: Option<bool>,
    pub show_gpa: Option<bool>,
    pub show_effort_grades: Option<bool>,
    pub show_behavior_rating: Option<bool>,
    pub show_psychomotor: Option<bool>,
    pub psychomotor_traits: serde_json::Value,
    pub show_affective: Option<bool>,
    pub affective_traits: serde_json::Value,
    pub show_teacher_comments: Option<bool>,
    pub show_class_teacher_comment: Option<bool>,
    pub show_principal_signature: Option<bool>,
    pub show_subject_teacher_signature: Option<bool>,
    pub comment_char_limit: Option<String>,
    pub show_attendance_summary: Option<bool>,
    pub show_next_term_dates: Option<bool>,
    pub show_co_curricular: Option<bool>,
    // Policies
    pub attendance_tracking_methods: serde_json::Value,
    pub late_grace_period: Option<String>,
    pub attendance_threshold: Option<String>,
    pub tardies_to_absence: Option<String>,
    pub consecutive_absence_alert: Option<String>,
    pub absence_categories: serde_json::Value,
    pub promotion_criteria: Option<String>,
    pub promotion_rules: serde_json::Value,
    pub discipline_framework: Option<String>,
    pub offense_categories: serde_json::Value,
    pub consequence_ladder: serde_json::Value,
    pub point_reset_period: Option<String>,
    pub parent_portal: Option<bool>,
    pub report_comments: Option<bool>,
    pub attendance_alerts: Option<bool>,
    pub fee_reminders: Option<bool>,
    pub exam_result_notify: Option<bool>,
    pub behavior_alerts: Option<bool>,
    pub homework_alerts: Option<bool>,
    pub notification_channels: serde_json::Value,
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct GradingScaleRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub grade: String,
    pub min_score: String,
    pub max_score: String,
    pub descriptor: Option<String>,
    pub gpa_points: Option<String>,
    pub position: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct TermRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub position: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct SubjectRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub department: Option<String>,
    pub position: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct GradeLevelRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub group_name: Option<String>,
    pub position: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct FeeCategoryRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub mandatory: bool,
    pub frequency: Option<String>,
    pub fee_type: Option<String>,
    pub applies_to: Option<String>,
    pub grade_levels: serde_json::Value,
    pub amounts: serde_json::Value,
    pub position: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct FeeDiscountRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub percentage: Option<String>,
    pub applies_to: Option<String>,
    pub position: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct ScheduleGroupRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub group_name: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub period_duration: Option<String>,
    pub position: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct SchedulePeriodRow {
    pub id: Uuid,
    pub group_id: Uuid,
    pub label: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub is_break: bool,
    pub position: i16,
}

// ── Aggregate (internal, not API) ──────────────────────────────────────

/// All relational data for one org, assembled from multiple tables.
pub struct SchoolSetupData {
    pub config: Option<SchoolConfigRow>,
    pub grading_scales: Vec<GradingScaleRow>,
    pub terms: Vec<TermRow>,
    pub subjects: Vec<SubjectRow>,
    pub grade_levels: Vec<GradeLevelRow>,
    pub fee_categories: Vec<FeeCategoryRow>,
    pub fee_discounts: Vec<FeeDiscountRow>,
    pub schedule_groups: Vec<ScheduleGroupRow>,
    pub schedule_periods: Vec<SchedulePeriodRow>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl SchoolSetupData {
    pub fn empty() -> Self {
        Self {
            config: None,
            grading_scales: vec![],
            terms: vec![],
            subjects: vec![],
            grade_levels: vec![],
            fee_categories: vec![],
            fee_discounts: vec![],
            schedule_groups: vec![],
            schedule_periods: vec![],
            updated_at: None,
        }
    }
}

// ── API Response Types (unchanged) ─────────────────────────────────────

/// Response returned by GET /api/v1/schools/setup.
#[derive(Debug, Serialize, ToSchema)]
pub struct SchoolSetupResponse {
    /// The setup data, or null if no setup has been saved yet.
    pub data: Option<serde_json::Value>,
    /// Section completion metadata.
    pub completion: SetupCompletion,
    /// When the setup was last saved. Null if never saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Completion status across all setup sections.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetupCompletion {
    pub total_sections: u8,
    pub completed_sections: u8,
    pub sections: Vec<SectionStatus>,
}

/// Completion status for a single setup section.
#[derive(Debug, Serialize, ToSchema)]
pub struct SectionStatus {
    pub name: String,
    pub complete: bool,
    pub required_fields: Vec<String>,
    pub missing_fields: Vec<String>,
}

/// Public branding info returned by GET /api/v1/schools/{slug}/public.
#[derive(Debug, Serialize, ToSchema)]
pub struct PublicBrandingResponse {
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<String>,
}
