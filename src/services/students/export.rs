use std::collections::HashMap;

use uuid::Uuid;

use crate::errors::AppError;
use crate::models::students::{StudentGuardianRow, StudentListQuery};

use super::crud::{fetch_filtered, fetch_guardians_for_students};
use super::StudentsService;

const CSV_HEADERS: &[&str] = &[
    "Admission No",
    "First Name",
    "Last Name",
    "Middle Name",
    "Grade",
    "Section",
    "Gender",
    "DOB",
    "Status",
    "Boarding",
    "Fee Status",
    "Guardian Name",
    "Guardian Phone",
    "Guardian Email",
];

impl StudentsService {
    /// Export filtered student list as CSV bytes.
    /// Buffers in memory; acceptable up to ~10k students. Future: stream via channel.
    pub async fn export_csv(
        &self,
        org_id: Uuid,
        q: StudentListQuery,
    ) -> Result<Vec<u8>, AppError> {
        let students = fetch_filtered(&self.pool, org_id, &q).await?;
        let ids: Vec<Uuid> = students.iter().map(|s| s.id).collect();
        let guardians_map = fetch_guardians_for_students(&self.pool, &ids).await?;

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(CSV_HEADERS)
            .map_err(|e| AppError::Internal(format!("csv header: {e}")))?;

        for s in students {
            let primary = primary_guardian(&guardians_map, &s.id);
            let dob = s.date_of_birth.to_string();
            let g_name = primary.as_ref().map(|g| guardian_name(g)).unwrap_or_default();
            let g_phone = primary.as_ref().and_then(|g| g.phone.as_deref()).unwrap_or("");
            let g_email = primary.as_ref().and_then(|g| g.email.as_deref()).unwrap_or("");

            // Every user-controlled cell goes through CSV-injection sanitization
            // so a name/phone/email starting with =, +, -, @ or tab can't be
            // interpreted as a formula when opened in Excel/Sheets.
            let row = [
                csv_safe(&s.admission_number),
                csv_safe(&s.first_name),
                csv_safe(&s.last_name),
                csv_safe(s.middle_name.as_deref().unwrap_or("")),
                csv_safe(&s.grade_level),
                csv_safe(s.section.as_deref().unwrap_or("")),
                csv_safe(&s.gender),
                csv_safe(&dob),
                csv_safe(&s.status),
                csv_safe(s.boarding_status.as_deref().unwrap_or("")),
                csv_safe("unknown"),
                csv_safe(&g_name),
                csv_safe(g_phone),
                csv_safe(g_email),
            ];
            wtr.write_record(&row)
                .map_err(|e| AppError::Internal(format!("csv row: {e}")))?;
        }

        wtr.flush()
            .map_err(|e| AppError::Internal(format!("csv flush: {e}")))?;
        wtr.into_inner()
            .map_err(|e| AppError::Internal(format!("csv finalize: {e}")))
    }
}

fn primary_guardian<'a>(
    map: &'a HashMap<Uuid, Vec<StudentGuardianRow>>,
    student_id: &Uuid,
) -> Option<&'a StudentGuardianRow> {
    let list = map.get(student_id)?;
    list.iter().find(|g| g.is_primary).or_else(|| list.first())
}

fn guardian_name(g: &StudentGuardianRow) -> String {
    format!("{} {}", g.first_name, g.last_name).trim().to_string()
}

/// Neutralize CSV-formula characters at the start of a cell.
/// If the value begins with `=`, `+`, `-`, `@`, tab, or carriage return,
/// prefix with a single quote so spreadsheet apps treat it as text.
fn csv_safe(value: &str) -> String {
    if matches!(value.chars().next(), Some('=' | '+' | '-' | '@' | '\t' | '\r')) {
        format!("'{value}")
    } else {
        value.to_string()
    }
}
