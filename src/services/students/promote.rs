use std::collections::HashSet;

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

        // Pre-validate all action enums, dedup student_ids, and check target
        // grade levels before opening the tx.
        let mut seen: HashSet<Uuid> = HashSet::with_capacity(req.decisions.len());
        for d in &req.decisions {
            if !seen.insert(d.student_id) {
                // Two decisions for the same student would produce contradictory
                // updates (e.g. promote+graduate) and inflate PromoteSummary counts.
                return Err(AppError::BadRequest(format!(
                    "duplicate student_id {} in decisions array",
                    d.student_id
                )));
            }
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

            let (to_grade, to_section, to_stream, change_kind) = match d.action.as_str() {
                "promote" => {
                    let to_g = d.to_grade.clone().unwrap();
                    // If the caller didn't specify a section, the UPDATE keeps
                    // the existing section via COALESCE. The audit row must
                    // record the *effective* section, not the raw `None` from
                    // the request, otherwise the history diverges from the row.
                    let effective_section =
                        d.to_section.clone().or_else(|| current.section.clone());
                    sqlx::query(
                        "UPDATE students SET grade_level = $3, section = COALESCE($4, section)
                         WHERE id = $1 AND org_id = $2",
                    )
                    .bind(d.student_id)
                    .bind(org_id)
                    .bind(&to_g)
                    .bind(&d.to_section)
                    .execute(&mut *tx)
                    .await?;
                    promoted += 1;
                    // Stream isn't changed by promote; record current.stream so
                    // the audit row reflects the post-promotion state.
                    (Some(to_g), effective_section, current.stream.clone(), "promote")
                }
                "retain" => {
                    retained += 1;
                    (
                        Some(current.grade_level.clone()),
                        current.section.clone(),
                        current.stream.clone(),
                        "retain",
                    )
                }
                "graduate" => {
                    // Bidirectional CHECK requires withdrawn_at NULL when status='graduated'.
                    sqlx::query(
                        r#"
                        UPDATE students SET
                            status = 'graduated',
                            graduation_date = COALESCE(graduation_date, $3),
                            withdrawn_at = NULL
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
                    (None, None, None, "graduate")
                }
                _ => unreachable!("validated above"),
            };

            sqlx::query(
                r#"
                INSERT INTO student_class_history
                    (student_id, org_id,
                     from_grade_level, from_section, from_stream,
                     to_grade_level, to_section, to_stream,
                     change_kind, reason, effective_date, changed_by_user_id, promotion_batch_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
            )
            .bind(d.student_id)
            .bind(org_id)
            .bind(&current.grade_level)
            .bind(&current.section)
            .bind(&current.stream)
            .bind(&to_grade)
            .bind(&to_section)
            .bind(&to_stream)
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
