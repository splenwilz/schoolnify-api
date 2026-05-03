use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::{PgConnection, QueryBuilder};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::students::{
    BulkImportResponse, ChangeClassRequest, ChangeStatusRequest, CreateStudentRequest,
    GuardianInput, ImportedStudent, PaginationInfo, StatusChangeRecord, StatusChangeResponse,
    StudentClassHistoryRow, StudentGuardianRow, StudentListQuery, StudentListResponse,
    StudentResponse, StudentRow, StudentStatusHistoryRow, StudentSummary, UpdateStudentRequest,
};

use super::admission;
use super::StudentsService;

const ALLOWED_GENDERS: &[&str] = &["male", "female"];
const ALLOWED_STATUSES: &[&str] = &[
    "active",
    "inactive",
    "suspended",
    "graduated",
    "withdrawn",
    "transferred",
];
const ALLOWED_BOARDING: &[&str] = &["day", "boarding", "weekly_boarding"];

const MAX_GUARDIANS: usize = 3;
const DEFAULT_PAGE_SIZE: i64 = 25;
const MAX_PAGE_SIZE: i64 = 100;

impl StudentsService {
    /// Create a single student with optional guardians.
    /// Auto-generates admission_number if not supplied.
    pub async fn create(
        &self,
        org_id: Uuid,
        req: CreateStudentRequest,
    ) -> Result<StudentResponse, AppError> {
        validate_gender(&req.gender)?;
        if let Some(ref bs) = req.boarding_status {
            validate_boarding(bs)?;
        }
        validate_grade_level(&self.pool, org_id, &req.grade_level).await?;
        validate_guardians(&req.guardians)?;

        let mut tx = self.pool.begin().await?;

        let admission_number = match req.admission_number {
            Some(ref s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => admission::generate_admission_number(&mut tx, org_id).await?,
        };

        let enrollment_date = req.enrollment_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let student: StudentRow = sqlx::query_as(
            r#"
            INSERT INTO students (
                org_id, admission_number, first_name, middle_name, last_name,
                date_of_birth, gender, grade_level, section, stream,
                enrollment_date, boarding_status,
                phone, email, address, city, state, postal_code,
                blood_group, genotype, allergies, medical_conditions,
                previous_school, state_of_origin, lga, religion, tribe,
                avatar_url
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12,
                $13, $14, $15, $16, $17, $18,
                $19, $20, $21, $22,
                $23, $24, $25, $26, $27,
                $28
            )
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&admission_number)
        .bind(&req.first_name)
        .bind(&req.middle_name)
        .bind(&req.last_name)
        .bind(req.date_of_birth)
        .bind(&req.gender)
        .bind(&req.grade_level)
        .bind(&req.section)
        .bind(&req.stream)
        .bind(enrollment_date)
        .bind(&req.boarding_status)
        .bind(&req.phone)
        .bind(&req.email)
        .bind(&req.address)
        .bind(&req.city)
        .bind(&req.state)
        .bind(&req.postal_code)
        .bind(&req.blood_group)
        .bind(&req.genotype)
        .bind(&req.allergies)
        .bind(&req.medical_conditions)
        .bind(&req.previous_school)
        .bind(&req.state_of_origin)
        .bind(&req.lga)
        .bind(&req.religion)
        .bind(&req.tribe)
        .bind(&req.avatar_url)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_unique_violation)?;

        let guardians = insert_guardians(&mut tx, student.id, org_id, &req.guardians).await?;

        tx.commit().await?;

        Ok(StudentResponse::from_row(student, guardians))
    }

    /// Get one student by id, scoped to org.
    pub async fn get(
        &self,
        org_id: Uuid,
        student_id: Uuid,
        include: &str,
    ) -> Result<StudentResponse, AppError> {
        let student: StudentRow = sqlx::query_as(
            "SELECT * FROM students WHERE id = $1 AND org_id = $2",
        )
        .bind(student_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Student not found".into()))?;

        let mut guardians_map = fetch_guardians_for_students(&self.pool, &[student.id]).await?;
        let guardians = guardians_map.remove(&student.id).unwrap_or_default();

        let mut response = StudentResponse::from_row(student, guardians);

        for token in include.split(',').map(str::trim) {
            match token {
                "recent_payments" => response.recent_payments = Some(vec![]),
                "recent_attendance" => response.recent_attendance = Some(vec![]),
                _ => {}
            }
        }

        Ok(response)
    }

    /// List students with filters, search, pagination, and whole-school summary.
    pub async fn list(
        &self,
        org_id: Uuid,
        q: StudentListQuery,
    ) -> Result<StudentListResponse, AppError> {
        let page = q.page.unwrap_or(1).max(1);
        let page_size = q
            .page_size
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE);
        // saturating math so an extreme `?page=...` value can't panic in debug
        // or wrap silently in release; an empty page is the right answer past the end.
        let offset = page.saturating_sub(1).saturating_mul(page_size);

        let sort_col = match q.sort.as_deref() {
            Some("first_name") => "first_name",
            Some("admission_number") => "admission_number",
            Some("enrollment_date") => "enrollment_date",
            Some("created_at") => "created_at",
            _ => "last_name",
        };
        let order = match q.order.as_deref() {
            Some("desc") | Some("DESC") => "DESC",
            _ => "ASC",
        };

        let (page_data, total, summary) = tokio::try_join!(
            fetch_page(&self.pool, org_id, &q, sort_col, order, page_size, offset),
            count_total(&self.pool, org_id, &q),
            fetch_summary(&self.pool, org_id),
        )?;

        let total_pages = if total == 0 {
            0
        } else {
            ((total as f64) / (page_size as f64)).ceil() as i64
        };

        let student_ids: Vec<Uuid> = page_data.iter().map(|s| s.id).collect();
        let mut guardians_map = fetch_guardians_for_students(&self.pool, &student_ids).await?;

        let data: Vec<StudentResponse> = page_data
            .into_iter()
            .map(|s| {
                let g = guardians_map.remove(&s.id).unwrap_or_default();
                StudentResponse::from_row(s, g)
            })
            .collect();

        Ok(StudentListResponse {
            data,
            pagination: PaginationInfo {
                page,
                page_size,
                total,
                total_pages,
            },
            summary,
        })
    }

    /// Update student fields (not status, class, or admission_number).
    /// If `guardians` is Some, replaces the full guardian set for this student.
    pub async fn patch(
        &self,
        org_id: Uuid,
        student_id: Uuid,
        req: UpdateStudentRequest,
    ) -> Result<StudentResponse, AppError> {
        if let Some(ref g) = req.gender {
            validate_gender(g)?;
        }
        if let Some(ref b) = req.boarding_status {
            validate_boarding(b)?;
        }
        if let Some(ref guardians) = req.guardians {
            validate_guardians(guardians)?;
        }

        let mut tx = self.pool.begin().await?;

        let student: StudentRow = sqlx::query_as(
            r#"
            UPDATE students SET
                first_name        = COALESCE($3, first_name),
                middle_name       = COALESCE($4, middle_name),
                last_name         = COALESCE($5, last_name),
                date_of_birth     = COALESCE($6, date_of_birth),
                gender            = COALESCE($7, gender),
                stream            = COALESCE($8, stream),
                boarding_status   = COALESCE($9, boarding_status),
                phone             = COALESCE($10, phone),
                email             = COALESCE($11, email),
                address           = COALESCE($12, address),
                city              = COALESCE($13, city),
                state             = COALESCE($14, state),
                postal_code       = COALESCE($15, postal_code),
                blood_group       = COALESCE($16, blood_group),
                genotype          = COALESCE($17, genotype),
                allergies         = COALESCE($18, allergies),
                medical_conditions = COALESCE($19, medical_conditions),
                previous_school   = COALESCE($20, previous_school),
                state_of_origin   = COALESCE($21, state_of_origin),
                lga               = COALESCE($22, lga),
                religion          = COALESCE($23, religion),
                tribe             = COALESCE($24, tribe),
                avatar_url        = COALESCE($25, avatar_url)
            WHERE id = $1 AND org_id = $2
            RETURNING *
            "#,
        )
        .bind(student_id)
        .bind(org_id)
        .bind(&req.first_name)
        .bind(&req.middle_name)
        .bind(&req.last_name)
        .bind(req.date_of_birth)
        .bind(&req.gender)
        .bind(&req.stream)
        .bind(&req.boarding_status)
        .bind(&req.phone)
        .bind(&req.email)
        .bind(&req.address)
        .bind(&req.city)
        .bind(&req.state)
        .bind(&req.postal_code)
        .bind(&req.blood_group)
        .bind(&req.genotype)
        .bind(&req.allergies)
        .bind(&req.medical_conditions)
        .bind(&req.previous_school)
        .bind(&req.state_of_origin)
        .bind(&req.lga)
        .bind(&req.religion)
        .bind(&req.tribe)
        .bind(&req.avatar_url)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Student not found".into()))?;

        let guardians = if let Some(new_guardians) = req.guardians {
            sqlx::query("DELETE FROM student_guardians WHERE student_id = $1")
                .bind(student_id)
                .execute(&mut *tx)
                .await?;
            insert_guardians(&mut tx, student_id, org_id, &new_guardians).await?
        } else {
            sqlx::query_as::<_, StudentGuardianRow>(
                "SELECT * FROM student_guardians WHERE student_id = $1 ORDER BY position",
            )
            .bind(student_id)
            .fetch_all(&mut *tx)
            .await?
        };

        tx.commit().await?;

        Ok(StudentResponse::from_row(student, guardians))
    }

    /// Soft-delete: marks student as `withdrawn` and writes a status_history row.
    /// Idempotent — calling DELETE twice on the same student returns Ok both times
    /// (no duplicate history row on the second call).
    pub async fn soft_delete(
        &self,
        org_id: Uuid,
        student_id: Uuid,
        changed_by: Option<Uuid>,
    ) -> Result<(), AppError> {
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM students WHERE id = $1 AND org_id = $2",
        )
        .bind(student_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        match current_status.as_deref() {
            None => Err(AppError::NotFound("Student not found".into())),
            Some("withdrawn") => Ok(()),
            Some(_) => {
                let req = ChangeStatusRequest {
                    status: "withdrawn".into(),
                    reason: Some("deleted via API".into()),
                    effective_date: None,
                };
                self.change_status(org_id, student_id, req, changed_by).await?;
                Ok(())
            }
        }
    }

    /// Change a student's status with audit history.
    pub async fn change_status(
        &self,
        org_id: Uuid,
        student_id: Uuid,
        req: ChangeStatusRequest,
        changed_by: Option<Uuid>,
    ) -> Result<StatusChangeResponse, AppError> {
        if !ALLOWED_STATUSES.contains(&req.status.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid status; must be one of {:?}",
                ALLOWED_STATUSES
            )));
        }
        let effective_date = req.effective_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let mut tx = self.pool.begin().await?;

        let current: StudentRow = sqlx::query_as(
            "SELECT * FROM students WHERE id = $1 AND org_id = $2",
        )
        .bind(student_id)
        .bind(org_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Student not found".into()))?;

        if current.status == req.status {
            return Err(AppError::BadRequest(format!(
                "Student already has status '{}'",
                req.status
            )));
        }

        let updated: StudentRow = sqlx::query_as(
            r#"
            UPDATE students SET
                status = $3,
                graduation_date = CASE
                    WHEN $3 = 'graduated' THEN COALESCE(graduation_date, $4)
                    ELSE graduation_date END,
                withdrawn_at = CASE
                    WHEN $3 = 'withdrawn' THEN COALESCE(withdrawn_at, NOW())
                    ELSE withdrawn_at END
            WHERE id = $1 AND org_id = $2
            RETURNING *
            "#,
        )
        .bind(student_id)
        .bind(org_id)
        .bind(&req.status)
        .bind(effective_date)
        .fetch_one(&mut *tx)
        .await?;

        let history: StudentStatusHistoryRow = sqlx::query_as(
            r#"
            INSERT INTO student_status_history
                (student_id, org_id, from_status, to_status, reason, effective_date, changed_by_user_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(student_id)
        .bind(org_id)
        .bind(&current.status)
        .bind(&req.status)
        .bind(&req.reason)
        .bind(effective_date)
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;

        let guardians = sqlx::query_as::<_, StudentGuardianRow>(
            "SELECT * FROM student_guardians WHERE student_id = $1 ORDER BY position",
        )
        .bind(student_id)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(StatusChangeResponse {
            student: StudentResponse::from_row(updated, guardians),
            status_change: StatusChangeRecord::from(history),
        })
    }

    /// Change a student's grade_level / section with audit history.
    pub async fn change_class(
        &self,
        org_id: Uuid,
        student_id: Uuid,
        req: ChangeClassRequest,
        changed_by: Option<Uuid>,
    ) -> Result<StudentResponse, AppError> {
        validate_grade_level(&self.pool, org_id, &req.grade_level).await?;
        let effective_date = req.effective_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let mut tx = self.pool.begin().await?;

        let current: StudentRow = sqlx::query_as(
            "SELECT * FROM students WHERE id = $1 AND org_id = $2",
        )
        .bind(student_id)
        .bind(org_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Student not found".into()))?;

        let updated: StudentRow = sqlx::query_as(
            r#"
            UPDATE students SET
                grade_level = $3,
                section = COALESCE($4, section),
                stream = COALESCE($5, stream)
            WHERE id = $1 AND org_id = $2
            RETURNING *
            "#,
        )
        .bind(student_id)
        .bind(org_id)
        .bind(&req.grade_level)
        .bind(&req.section)
        .bind(&req.stream)
        .fetch_one(&mut *tx)
        .await?;

        let _: StudentClassHistoryRow = sqlx::query_as(
            r#"
            INSERT INTO student_class_history
                (student_id, org_id, from_grade_level, from_section, to_grade_level, to_section,
                 change_kind, reason, effective_date, changed_by_user_id, promotion_batch_id)
            VALUES ($1, $2, $3, $4, $5, $6, 'manual', $7, $8, $9, NULL)
            RETURNING *
            "#,
        )
        .bind(student_id)
        .bind(org_id)
        .bind(&current.grade_level)
        .bind(&current.section)
        .bind(&req.grade_level)
        .bind(&updated.section)
        .bind(&req.reason)
        .bind(effective_date)
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;

        let guardians = sqlx::query_as::<_, StudentGuardianRow>(
            "SELECT * FROM student_guardians WHERE student_id = $1 ORDER BY position",
        )
        .bind(student_id)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(StudentResponse::from_row(updated, guardians))
    }

    pub async fn bulk_import_response(
        &self,
        imported: usize,
        skipped: usize,
        errors: Vec<crate::models::students::ImportRowError>,
        imported_students: Vec<ImportedStudent>,
    ) -> BulkImportResponse {
        BulkImportResponse {
            imported,
            skipped,
            errors,
            imported_students,
        }
    }
}

// ── Validation ──────────────────────────────────────────────────────────

pub(super) fn validate_gender(g: &str) -> Result<(), AppError> {
    if !ALLOWED_GENDERS.contains(&g) {
        return Err(AppError::BadRequest(format!(
            "Invalid gender '{g}'; must be one of {:?}",
            ALLOWED_GENDERS
        )));
    }
    Ok(())
}

pub(super) fn validate_boarding(b: &str) -> Result<(), AppError> {
    if !ALLOWED_BOARDING.contains(&b) {
        return Err(AppError::BadRequest(format!(
            "Invalid boarding_status '{b}'; must be one of {:?}",
            ALLOWED_BOARDING
        )));
    }
    Ok(())
}

pub(super) async fn validate_grade_level(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    grade_level: &str,
) -> Result<(), AppError> {
    let exists: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM school_grade_levels WHERE org_id = $1 AND name = $2)",
    )
    .bind(org_id)
    .bind(grade_level)
    .fetch_optional(pool)
    .await?;

    if exists.unwrap_or(false) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "grade_level '{grade_level}' is not configured for this school"
        )))
    }
}

pub(super) fn validate_guardians(guardians: &[GuardianInput]) -> Result<(), AppError> {
    if guardians.len() > MAX_GUARDIANS {
        return Err(AppError::BadRequest(format!(
            "At most {MAX_GUARDIANS} guardians allowed"
        )));
    }
    let primary_count = guardians.iter().filter(|g| g.is_primary.unwrap_or(false)).count();
    if primary_count > 1 {
        return Err(AppError::BadRequest(
            "Only one guardian may be marked is_primary".into(),
        ));
    }
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────

pub(super) async fn insert_guardians(
    tx: &mut PgConnection,
    student_id: Uuid,
    org_id: Uuid,
    guardians: &[GuardianInput],
) -> Result<Vec<StudentGuardianRow>, AppError> {
    let mut rows = Vec::with_capacity(guardians.len());
    let any_primary = guardians.iter().any(|g| g.is_primary.unwrap_or(false));

    for (i, g) in guardians.iter().enumerate() {
        let is_primary = if any_primary {
            g.is_primary.unwrap_or(false)
        } else {
            // No explicit primary → first guardian becomes primary.
            i == 0
        };
        let row: StudentGuardianRow = sqlx::query_as(
            r#"
            INSERT INTO student_guardians
                (student_id, org_id, first_name, last_name, phone, email,
                 relationship, occupation, is_primary, position)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(student_id)
        .bind(org_id)
        .bind(&g.first_name)
        .bind(&g.last_name)
        .bind(&g.phone)
        .bind(&g.email)
        .bind(&g.relationship)
        .bind(&g.occupation)
        .bind(is_primary)
        .bind(i as i16)
        .fetch_one(&mut *tx)
        .await?;
        rows.push(row);
    }
    Ok(rows)
}

pub(super) async fn fetch_guardians_for_students(
    pool: &sqlx::PgPool,
    student_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<StudentGuardianRow>>, AppError> {
    let mut map: HashMap<Uuid, Vec<StudentGuardianRow>> = HashMap::new();
    if student_ids.is_empty() {
        return Ok(map);
    }
    let rows: Vec<StudentGuardianRow> = sqlx::query_as(
        "SELECT * FROM student_guardians WHERE student_id = ANY($1) ORDER BY student_id, position",
    )
    .bind(student_ids)
    .fetch_all(pool)
    .await?;
    for r in rows {
        map.entry(r.student_id).or_default().push(r);
    }
    Ok(map)
}

fn map_unique_violation(e: sqlx::Error) -> AppError {
    match &e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            AppError::Conflict("admission_number already exists for this school".into())
        }
        _ => e.into(),
    }
}

async fn fetch_page(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    q: &StudentListQuery,
    sort_col: &str,
    order: &str,
    page_size: i64,
    offset: i64,
) -> Result<Vec<StudentRow>, AppError> {
    let mut qb = QueryBuilder::<sqlx::Postgres>::new("SELECT * FROM students WHERE org_id = ");
    qb.push_bind(org_id);
    push_filters(&mut qb, q);
    qb.push(format!(" ORDER BY {sort_col} {order} LIMIT "));
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb.build_query_as::<StudentRow>().fetch_all(pool).await?;
    Ok(rows)
}

async fn count_total(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    q: &StudentListQuery,
) -> Result<i64, AppError> {
    let mut qb =
        QueryBuilder::<sqlx::Postgres>::new("SELECT COUNT(*) FROM students WHERE org_id = ");
    qb.push_bind(org_id);
    push_filters(&mut qb, q);

    let total: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(total)
}

async fn fetch_summary(pool: &sqlx::PgPool, org_id: Uuid) -> Result<StudentSummary, AppError> {
    let row: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint AS total_students,
            COUNT(*) FILTER (WHERE status = 'active')::bigint AS active
        FROM students WHERE org_id = $1
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await?;

    Ok(StudentSummary {
        total_students: row.0,
        active: row.1,
        average_gpa: None,
        average_attendance: None,
    })
}

fn push_filters(qb: &mut QueryBuilder<'_, sqlx::Postgres>, q: &StudentListQuery) {
    // Status: default 'active' unless explicitly set. "all" disables the filter.
    let status_filter = match q.status.as_deref() {
        None | Some("") => Some("active".to_string()),
        Some("all") => None,
        Some(s) => Some(s.to_string()),
    };
    if let Some(status) = status_filter {
        qb.push(" AND status = ");
        qb.push_bind(status);
    }

    if let Some(g) = q.grade_level.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND grade_level = ");
        qb.push_bind(g.to_string());
    }
    if let Some(s) = q.section.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND section = ");
        qb.push_bind(s.to_string());
    }
    if let Some(g) = q.gender.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND gender = ");
        qb.push_bind(g.to_string());
    }
    if let Some(b) = q.boarding_status.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND boarding_status = ");
        qb.push_bind(b.to_string());
    }

    if let Some(search) = q.search.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{search}%");
        qb.push(" AND (first_name ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR last_name ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR admission_number ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(
            " OR EXISTS (SELECT 1 FROM student_guardians g WHERE g.student_id = students.id AND (g.first_name ILIKE ",
        );
        qb.push_bind(pattern.clone());
        qb.push(" OR g.last_name ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR g.phone ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR g.email ILIKE ");
        qb.push_bind(pattern);
        qb.push(")))");
    }
}

/// Used by export.rs for an unpaginated, filtered scan with the same WHERE.
pub(super) async fn fetch_filtered(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    q: &StudentListQuery,
) -> Result<Vec<StudentRow>, AppError> {
    let sort_col = match q.sort.as_deref() {
        Some("first_name") => "first_name",
        Some("admission_number") => "admission_number",
        Some("enrollment_date") => "enrollment_date",
        Some("created_at") => "created_at",
        _ => "last_name",
    };
    let order = match q.order.as_deref() {
        Some("desc") | Some("DESC") => "DESC",
        _ => "ASC",
    };

    let mut qb = QueryBuilder::<sqlx::Postgres>::new("SELECT * FROM students WHERE org_id = ");
    qb.push_bind(org_id);
    push_filters(&mut qb, q);
    qb.push(format!(" ORDER BY {sort_col} {order}"));

    let rows = qb.build_query_as::<StudentRow>().fetch_all(pool).await?;
    Ok(rows)
}

pub(super) fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}
