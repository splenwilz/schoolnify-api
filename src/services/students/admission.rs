use chrono::Datelike;
use chrono_tz::Tz;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::errors::AppError;

/// Atomically generate the next admission number for an org.
/// Pattern: `{prefix}/{year}/{seq:03}`. Sequence resets on year boundary
/// **in the school's configured timezone** (falls back to UTC if unset/invalid).
/// If `admission_number_prefix` is NULL, falls back to the org slug (uppercased).
///
/// Uses INSERT ... ON CONFLICT DO UPDATE so the school_configs row is created
/// on first call. Holds a row lock for the duration of the enclosing tx.
pub(super) async fn generate_admission_number(
    tx: &mut PgConnection,
    org_id: Uuid,
) -> Result<String, AppError> {
    // Compute the year in the school's local timezone so a school operating
    // near midnight on Dec 31 doesn't generate next-year admission numbers.
    let tz_str: Option<String> =
        sqlx::query_scalar("SELECT timezone FROM school_configs WHERE org_id = $1")
            .bind(org_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();
    let tz: Tz = tz_str
        .as_deref()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(chrono_tz::UTC);
    let current_year: i16 = chrono::Utc::now().with_timezone(&tz).year() as i16;

    let row: (Option<String>, i32, String) = sqlx::query_as(
        r#"
        WITH bumped AS (
            INSERT INTO school_configs (org_id, admission_number_seq_year, admission_number_next_seq)
            VALUES ($1, $2, 2)
            ON CONFLICT (org_id) DO UPDATE
            SET admission_number_next_seq = CASE
                    WHEN school_configs.admission_number_seq_year = $2
                    THEN school_configs.admission_number_next_seq + 1
                    ELSE 2 END,
                admission_number_seq_year = $2
            RETURNING org_id, admission_number_prefix, admission_number_next_seq - 1 AS used_seq
        )
        SELECT b.admission_number_prefix, b.used_seq, o.slug
        FROM bumped b
        JOIN organizations o ON o.id = b.org_id
        "#,
    )
    .bind(org_id)
    .bind(current_year)
    .fetch_one(&mut *tx)
    .await?;

    let (prefix_opt, used_seq, slug) = row;
    let prefix = prefix_opt
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| slug.to_uppercase());

    Ok(format!("{}/{}/{:03}", prefix, current_year, used_seq))
}
