use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::school_setup::{
    FeeCategoryRow, FeeDiscountRow, GradeLevelRow, GradingScaleRow, PublicBrandingResponse,
    ScheduleGroupRow, SchedulePeriodRow, SchoolConfigRow, SchoolSetupData, SectionStatus,
    SetupCompletion, SubjectRow, TermRow,
};

pub struct SchoolSetupService {
    pool: PgPool,
}

// ── Public API ────────────────────────────────────────────────────────────

impl SchoolSetupService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Fetch full school setup data for an organization (config row + 8 child tables).
    pub async fn get_by_org_id(
        &self,
        org_id: Uuid,
    ) -> Result<Option<SchoolSetupData>, AppError> {
        let (config, scales, terms, subjects, levels, fee_cats, fee_discs, groups, periods) =
            tokio::try_join!(
                fetch_config(&self.pool, org_id),
                fetch_grading_scales(&self.pool, org_id),
                fetch_terms(&self.pool, org_id),
                fetch_subjects(&self.pool, org_id),
                fetch_grade_levels(&self.pool, org_id),
                fetch_fee_categories(&self.pool, org_id),
                fetch_fee_discounts(&self.pool, org_id),
                fetch_schedule_groups(&self.pool, org_id),
                fetch_schedule_periods(&self.pool, org_id),
            )?;

        if config.is_none()
            && scales.is_empty()
            && terms.is_empty()
            && subjects.is_empty()
            && levels.is_empty()
            && fee_cats.is_empty()
            && fee_discs.is_empty()
            && groups.is_empty()
            && periods.is_empty()
        {
            return Ok(None);
        }

        let updated_at = config.as_ref().map(|c| c.updated_at);
        Ok(Some(SchoolSetupData {
            config,
            grading_scales: scales,
            terms,
            subjects,
            grade_levels: levels,
            fee_categories: fee_cats,
            fee_discounts: fee_discs,
            schedule_groups: groups,
            schedule_periods: periods,
            updated_at,
        }))
    }

    /// Insert or update setup data from a partial JSON payload.
    /// Each top-level key dispatches to the corresponding section handler.
    pub async fn upsert_merge(
        &self,
        org_id: Uuid,
        partial_data: &serde_json::Value,
    ) -> Result<SchoolSetupData, AppError> {
        let mut tx = self.pool.begin().await?;
        Self::ensure_config_row(&mut tx, org_id).await?;

        let obj = partial_data.as_object().ok_or_else(|| {
            AppError::BadRequest("Payload must be a JSON object".into())
        })?;

        for key in obj.keys() {
            let section = &obj[key];
            match key.as_str() {
                "identity" => upsert_identity(&mut tx, org_id, section).await?,
                "branding" => upsert_branding(&mut tx, org_id, section).await?,
                "location" => upsert_location(&mut tx, org_id, section).await?,
                "localization" => upsert_localization(&mut tx, org_id, section).await?,
                "academic_calendar" => {
                    upsert_academic_calendar(&mut tx, org_id, section).await?
                }
                "grade_levels" => upsert_grade_levels(&mut tx, org_id, section).await?,
                "grading" => upsert_grading(&mut tx, org_id, section).await?,
                "subjects" => upsert_subjects(&mut tx, org_id, section).await?,
                "fees" => upsert_fees(&mut tx, org_id, section).await?,
                "schedule" => upsert_schedule(&mut tx, org_id, section).await?,
                "report_card" => upsert_report_card(&mut tx, org_id, section).await?,
                "policies" => upsert_policies(&mut tx, org_id, section).await?,
                _ => {} // ignore unknown keys
            }
        }

        tx.commit().await?;

        // Re-fetch the full data after commit
        self.get_by_org_id(org_id)
            .await?
            .ok_or_else(|| AppError::Internal("Setup missing after upsert".into()))
    }

    /// Get public branding for a school by its URL slug.
    pub async fn get_public_branding(
        &self,
        slug: &str,
    ) -> Result<Option<PublicBrandingResponse>, AppError> {
        let row: Option<(String, String, Option<String>, Option<String>, Option<String>, Option<String>)> =
            sqlx::query_as(
                r#"
                SELECT o.name, o.slug,
                       c.logo_url, c.motto, c.primary_color, c.secondary_color
                FROM organizations o
                LEFT JOIN school_configs c ON c.org_id = o.id
                WHERE o.slug = $1 AND o.is_active = TRUE
                "#,
            )
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            None => Ok(None),
            Some((name, slug, logo_url, motto, primary_color, secondary_color)) => {
                Ok(Some(PublicBrandingResponse {
                    name,
                    slug,
                    logo_url,
                    motto,
                    primary_color,
                    secondary_color,
                }))
            }
        }
    }

    /// Compute section-level completion from typed struct fields.
    /// Pure function -- no DB access.
    pub fn compute_completion(data: &SchoolSetupData) -> SetupCompletion {
        let sections = vec![
            check_identity(data),
            check_branding(data),
            check_location(data),
            check_localization(data),
            check_academic_calendar(data),
            check_grade_levels(data),
            check_grading(data),
            check_schedule(data),
            check_subjects(data),
            check_fees(data),
            check_report_card(data),
            check_policies(data),
        ];

        let completed = sections.iter().filter(|s| s.complete).count() as u8;
        SetupCompletion {
            total_sections: sections.len() as u8,
            completed_sections: completed,
            sections,
        }
    }

    /// Ensure a config row exists for the given org.
    async fn ensure_config_row(
        tx: &mut sqlx::PgConnection,
        org_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO school_configs (org_id) VALUES ($1) ON CONFLICT (org_id) DO NOTHING",
        )
        .bind(org_id)
        .execute(&mut *tx)
        .await?;
        Ok(())
    }
}

// ── Fetch helpers (used by get_by_org_id) ─────────────────────────────────

async fn fetch_config(pool: &PgPool, org_id: Uuid) -> Result<Option<SchoolConfigRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_configs WHERE org_id = $1")
        .bind(org_id)
        .fetch_optional(pool)
        .await?)
}

async fn fetch_grading_scales(pool: &PgPool, org_id: Uuid) -> Result<Vec<GradingScaleRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_grading_scales WHERE org_id = $1 ORDER BY position")
        .bind(org_id)
        .fetch_all(pool)
        .await?)
}

async fn fetch_terms(pool: &PgPool, org_id: Uuid) -> Result<Vec<TermRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_terms WHERE org_id = $1 ORDER BY position")
        .bind(org_id)
        .fetch_all(pool)
        .await?)
}

async fn fetch_subjects(pool: &PgPool, org_id: Uuid) -> Result<Vec<SubjectRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_subjects WHERE org_id = $1 ORDER BY position")
        .bind(org_id)
        .fetch_all(pool)
        .await?)
}

async fn fetch_grade_levels(pool: &PgPool, org_id: Uuid) -> Result<Vec<GradeLevelRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_grade_levels WHERE org_id = $1 ORDER BY position")
        .bind(org_id)
        .fetch_all(pool)
        .await?)
}

async fn fetch_fee_categories(pool: &PgPool, org_id: Uuid) -> Result<Vec<FeeCategoryRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_fee_categories WHERE org_id = $1 ORDER BY position")
        .bind(org_id)
        .fetch_all(pool)
        .await?)
}

async fn fetch_fee_discounts(pool: &PgPool, org_id: Uuid) -> Result<Vec<FeeDiscountRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_fee_discounts WHERE org_id = $1 ORDER BY position")
        .bind(org_id)
        .fetch_all(pool)
        .await?)
}

async fn fetch_schedule_groups(pool: &PgPool, org_id: Uuid) -> Result<Vec<ScheduleGroupRow>, AppError> {
    Ok(sqlx::query_as("SELECT * FROM school_schedule_groups WHERE org_id = $1 ORDER BY position")
        .bind(org_id)
        .fetch_all(pool)
        .await?)
}

async fn fetch_schedule_periods(pool: &PgPool, org_id: Uuid) -> Result<Vec<SchedulePeriodRow>, AppError> {
    Ok(sqlx::query_as(
        r#"SELECT p.* FROM school_schedule_periods p
           JOIN school_schedule_groups g ON g.id = p.group_id
           WHERE g.org_id = $1 ORDER BY p.position"#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?)
}

// ── Scalar section upserts ────────────────────────────────────────────────

async fn upsert_identity(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE school_configs SET
            school_type = $2, ownership_type = $3, motto = $4,
            founded_year = $5, accreditation_number = $6, logo_url = $7
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "school_type"))
    .bind(str_val(v, "ownership_type"))
    .bind(str_val(v, "motto"))
    .bind(str_val(v, "founded_year"))
    .bind(str_val(v, "accreditation_number"))
    .bind(str_val(v, "logo_url"))
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn upsert_branding(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE school_configs SET
            primary_color = $2, secondary_color = $3, logo_url = COALESCE($4, logo_url)
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "primary_color"))
    .bind(str_val(v, "secondary_color"))
    .bind(str_val(v, "logo_url"))
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn upsert_location(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE school_configs SET
            country = $2, state_region = $3, city = $4, timezone = $5
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "country"))
    .bind(str_val(v, "state_region"))
    .bind(str_val(v, "city"))
    .bind(str_val(v, "timezone"))
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn upsert_localization(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE school_configs SET
            currency = $2, date_format = $3, language = $4
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "currency"))
    .bind(str_val(v, "date_format"))
    .bind(str_val(v, "language"))
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn upsert_report_card(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE school_configs SET
            report_template = $2,
            show_assessment_breakdown = $3, show_class_average = $4,
            show_highest_lowest = $5, show_grading_legend = $6,
            show_position = $7, show_gpa = $8,
            show_effort_grades = $9, show_behavior_rating = $10,
            show_psychomotor = $11, psychomotor_traits = $12,
            show_affective = $13, affective_traits = $14,
            show_teacher_comments = $15, show_class_teacher_comment = $16,
            show_principal_signature = $17, show_subject_teacher_signature = $18,
            comment_char_limit = $19, show_attendance_summary = $20,
            show_next_term_dates = $21, show_co_curricular = $22
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "report_template"))
    .bind(bool_val(v, "show_assessment_breakdown"))
    .bind(bool_val(v, "show_class_average"))
    .bind(bool_val(v, "show_highest_lowest"))
    .bind(bool_val(v, "show_grading_legend"))
    .bind(bool_val(v, "show_position"))
    .bind(bool_val(v, "show_gpa"))
    .bind(bool_val(v, "show_effort_grades"))
    .bind(bool_val(v, "show_behavior_rating"))
    .bind(bool_val(v, "show_psychomotor"))
    .bind(json_val(v, "psychomotor_traits"))
    .bind(bool_val(v, "show_affective"))
    .bind(json_val(v, "affective_traits"))
    .bind(bool_val(v, "show_teacher_comments"))
    .bind(bool_val(v, "show_class_teacher_comment"))
    .bind(bool_val(v, "show_principal_signature"))
    .bind(bool_val(v, "show_subject_teacher_signature"))
    .bind(str_val(v, "comment_char_limit"))
    .bind(bool_val(v, "show_attendance_summary"))
    .bind(bool_val(v, "show_next_term_dates"))
    .bind(bool_val(v, "show_co_curricular"))
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn upsert_policies(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE school_configs SET
            attendance_tracking_methods = $2, late_grace_period = $3,
            attendance_threshold = $4, tardies_to_absence = $5,
            consecutive_absence_alert = $6, absence_categories = $7,
            promotion_criteria = $8, promotion_rules = $9,
            discipline_framework = $10, offense_categories = $11,
            consequence_ladder = $12, point_reset_period = $13,
            parent_portal = $14, report_comments = $15,
            attendance_alerts = $16, fee_reminders = $17,
            exam_result_notify = $18, behavior_alerts = $19,
            homework_alerts = $20, notification_channels = $21
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(json_val(v, "attendance_tracking_methods"))
    .bind(str_val(v, "late_grace_period"))
    .bind(str_val(v, "attendance_threshold"))
    .bind(str_val(v, "tardies_to_absence"))
    .bind(str_val(v, "consecutive_absence_alert"))
    .bind(json_val(v, "absence_categories"))
    .bind(str_val(v, "promotion_criteria"))
    .bind(json_val(v, "promotion_rules"))
    .bind(str_val(v, "discipline_framework"))
    .bind(json_val(v, "offense_categories"))
    .bind(json_val(v, "consequence_ladder"))
    .bind(str_val(v, "point_reset_period"))
    .bind(bool_val(v, "parent_portal"))
    .bind(bool_val(v, "report_comments"))
    .bind(bool_val(v, "attendance_alerts"))
    .bind(bool_val(v, "fee_reminders"))
    .bind(bool_val(v, "exam_result_notify"))
    .bind(bool_val(v, "behavior_alerts"))
    .bind(bool_val(v, "homework_alerts"))
    .bind(json_val(v, "notification_channels"))
    .execute(&mut *tx)
    .await?;
    Ok(())
}

// ── Mixed section upserts (scalars + child rows) ──────────────────────────

async fn upsert_academic_calendar(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    // Scalar columns
    sqlx::query(
        r#"UPDATE school_configs SET
            calendar_type = $2, current_academic_year = $3
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "calendar_type"))
    .bind(str_val(v, "current_academic_year"))
    .execute(&mut *tx)
    .await?;

    // Child rows: terms
    if let Some(terms) = v.get("terms").and_then(|t| t.as_array()) {
        sqlx::query("DELETE FROM school_terms WHERE org_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        for (i, t) in terms.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO school_terms (org_id, name, start_date, end_date, position)
                   VALUES ($1, $2, $3, $4, $5)"#,
            )
            .bind(org_id)
            .bind(str_val(t, "name").unwrap_or_default())
            .bind(str_val(t, "start_date"))
            .bind(str_val(t, "end_date"))
            .bind(i as i16)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

async fn upsert_grade_levels(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    // Scalar columns
    sqlx::query(
        r#"UPDATE school_configs SET
            grade_level_structure_id = $2, group_sections = $3, custom_group_levels = $4
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "grade_level_structure_id"))
    .bind(json_val(v, "group_sections"))
    .bind(json_val(v, "custom_group_levels"))
    .execute(&mut *tx)
    .await?;

    // Child rows: grade_levels (can be string array or object array)
    if let Some(levels) = v.get("grade_levels").and_then(|l| l.as_array()) {
        sqlx::query("DELETE FROM school_grade_levels WHERE org_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        for (i, lv) in levels.iter().enumerate() {
            let (name, group_name) = if let Some(s) = lv.as_str() {
                (s.to_string(), None)
            } else {
                (
                    str_val(lv, "name").unwrap_or_default(),
                    str_val(lv, "group_name"),
                )
            };
            sqlx::query(
                r#"INSERT INTO school_grade_levels (org_id, name, group_name, position)
                   VALUES ($1, $2, $3, $4)"#,
            )
            .bind(org_id)
            .bind(name)
            .bind(group_name)
            .bind(i as i16)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

async fn upsert_grading(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    // Scalar columns
    sqlx::query(
        r#"UPDATE school_configs SET
            grading_preset_id = $2, ca_weight = $3, exam_weight = $4,
            passmark = $5, gpa_enabled = $6,
            assignment_weight = $7, test_weight = $8, project_weight = $9
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "grading_preset_id"))
    .bind(str_val(v, "ca_weight"))
    .bind(str_val(v, "exam_weight"))
    .bind(str_val(v, "passmark"))
    .bind(bool_val(v, "gpa_enabled"))
    .bind(str_val(v, "assignment_weight"))
    .bind(str_val(v, "test_weight"))
    .bind(str_val(v, "project_weight"))
    .execute(&mut *tx)
    .await?;

    // Child rows: grading_scale
    if let Some(scales) = v.get("grading_scale").and_then(|s| s.as_array()) {
        sqlx::query("DELETE FROM school_grading_scales WHERE org_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        for (i, s) in scales.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO school_grading_scales
                   (org_id, grade, min_score, max_score, descriptor, gpa_points, position)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            )
            .bind(org_id)
            .bind(str_val(s, "grade").unwrap_or_default())
            .bind(str_val(s, "min_score").unwrap_or_else(|| "0".into()))
            .bind(str_val(s, "max_score").unwrap_or_else(|| "0".into()))
            .bind(str_val(s, "descriptor"))
            .bind(str_val(s, "gpa_points"))
            .bind(i as i16)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

async fn upsert_subjects(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    // Scalar columns
    sqlx::query(
        "UPDATE school_configs SET subject_departments = $2 WHERE org_id = $1",
    )
    .bind(org_id)
    .bind(json_val(v, "subject_departments"))
    .execute(&mut *tx)
    .await?;

    // Child rows: subjects (string array or object array)
    if let Some(subjects) = v.get("subjects").and_then(|s| s.as_array()) {
        sqlx::query("DELETE FROM school_subjects WHERE org_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        for (i, subj) in subjects.iter().enumerate() {
            let (name, dept) = if let Some(s) = subj.as_str() {
                (s.to_string(), None)
            } else {
                (
                    str_val(subj, "name").unwrap_or_default(),
                    str_val(subj, "department"),
                )
            };
            sqlx::query(
                r#"INSERT INTO school_subjects (org_id, name, department, position)
                   VALUES ($1, $2, $3, $4)"#,
            )
            .bind(org_id)
            .bind(name)
            .bind(dept)
            .bind(i as i16)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

async fn upsert_fees(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    // Scalar columns
    sqlx::query(
        r#"UPDATE school_configs SET
            fee_payment_schedule = $2, fee_payment_due_day = $3,
            late_fee_percentage = $4, late_fee_grace_days = $5
           WHERE org_id = $1"#,
    )
    .bind(org_id)
    .bind(str_val(v, "fee_payment_schedule"))
    .bind(str_val(v, "fee_payment_due_day"))
    .bind(str_val(v, "late_fee_percentage"))
    .bind(str_val(v, "late_fee_grace_days"))
    .execute(&mut *tx)
    .await?;

    // Child rows: fee_categories
    if let Some(cats) = v.get("fee_categories").and_then(|c| c.as_array()) {
        sqlx::query("DELETE FROM school_fee_categories WHERE org_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        for (i, c) in cats.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO school_fee_categories
                   (org_id, name, mandatory, frequency, fee_type, applies_to, grade_levels, amounts, position)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
            )
            .bind(org_id)
            .bind(str_val(c, "name").unwrap_or_default())
            .bind(bool_val(c, "mandatory").unwrap_or(false))
            .bind(str_val(c, "frequency"))
            .bind(str_val(c, "fee_type"))
            .bind(str_val(c, "applies_to"))
            .bind(json_val_or(c, "grade_levels", serde_json::json!([])))
            .bind(json_val_or(c, "amounts", serde_json::json!({})))
            .bind(i as i16)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Child rows: discount_types
    if let Some(discs) = v.get("discount_types").and_then(|d| d.as_array()) {
        sqlx::query("DELETE FROM school_fee_discounts WHERE org_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        for (i, d) in discs.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO school_fee_discounts
                   (org_id, name, percentage, applies_to, position)
                   VALUES ($1, $2, $3, $4, $5)"#,
            )
            .bind(org_id)
            .bind(str_val(d, "name").unwrap_or_default())
            .bind(str_val(d, "percentage"))
            .bind(str_val(d, "applies_to"))
            .bind(i as i16)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

async fn upsert_schedule(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    v: &serde_json::Value,
) -> Result<(), AppError> {
    // Delete existing groups (periods cascade via FK)
    sqlx::query("DELETE FROM school_schedule_groups WHERE org_id = $1")
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

    // Schedules can be an object (keyed by group name) or an array of group objects
    if let Some(obj) = v.get("schedules").and_then(|s| s.as_object()) {
        let mut pos: i16 = 0;
        for (group_name, group_val) in obj {
            let group_id = insert_schedule_group(
                &mut *tx, org_id, group_name, group_val, pos,
            )
            .await?;
            insert_schedule_periods(&mut *tx, group_id, group_val).await?;
            pos += 1;
        }
    } else if let Some(arr) = v.get("schedules").and_then(|s| s.as_array()) {
        for (i, group_val) in arr.iter().enumerate() {
            let name = str_val(group_val, "group_name").unwrap_or_default();
            let group_id =
                insert_schedule_group(&mut *tx, org_id, &name, group_val, i as i16).await?;
            insert_schedule_periods(&mut *tx, group_id, group_val).await?;
        }
    }
    Ok(())
}

async fn insert_schedule_group(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    group_name: &str,
    v: &serde_json::Value,
    position: i16,
) -> Result<Uuid, AppError> {
    let row: (Uuid,) = sqlx::query_as(
        r#"INSERT INTO school_schedule_groups
           (org_id, group_name, start_time, end_time, period_duration, position)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(group_name)
    .bind(str_val(v, "start_time"))
    .bind(str_val(v, "end_time"))
    .bind(str_val(v, "period_duration"))
    .bind(position)
    .fetch_one(&mut *tx)
    .await?;
    Ok(row.0)
}

async fn insert_schedule_periods(
    tx: &mut sqlx::PgConnection,
    group_id: Uuid,
    group_val: &serde_json::Value,
) -> Result<(), AppError> {
    if let Some(periods) = group_val.get("periods").and_then(|p| p.as_array()) {
        for (i, p) in periods.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO school_schedule_periods
                   (group_id, label, start_time, end_time, is_break, position)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(group_id)
            .bind(str_val(p, "label").unwrap_or_default())
            .bind(str_val(p, "start_time"))
            .bind(str_val(p, "end_time"))
            .bind(bool_val(p, "is_break").unwrap_or(false))
            .bind(i as i16)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

// ── JSON extraction helpers ───────────────────────────────────────────────

fn str_val(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn bool_val(v: &serde_json::Value, key: &str) -> Option<bool> {
    v.get(key).and_then(|v| v.as_bool())
}

fn json_val(v: &serde_json::Value, key: &str) -> serde_json::Value {
    v.get(key).cloned().unwrap_or(serde_json::Value::Null)
}

fn json_val_or(v: &serde_json::Value, key: &str, default: serde_json::Value) -> serde_json::Value {
    v.get(key).cloned().unwrap_or(default)
}

// ── Section completion checks ─────────────────────────────────────────────

fn str_filled(opt: &Option<String>) -> bool {
    opt.as_ref().is_some_and(|s| !s.is_empty())
}

fn make_status(name: &str, required: &[&str], missing: Vec<String>) -> SectionStatus {
    SectionStatus {
        name: name.to_string(),
        complete: missing.is_empty(),
        required_fields: required.iter().map(|k| k.to_string()).collect(),
        missing_fields: missing,
    }
}

fn check_identity(data: &SchoolSetupData) -> SectionStatus {
    let required = &["school_type", "motto"];
    let mut missing = Vec::new();
    if let Some(c) = &data.config {
        if !str_filled(&c.school_type) { missing.push("school_type".into()); }
        if !str_filled(&c.motto) { missing.push("motto".into()); }
    } else {
        missing = required.iter().map(|k| k.to_string()).collect();
    }
    make_status("identity", required, missing)
}

fn check_branding(data: &SchoolSetupData) -> SectionStatus {
    let required = &["primary_color", "secondary_color"];
    let mut missing = Vec::new();
    if let Some(c) = &data.config {
        if !str_filled(&c.primary_color) { missing.push("primary_color".into()); }
        if !str_filled(&c.secondary_color) { missing.push("secondary_color".into()); }
    } else {
        missing = required.iter().map(|k| k.to_string()).collect();
    }
    make_status("branding", required, missing)
}

fn check_location(data: &SchoolSetupData) -> SectionStatus {
    let required = &["country", "timezone"];
    let mut missing = Vec::new();
    if let Some(c) = &data.config {
        if !str_filled(&c.country) { missing.push("country".into()); }
        if !str_filled(&c.timezone) { missing.push("timezone".into()); }
    } else {
        missing = required.iter().map(|k| k.to_string()).collect();
    }
    make_status("location", required, missing)
}

fn check_localization(data: &SchoolSetupData) -> SectionStatus {
    let required = &["currency", "date_format", "language"];
    let mut missing = Vec::new();
    if let Some(c) = &data.config {
        if !str_filled(&c.currency) { missing.push("currency".into()); }
        if !str_filled(&c.date_format) { missing.push("date_format".into()); }
        if !str_filled(&c.language) { missing.push("language".into()); }
    } else {
        missing = required.iter().map(|k| k.to_string()).collect();
    }
    make_status("localization", required, missing)
}

fn check_academic_calendar(data: &SchoolSetupData) -> SectionStatus {
    let required = &["calendar_type", "current_academic_year"];
    let mut missing = Vec::new();
    if let Some(c) = &data.config {
        if !str_filled(&c.calendar_type) { missing.push("calendar_type".into()); }
        if !str_filled(&c.current_academic_year) { missing.push("current_academic_year".into()); }
    } else {
        missing = required.iter().map(|k| k.to_string()).collect();
    }
    make_status("academic_calendar", required, missing)
}

fn check_grade_levels(data: &SchoolSetupData) -> SectionStatus {
    let required = &["grade_levels"];
    let missing = if data.grade_levels.is_empty() {
        vec!["grade_levels".into()]
    } else {
        vec![]
    };
    make_status("grade_levels", required, missing)
}

fn check_grading(data: &SchoolSetupData) -> SectionStatus {
    let required = &["grading_scale", "ca_weight", "exam_weight", "passmark"];
    let mut missing = Vec::new();
    if data.grading_scales.is_empty() {
        missing.push("grading_scale".into());
    }
    if let Some(c) = &data.config {
        if !str_filled(&c.ca_weight) { missing.push("ca_weight".into()); }
        if !str_filled(&c.exam_weight) { missing.push("exam_weight".into()); }
        if !str_filled(&c.passmark) { missing.push("passmark".into()); }
    } else {
        missing.push("ca_weight".into());
        missing.push("exam_weight".into());
        missing.push("passmark".into());
    }
    make_status("grading", required, missing)
}

fn check_schedule(data: &SchoolSetupData) -> SectionStatus {
    let required = &["schedules"];
    let missing = if data.schedule_groups.is_empty() || data.schedule_periods.is_empty() {
        vec!["schedules".into()]
    } else {
        vec![]
    };
    make_status("schedule", required, missing)
}

fn check_subjects(data: &SchoolSetupData) -> SectionStatus {
    let required = &["subjects"];
    let missing = if data.subjects.is_empty() {
        vec!["subjects".into()]
    } else {
        vec![]
    };
    make_status("subjects", required, missing)
}

fn check_fees(data: &SchoolSetupData) -> SectionStatus {
    let required = &["fee_categories"];
    let missing = if data.fee_categories.is_empty() {
        vec!["fee_categories".into()]
    } else {
        vec![]
    };
    make_status("fees", required, missing)
}

fn check_report_card(data: &SchoolSetupData) -> SectionStatus {
    let required = &["report_template"];
    let mut missing = Vec::new();
    if let Some(c) = &data.config {
        if !str_filled(&c.report_template) { missing.push("report_template".into()); }
    } else {
        missing = required.iter().map(|k| k.to_string()).collect();
    }
    make_status("report_card", required, missing)
}

fn check_policies(data: &SchoolSetupData) -> SectionStatus {
    let required = &["promotion_criteria", "discipline_framework"];
    let mut missing = Vec::new();
    if let Some(c) = &data.config {
        if !str_filled(&c.promotion_criteria) { missing.push("promotion_criteria".into()); }
        if !str_filled(&c.discipline_framework) { missing.push("discipline_framework".into()); }
    } else {
        missing = required.iter().map(|k| k.to_string()).collect();
    }
    make_status("policies", required, missing)
}

// ── Unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn empty_data() -> SchoolSetupData {
        SchoolSetupData::empty()
    }

    fn config_with(f: impl FnOnce(&mut SchoolConfigRow)) -> SchoolConfigRow {
        let mut c = SchoolConfigRow {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            school_type: None,
            ownership_type: None,
            motto: None,
            founded_year: None,
            accreditation_number: None,
            logo_url: None,
            primary_color: None,
            secondary_color: None,
            country: None,
            state_region: None,
            city: None,
            timezone: None,
            currency: None,
            date_format: None,
            language: None,
            calendar_type: None,
            current_academic_year: None,
            grade_level_structure_id: None,
            group_sections: serde_json::json!({}),
            custom_group_levels: serde_json::json!({}),
            grading_preset_id: None,
            ca_weight: None,
            exam_weight: None,
            passmark: None,
            gpa_enabled: None,
            assignment_weight: None,
            test_weight: None,
            project_weight: None,
            subject_departments: serde_json::json!({}),
            fee_payment_schedule: None,
            fee_payment_due_day: None,
            late_fee_percentage: None,
            late_fee_grace_days: None,
            report_template: None,
            show_assessment_breakdown: None,
            show_class_average: None,
            show_highest_lowest: None,
            show_grading_legend: None,
            show_position: None,
            show_gpa: None,
            show_effort_grades: None,
            show_behavior_rating: None,
            show_psychomotor: None,
            psychomotor_traits: serde_json::json!([]),
            show_affective: None,
            affective_traits: serde_json::json!([]),
            show_teacher_comments: None,
            show_class_teacher_comment: None,
            show_principal_signature: None,
            show_subject_teacher_signature: None,
            comment_char_limit: None,
            show_attendance_summary: None,
            show_next_term_dates: None,
            show_co_curricular: None,
            attendance_tracking_methods: serde_json::json!({}),
            late_grace_period: None,
            attendance_threshold: None,
            tardies_to_absence: None,
            consecutive_absence_alert: None,
            absence_categories: serde_json::json!([]),
            promotion_criteria: None,
            promotion_rules: serde_json::json!({}),
            discipline_framework: None,
            offense_categories: serde_json::json!([]),
            consequence_ladder: serde_json::json!([]),
            point_reset_period: None,
            parent_portal: None,
            report_comments: None,
            attendance_alerts: None,
            fee_reminders: None,
            exam_result_notify: None,
            behavior_alerts: None,
            homework_alerts: None,
            notification_channels: serde_json::json!([]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        f(&mut c);
        c
    }

    #[test]
    fn test_compute_completion_empty_data() {
        let data = empty_data();
        let result = SchoolSetupService::compute_completion(&data);
        assert_eq!(result.total_sections, 12);
        assert_eq!(result.completed_sections, 0);
        assert!(result.sections.iter().all(|s| !s.complete));
    }

    #[test]
    fn test_compute_completion_partial_section() {
        let mut data = empty_data();
        data.config = Some(config_with(|c| {
            c.school_type = Some("secondary".into());
        }));
        let result = SchoolSetupService::compute_completion(&data);
        let identity = result.sections.iter().find(|s| s.name == "identity").unwrap();
        assert!(!identity.complete);
        assert_eq!(identity.missing_fields, vec!["motto"]);
    }

    #[test]
    fn test_compute_completion_full_section() {
        let mut data = empty_data();
        data.config = Some(config_with(|c| {
            c.school_type = Some("secondary".into());
            c.motto = Some("Learn well".into());
        }));
        let result = SchoolSetupService::compute_completion(&data);
        let identity = result.sections.iter().find(|s| s.name == "identity").unwrap();
        assert!(identity.complete);
        assert!(identity.missing_fields.is_empty());
        assert_eq!(result.completed_sections, 1);
    }

    #[test]
    fn test_compute_completion_empty_string_not_filled() {
        let mut data = empty_data();
        data.config = Some(config_with(|c| {
            c.school_type = Some("".into());
            c.motto = Some("Learn".into());
        }));
        let result = SchoolSetupService::compute_completion(&data);
        let identity = result.sections.iter().find(|s| s.name == "identity").unwrap();
        assert!(!identity.complete);
        assert_eq!(identity.missing_fields, vec!["school_type"]);
    }

    #[test]
    fn test_policies_requires_both_fields() {
        let mut data = empty_data();
        data.config = Some(config_with(|c| {
            c.promotion_criteria = Some("automatic".into());
        }));
        let result = SchoolSetupService::compute_completion(&data);
        let policies = result.sections.iter().find(|s| s.name == "policies").unwrap();
        assert!(!policies.complete);
        assert_eq!(policies.missing_fields, vec!["discipline_framework"]);
    }

    #[test]
    fn test_policies_complete_with_both_required() {
        let mut data = empty_data();
        data.config = Some(config_with(|c| {
            c.promotion_criteria = Some("automatic".into());
            c.discipline_framework = Some("merit_demerit".into());
        }));
        let result = SchoolSetupService::compute_completion(&data);
        let policies = result.sections.iter().find(|s| s.name == "policies").unwrap();
        assert!(policies.complete);
    }

    #[test]
    fn test_grading_scales_empty_means_incomplete() {
        let mut data = empty_data();
        data.config = Some(config_with(|c| {
            c.ca_weight = Some("40".into());
            c.exam_weight = Some("60".into());
            c.passmark = Some("50".into());
        }));
        // grading_scales is empty
        let result = SchoolSetupService::compute_completion(&data);
        let grading = result.sections.iter().find(|s| s.name == "grading").unwrap();
        assert!(!grading.complete);
        assert!(grading.missing_fields.contains(&"grading_scale".to_string()));
    }

    #[test]
    fn test_child_table_grade_levels_empty_means_incomplete() {
        let data = empty_data();
        let result = SchoolSetupService::compute_completion(&data);
        let section = result.sections.iter().find(|s| s.name == "grade_levels").unwrap();
        assert!(!section.complete);
        assert_eq!(section.missing_fields, vec!["grade_levels"]);
    }

    #[test]
    fn test_child_table_grade_levels_filled_means_complete() {
        let mut data = empty_data();
        data.grade_levels.push(GradeLevelRow {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            name: "Grade 1".into(),
            group_name: None,
            position: 0,
        });
        let result = SchoolSetupService::compute_completion(&data);
        let section = result.sections.iter().find(|s| s.name == "grade_levels").unwrap();
        assert!(section.complete);
    }
}
