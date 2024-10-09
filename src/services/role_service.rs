use sqlx::PgPool;
use uuid::Uuid;
use crate::models::role::Role;

pub async fn create_role(pool: &PgPool, name: &str, description: Option<&str>) -> Result<Role, sqlx::Error> {
    let role = sqlx::query_as!(
        Role,
        r#"
        INSERT INTO Role (id, name, description)
        VALUES ($1, $2, $3)
        RETURNING id, name, description, created_at
        "#,
        Uuid::new_v4(),
        name,
        description
    )
    .fetch_one(pool)
    .await?;

    Ok(role)
}

pub async fn get_roles(pool: &PgPool) -> Result<Vec<Role>, sqlx::Error> {
    let roles = sqlx::query_as!(
        Role,
        r#"
        SELECT id, name, description, created_at
        FROM Role
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(roles)
}

pub async fn get_role_by_id(pool: &PgPool, role_id: Uuid) -> Result<Option<Role>, sqlx::Error> {
    let role = sqlx::query_as!(
        Role,
        r#"
        SELECT id, name, description, created_at
        FROM Role
        WHERE id = $1
        "#,
        role_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(role)
}

pub async fn update_role(pool: &PgPool, role_id: Uuid, name: Option<&str>, description: Option<&str>) -> Result<Role, sqlx::Error> {
    let role = sqlx::query_as!(
        Role,
        r#"
        UPDATE Role
        SET name = COALESCE($1, name),
            description = COALESCE($2, description)
        WHERE id = $3
        RETURNING id, name, description, created_at
        "#,
        name,
        description,
        role_id
    )
    .fetch_one(pool)
    .await?;

    Ok(role)
}

pub async fn delete_role(pool: &PgPool, role_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM Role
        WHERE id = $1
        "#,
        role_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn test_create_role() {
        // Setup: Create a temporary database pool
        let pool = PgPoolOptions::new()
            .connect("postgres://nexus_admin:67945731797@localhost/nexus")
            .await
            .expect("Failed to connect to the database");

        // Test: Create a new role
        let role_name = "TestRole";
        let role_description = Some("A role for testing purposes");
        let role = create_role(&pool, role_name, role_description).await
            .expect("Failed to create role");

        assert_eq!(role.name, role_name);
        assert_eq!(role.description, role_description.map(|desc| desc.to_string()));

        // Cleanup: Delete the created role
        delete_role(&pool, role.id).await.expect("Failed to delete role");
    }
}