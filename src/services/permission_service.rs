use sqlx::PgPool;
use uuid::Uuid;
use crate::models::permission::Permission;

pub async fn create_permission(pool: &PgPool, code: &str, description: Option<&str>) -> Result<Permission, sqlx::Error> {
    let permission = sqlx::query_as!(
        Permission,
        r#"
        INSERT INTO Permission (id, code, description)
        VALUES ($1, $2, $3)
        RETURNING id, code, description, created_at
        "#,
        Uuid::new_v4(),
        code,
        description
    )
    .fetch_one(pool)
    .await?;

    Ok(permission)
}

pub async fn get_permissions(pool: &PgPool) -> Result<Vec<Permission>, sqlx::Error> {
    let permissions = sqlx::query_as!(
        Permission,
        r#"
        SELECT id, code, description, created_at
        FROM Permission
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(permissions)
}

pub async fn get_permission_by_id(pool: &PgPool, permission_id: Uuid) -> Result<Option<Permission>, sqlx::Error> {
    let permission = sqlx::query_as!(
        Permission,
        r#"
        SELECT id, code, description, created_at
        FROM Permission
        WHERE id = $1
        "#,
        permission_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(permission)
}

pub async fn update_permission(pool: &PgPool, permission_id: Uuid, code: Option<&str>, description: Option<&str>) -> Result<Permission, sqlx::Error> {
    let permission = sqlx::query_as!(
        Permission,
        r#"
        UPDATE Permission
        SET code = COALESCE($1, code),
            description = COALESCE($2, description)
        WHERE id = $3
        RETURNING id, code, description, created_at
        "#,
        code,
        description,
        permission_id
    )
    .fetch_one(pool)
    .await?;

    Ok(permission)
}

pub async fn delete_permission(pool: &PgPool, permission_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM Permission
        WHERE id = $1
        "#,
        permission_id
    )
    .execute(pool)
    .await?;

    Ok(())
}