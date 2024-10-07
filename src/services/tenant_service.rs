use sqlx::PgPool;
use uuid::Uuid;
// use crate::models::tenant::Tenant;
use serde::Deserialize;

use crate::models::tenant::Tenant;

#[derive(Deserialize)]
pub struct TenantRequest {
    pub name: String,
    pub domain: Option<String>,
    pub address: String,
    pub contact_email: String,
    pub contact_phone: Option<String>,
    pub logo_url: Option<String>,
    pub timezone: String,
}

pub async fn create_tenant(pool: &PgPool, request: &TenantRequest) -> Result<Uuid, sqlx::Error> {
    let new_tenant_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO Tenant (id, name, domain, address, contact_email, contact_phone, logo_url, timezone)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        new_tenant_id,
        request.name,
        request.domain,
        request.address,
        request.contact_email,
        request.contact_phone,
        request.logo_url,
        request.timezone
    )
    .execute(pool)
    .await?;

    Ok(new_tenant_id)
}

// Function to fetch all tenants
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

// Function to get a tenant by its ID
pub async fn get_tenant_by_id(pool: &PgPool, tenant_id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
    let tenant = sqlx::query_as!(
        Tenant,
        r#"
        SELECT id, name, domain, address, contact_email, contact_phone, logo_url, timezone, created_at, is_active
        FROM Tenant
        WHERE id = $1
        "#,
        tenant_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(tenant)
}

pub async fn get_tenant_by_name(pool: &PgPool, tenant_name: &str) -> Result<Option<Tenant>, sqlx::Error> {
    let tenant = sqlx::query_as!(
        Tenant,
        r#"
        SELECT id, name, domain, address, contact_email, contact_phone, logo_url, timezone, created_at, is_active
        FROM Tenant
        WHERE name = $1
        "#,
        tenant_name
    )
    .fetch_optional(pool)
    .await?;

    Ok(tenant)
}

pub async fn get_tenant_by_domain(pool: &PgPool, tenant_domain: &str) -> Result<Option<Tenant>, sqlx::Error> {
    let tenant = sqlx::query_as!(
        Tenant,
        r#"
        SELECT id, name, domain, address, contact_email, contact_phone, logo_url, timezone, created_at, is_active
        FROM Tenant
        WHERE domain = $1
        "#,
        tenant_domain
    )
    .fetch_optional(pool)
    .await?;

    Ok(tenant)
}

pub async fn update_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    name: Option<&str>,
    domain: Option<&str>,
    address: Option<&str>,
    contact_email: Option<&str>,
    contact_phone: Option<&str>,
    logo_url: Option<&str>,
    timezone: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE Tenant
        SET name = COALESCE($2, name),
            domain = COALESCE($3, domain),
            address = COALESCE($4, address),
            contact_email = COALESCE($5, contact_email),
            contact_phone = COALESCE($6, contact_phone),
            logo_url = COALESCE($7, logo_url),
            timezone = COALESCE($8, timezone)
        WHERE id = $1
        "#,
        tenant_id,
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

    Ok(())
}

pub async fn delete_tenant(pool: &PgPool, tenant_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM Tenant
        WHERE id = $1
        "#,
        tenant_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_tenant_by_name(pool: &PgPool, tenant_name: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM Tenant
        WHERE name = $1
        "#,
        tenant_name
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_tenant_by_domain(pool: &PgPool, tenant_domain: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM Tenant
        WHERE domain = $1
        "#,
        tenant_domain
    )
    .execute(pool)
    .await?;

    Ok(())
}


