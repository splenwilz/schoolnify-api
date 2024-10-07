use sqlx::PgPool;
use crate::models::Tenant;
use uuid::Uuid;

pub async fn create_tenant(
    pool: &PgPool,
    name: &str,
    domain: Option<&str>,
    address: &str,
    contact_email: &str,
    contact_phone: Option<&str>,
    logo_url: Option<&str>,
    timezone: &str,
) -> Result<Uuid, sqlx::Error> {
    // Check if the domain already exists
    if let Some(domain_value) = domain {
        let existing_tenant = sqlx::query!(
            r#"
            SELECT id FROM Tenant WHERE domain = $1
            "#,
            domain_value
        )
        .fetch_optional(pool)
        .await?;

        // If the query returns some result, it means a tenant with the same domain exists
        if existing_tenant.is_some() {
            return Err(sqlx::Error::RowNotFound);  // Return an error if domain exists
        }
    }

    // Proceed to create a new tenant if no existing domain was found
    let new_tenant_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO Tenant (id, name, domain, address, contact_email, contact_phone, logo_url, timezone)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        new_tenant_id,
        name,
        domain,
        address,
        contact_email,
        contact_phone,
        logo_url,
        timezone
    )
    .execute(pool)
    .await?;

    Ok(new_tenant_id)
}



pub async fn get_tenants(pool: &PgPool) -> Result<Vec<Tenant>, sqlx::Error> {
    let tenants = sqlx::query_as!(
        Tenant,
        r#"
        SELECT id, name, domain, address, contact_email, contact_phone, logo_url, timezone, created_at, is_active
        FROM Tenant
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(tenants)
}



pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    first_name: &str,
    last_name: &str,
    date_of_birth: Option<chrono::NaiveDate>,
    gender: Option<&str>,
    contact_phone: Option<&str>,
    address: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let new_user_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO "User" (id, email, password_hash, first_name, last_name, date_of_birth, gender, contact_phone, address)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        new_user_id,
        email,
        password_hash,
        first_name,
        last_name,
        date_of_birth,
        gender,
        contact_phone,
        address
    )
    .execute(pool)
    .await?;

    Ok(new_user_id)
}


