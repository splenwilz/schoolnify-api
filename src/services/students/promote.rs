use uuid::Uuid;

use crate::errors::AppError;
use crate::models::students::{PromoteRequest, PromoteSummary, StudentRow};

use super::crud::{today, validate_grade_level};
use super::StudentsService;

const ALLOWED_ACTIONS: &[&str] = &["promote", "retain", "graduate"];

impl StudentsService {
    /// Bulk promote / retain / graduate students. Single transaction; all-or-nothing.
    /// Each decision writes one `student_class_history` row sharing a `promotion_batch_id`.
    pub async fn promote(
        &self,
        org_id: Uuid,
        req: PromoteRequest,
        changed_by: Option<Uuid>,
    ) -> Result<PromoteSummary, AppError> {
        if req.decisions.is_empty() {
            return Err(AppError::BadRequest("decisions array is empty".into()));
        }

        // Pre-validate all action enums and target grade levels before opening the tx.
        for d in &req.decisions {
            if !ALLOWED_ACTIONS.contains(&d.action.as_str()) {
                return Err(AppError::BadRequest(format!(
                    "Invalid action '{}'; must be promote/retain/graduate",
                    d.action
                )));
            }
            if d.action == "promote" {
                let target = d.to_grade.as_deref().ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "decision for student {} is missing to_grade",
                        d.student_id
                    ))
                })?;
                validate_grade_level(&self.pool, org_id, target).await?;
            }
        }

        let batch_id = Uuid::new_v4();
        let effective_date = req.effective_date.unwrap_or_else(today);

        let mut tx = self.pool.begin().await?;
        let mut promoted = 0i64;
        let mut retained = 0i64;
        let mut graduated = 0i64;

        for d in &req.decisions {
            let current: StudentRow = sqlx::query_as(
                "SELECT * FROM students WHERE id = $1 AND org_id = $2",
            )
            .bind(d.student_id)
            .bind(org_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Student {} not found", d.student_id))
            })?;

            let (to_grade, to_section, change_kind) = match d.action.as_str() {
                "promote" => {
                    let to_g = d.to_grade.clone().unwrap();
                    let to_s = d.to_section.clone();
                    sqlx::query(
                        "UPDATE students SET grade_level = $3, section = COALESCE($4, section)
                         WHERE id = $1 AND org_id = $2",
                    )
                    .bind(d.student_id)
                    .bind(org_id)
                    .bind(&to_g)
                    .bind(&to_s)
                    .execute(&mut *tx)
                    .await?;
                    promoted += 1;
                    (Some(to_g), to_s, "promote")
                }
                "retain" => {
                    retained += 1;
                    (
                        Some(current.grade_level.clone()),
                        current.section.clone(),
                        "retain",
                    )
                }
                "graduate" => {
                    sqlx::query(
                        r#"
                        UPDATE students SET
                            status = 'graduated',
                            graduation_date = COALESCE(graduation_date, $3)
                        WHERE id = $1 AND org_id = $2
                        "#,
                    )
                    .bind(d.student_id)
                    .bind(org_id)
                    .bind(effective_date)
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query(
                        r#"
                        INSERT INTO student_status_history
                            (student_id, org_id, from_status, to_status, reason,
                             effective_date, changed_by_user_id)
                        VALUES ($1, $2, $3, 'graduated', $4, $5, $6)
                        "#,
                    )
                    .bind(d.student_id)
                    .bind(org_id)
                    .bind(&current.status)
                    .bind(&d.reason)
                    .bind(effective_date)
                    .bind(changed_by)
                    .execute(&mut *tx)
                    .await?;
                    graduated += 1;
                    (None, None, "graduate")
                }
                _ => unreachable!("validated above"),
            };

            sqlx::query(
                r#"
                INSERT INTO student_class_history
                    (student_id, org_id, from_grade_level, from_section,
                     to_grade_level, to_section, change_kind, reason,
                     effective_date, changed_by_user_id, promotion_batch_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(d.student_id)
            .bind(org_id)
            .bind(&current.grade_level)
            .bind(&current.section)
            .bind(&to_grade)
            .bind(&to_section)
            .bind(change_kind)
            .bind(&d.reason)
            .bind(effective_date)
            .bind(changed_by)
            .bind(batch_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(PromoteSummary {
            promoted,
            retained,
            graduated,
            batch_id,
            errors: vec![],
        })
    }
}
