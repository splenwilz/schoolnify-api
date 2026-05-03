use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::Acquire;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::students::{
    BulkImportResponse, GuardianInput, ImportRowError, ImportedStudent, StudentRow,
};

use super::admission::generate_admission_number;
use super::crud::insert_guardians;
use super::StudentsService;

const MAX_IMPORT_ROWS: usize = 5000;

/// Per-row candidate after applying the mapping. Validation happens in build().
struct Candidate {
    row_num: usize,
    first_name: String,
    middle_name: Option<String>,
    last_name: String,
    date_of_birth: NaiveDate,
    gender: String,
    grade_level: String,
    section: Option<String>,
    stream: Option<String>,
    admission_number: Option<String>,
    enrollment_date: Option<NaiveDate>,
    boarding_status: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    blood_group: Option<String>,
    genotype: Option<String>,
    allergies: Option<String>,
    medical_conditions: Option<String>,
    previous_school: Option<String>,
    state_of_origin: Option<String>,
    lga: Option<String>,
    religion: Option<String>,
    tribe: Option<String>,
    avatar_url: Option<String>,
    guardians: Vec<GuardianInput>,
}

impl StudentsService {
    /// Bulk import students from a CSV byte buffer + column mapping.
    /// `mapping` maps CSV header names to our field keys (e.g. "Surname" → "last_name",
    /// "Father Name" → "guardian1_first_name").
    pub async fn bulk_import(
        &self,
        org_id: Uuid,
        csv_bytes: &[u8],
        mapping: HashMap<String, String>,
        skip_invalid: bool,
        _changed_by: Option<Uuid>,
    ) -> Result<(BulkImportResponse, bool), AppError> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .trim(csv::Trim::All)
            .from_reader(csv_bytes);

        let headers = rdr
            .headers()
            .map_err(|e| AppError::BadRequest(format!("Invalid CSV header: {e}")))?
            .clone();

        // Pre-resolve header index → field key
        let mut header_to_field: Vec<Option<String>> = Vec::with_capacity(headers.len());
        for h in headers.iter() {
            header_to_field.push(mapping.get(h).cloned());
        }

        let mut valid: Vec<Candidate> = Vec::new();
        let mut errors: Vec<ImportRowError> = Vec::new();
        let mut total_rows: usize = 0;

        for (i, result) in rdr.records().enumerate() {
            let row_num = i + 2; // 1 = header
            total_rows += 1;
            if total_rows > MAX_IMPORT_ROWS {
                errors.push(ImportRowError {
                    row: row_num,
                    field: None,
                    message: format!("Row limit {MAX_IMPORT_ROWS} exceeded; remaining rows skipped"),
                });
                break;
            }
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    errors.push(ImportRowError {
                        row: row_num,
                        field: None,
                        message: format!("Failed to read row: {e}"),
                    });
                    continue;
                }
            };

            match build_candidate(row_num, &record, &header_to_field) {
                Ok(c) => valid.push(c),
                Err(err) => errors.push(err),
            }
        }

        let imported_count = valid.len();
        let skipped_count = errors.len();

        // If invalid rows present and not skipping, signal 422 to the handler.
        if !errors.is_empty() && !skip_invalid {
            let response = BulkImportResponse {
                imported: 0,
                skipped: skipped_count,
                errors,
                imported_students: vec![],
            };
            return Ok((response, false));
        }

        // Insert all valid rows in a single transaction.
        let mut imported_students: Vec<ImportedStudent> = Vec::with_capacity(imported_count);
        let mut tx = self.pool.begin().await?;

        for c in valid {
            // Validate grade_level inside the loop so we get per-row errors.
            // Use a separate connection for validation to avoid borrow conflicts.
            let exists: Option<bool> = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM school_grade_levels WHERE org_id = $1 AND name = $2)",
            )
            .bind(org_id)
            .bind(&c.grade_level)
            .fetch_optional(&mut *tx)
            .await?;
            if !exists.unwrap_or(false) {
                // Row failed grade_level validation post-parse: rollback if not skipping.
                if !skip_invalid {
                    tx.rollback().await?;
                    let response = BulkImportResponse {
                        imported: 0,
                        skipped: skipped_count + 1,
                        errors: vec![ImportRowError {
                            row: c.row_num,
                            field: Some("grade_level".into()),
                            message: format!(
                                "grade_level '{}' not configured for this school",
                                c.grade_level
                            ),
                        }],
                        imported_students: vec![],
                    };
                    return Ok((response, false));
                }
                errors.push(ImportRowError {
                    row: c.row_num,
                    field: Some("grade_level".into()),
                    message: format!(
                        "grade_level '{}' not configured for this school",
                        c.grade_level
                    ),
                });
                continue;
            }

            let admission_number = match c.admission_number.as_deref() {
                Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                _ => generate_admission_number(&mut tx, org_id).await?,
            };
            let enrollment_date = c.enrollment_date.unwrap_or_else(super::crud::today);

            // Wrap each student INSERT in a SAVEPOINT so a unique-violation on
            // an admin-supplied admission_number doesn't poison the whole tx.
            // Without this, `?` would propagate, sqlx would auto-rollback the
            // entire bulk import, and `skip_invalid=true` couldn't honor its
            // contract of importing valid rows alongside row-level errors.
            let mut sp = tx.begin().await?;

            let result = sqlx::query_as::<_, StudentRow>(
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
            .bind(&c.first_name)
            .bind(&c.middle_name)
            .bind(&c.last_name)
            .bind(c.date_of_birth)
            .bind(&c.gender)
            .bind(&c.grade_level)
            .bind(&c.section)
            .bind(&c.stream)
            .bind(enrollment_date)
            .bind(&c.boarding_status)
            .bind(&c.phone)
            .bind(&c.email)
            .bind(&c.address)
            .bind(&c.city)
            .bind(&c.state)
            .bind(&c.postal_code)
            .bind(&c.blood_group)
            .bind(&c.genotype)
            .bind(&c.allergies)
            .bind(&c.medical_conditions)
            .bind(&c.previous_school)
            .bind(&c.state_of_origin)
            .bind(&c.lga)
            .bind(&c.religion)
            .bind(&c.tribe)
            .bind(&c.avatar_url)
            .fetch_one(&mut *sp)
            .await;

            match result {
                Ok(inserted) => {
                    // Guardian inserts are inside the same savepoint, so a guardian
                    // failure can be isolated to this row instead of aborting the
                    // whole import. Mirrors the unique-violation arm below.
                    match insert_guardians(&mut sp, inserted.id, org_id, &c.guardians).await {
                        Ok(_) => {
                            sp.commit().await?;
                            imported_students.push(ImportedStudent {
                                id: inserted.id,
                                admission_number: inserted.admission_number.clone(),
                                first_name: inserted.first_name.clone(),
                                last_name: inserted.last_name.clone(),
                            });
                        }
                        Err(e) => {
                            sp.rollback().await?;
                            let msg = format!("guardian insert failed: {e}");
                            if skip_invalid {
                                errors.push(ImportRowError {
                                    row: c.row_num,
                                    field: Some("guardians".into()),
                                    message: msg,
                                });
                                continue;
                            } else {
                                tx.rollback().await?;
                                let response = BulkImportResponse {
                                    imported: 0,
                                    skipped: skipped_count + 1,
                                    errors: vec![ImportRowError {
                                        row: c.row_num,
                                        field: Some("guardians".into()),
                                        message: msg,
                                    }],
                                    imported_students: vec![],
                                };
                                return Ok((response, false));
                            }
                        }
                    }
                }
                Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                    sp.rollback().await?;
                    let msg = format!("admission_number '{admission_number}' already exists");
                    if skip_invalid {
                        errors.push(ImportRowError {
                            row: c.row_num,
                            field: Some("admission_number".into()),
                            message: msg,
                        });
                        continue;
                    } else {
                        tx.rollback().await?;
                        let response = BulkImportResponse {
                            imported: 0,
                            skipped: skipped_count + 1,
                            errors: vec![ImportRowError {
                                row: c.row_num,
                                field: Some("admission_number".into()),
                                message: msg,
                            }],
                            imported_students: vec![],
                        };
                        return Ok((response, false));
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        tx.commit().await?;

        let final_imported = imported_students.len();
        let final_skipped = errors.len();

        let response = BulkImportResponse {
            imported: final_imported,
            skipped: final_skipped,
            errors,
            imported_students,
        };
        Ok((response, true))
    }
}

fn build_candidate(
    row_num: usize,
    record: &csv::StringRecord,
    header_to_field: &[Option<String>],
) -> Result<Candidate, ImportRowError> {
    // Build a field-key → value map for this row.
    let mut fields: HashMap<&str, String> = HashMap::new();
    let mut guardians_raw: HashMap<usize, HashMap<&str, String>> = HashMap::new();

    for (i, field_key_opt) in header_to_field.iter().enumerate() {
        let Some(field_key) = field_key_opt else { continue };
        let value = record.get(i).unwrap_or("").trim().to_string();
        if value.is_empty() {
            continue;
        }
        if let Some((idx, sub)) = parse_guardian_key(field_key) {
            guardians_raw
                .entry(idx)
                .or_default()
                .insert(sub, value);
        } else {
            fields.insert(field_key.as_str(), value);
        }
    }

    let first_name = fields
        .remove("first_name")
        .ok_or_else(|| row_err(row_num, Some("first_name"), "missing"))?;
    let last_name = fields
        .remove("last_name")
        .ok_or_else(|| row_err(row_num, Some("last_name"), "missing"))?;
    let dob_str = fields
        .remove("date_of_birth")
        .ok_or_else(|| row_err(row_num, Some("date_of_birth"), "missing"))?;
    let date_of_birth = NaiveDate::parse_from_str(&dob_str, "%Y-%m-%d")
        .map_err(|_| row_err(row_num, Some("date_of_birth"), "must be YYYY-MM-DD"))?;
    let gender = fields
        .remove("gender")
        .map(|s| s.to_lowercase())
        .ok_or_else(|| row_err(row_num, Some("gender"), "missing"))?;
    if !["male", "female"].contains(&gender.as_str()) {
        return Err(row_err(row_num, Some("gender"), "must be male or female"));
    }
    let grade_level = fields
        .remove("grade_level")
        .ok_or_else(|| row_err(row_num, Some("grade_level"), "missing"))?;

    let enrollment_date = match fields.remove("enrollment_date") {
        Some(s) => Some(
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map_err(|_| row_err(row_num, Some("enrollment_date"), "must be YYYY-MM-DD"))?,
        ),
        None => None,
    };

    let boarding_status = fields.remove("boarding_status").map(|s| s.to_lowercase());
    if let Some(ref bs) = boarding_status
        && !["day", "boarding", "weekly_boarding"].contains(&bs.as_str())
    {
        return Err(row_err(
            row_num,
            Some("boarding_status"),
            "must be day, boarding, or weekly_boarding",
        ));
    }

    // Build guardians from accumulated raw map, preserving 1/2/3 order.
    let mut guardian_indices: Vec<usize> = guardians_raw.keys().copied().collect();
    guardian_indices.sort();
    let mut guardians = Vec::with_capacity(guardian_indices.len());
    for idx in guardian_indices {
        let mut g = guardians_raw.remove(&idx).unwrap();
        let first_name = g.remove("first_name").unwrap_or_default();
        let last_name = g.remove("last_name").unwrap_or_default();
        if first_name.is_empty() && last_name.is_empty() {
            continue;
        }
        guardians.push(GuardianInput {
            first_name,
            last_name,
            phone: g.remove("phone"),
            email: g.remove("email"),
            relationship: g.remove("relationship"),
            occupation: g.remove("occupation"),
            // guardian1 = primary by convention
            is_primary: Some(idx == 1),
        });
    }
    if guardians.len() > 3 {
        return Err(row_err(row_num, None, "at most 3 guardians per student"));
    }

    Ok(Candidate {
        row_num,
        first_name,
        middle_name: fields.remove("middle_name"),
        last_name,
        date_of_birth,
        gender,
        grade_level,
        section: fields.remove("section"),
        stream: fields.remove("stream"),
        admission_number: fields.remove("admission_number"),
        enrollment_date,
        boarding_status,
        phone: fields.remove("phone"),
        email: fields.remove("email"),
        address: fields.remove("address"),
        city: fields.remove("city"),
        state: fields.remove("state"),
        postal_code: fields.remove("postal_code"),
        blood_group: fields.remove("blood_group"),
        genotype: fields.remove("genotype"),
        allergies: fields.remove("allergies"),
        medical_conditions: fields.remove("medical_conditions"),
        previous_school: fields.remove("previous_school"),
        state_of_origin: fields.remove("state_of_origin"),
        lga: fields.remove("lga"),
        religion: fields.remove("religion"),
        tribe: fields.remove("tribe"),
        avatar_url: fields.remove("avatar_url"),
        guardians,
    })
}

/// Parse `guardian1_first_name` → Some((1, "first_name")).
fn parse_guardian_key(key: &str) -> Option<(usize, &str)> {
    let stripped = key.strip_prefix("guardian")?;
    let underscore_idx = stripped.find('_')?;
    let (num_str, rest) = stripped.split_at(underscore_idx);
    let idx: usize = num_str.parse().ok()?;
    if !(1..=3).contains(&idx) {
        return None;
    }
    Some((idx, &rest[1..]))
}

fn row_err(row: usize, field: Option<&str>, message: &str) -> ImportRowError {
    ImportRowError {
        row,
        field: field.map(String::from),
        message: message.into(),
    }
}
