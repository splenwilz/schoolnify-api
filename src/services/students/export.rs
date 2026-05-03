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
            wtr.write_record([
                s.admission_number.as_str(),
                s.first_name.as_str(),
                s.last_name.as_str(),
                s.middle_name.as_deref().unwrap_or(""),
                s.grade_level.as_str(),
                s.section.as_deref().unwrap_or(""),
                s.gender.as_str(),
                &s.date_of_birth.to_string(),
                s.status.as_str(),
                s.boarding_status.as_deref().unwrap_or(""),
                "unknown",
                primary.as_ref().map(|g| guardian_name(g)).unwrap_or_default().as_str(),
                primary.as_ref().and_then(|g| g.phone.as_deref()).unwrap_or(""),
                primary.as_ref().and_then(|g| g.email.as_deref()).unwrap_or(""),
            ])
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
