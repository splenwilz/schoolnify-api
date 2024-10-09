use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use crate::services::permission_service::{create_permission, get_permissions};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreatePermissionRequest {
    pub code: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct GetPermissionByIdPath {
    id: Uuid,
}

#[derive(Deserialize)]
pub struct UpdatePermissionRequest {
    pub code: Option<String>,
    pub description: Option<String>,
}

// Handler for creating a new permission
pub async fn create_permission_handler(
    pool: web::Data<sqlx::PgPool>,
    request: web::Json<CreatePermissionRequest>,
) -> impl Responder {
    // Optionally, add authorization checks here to ensure only authorized users can create permissions

    match create_permission(
        pool.get_ref(),
        &request.code,
        request.description.as_deref(),
    )
    .await
    {
        Ok(permission) => HttpResponse::Created().json(permission),
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("Permission_code_key") => {
            HttpResponse::BadRequest().body("Permission with this code already exists")
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to create permission"),
    }
}

// Handler for retrieving all permissions
pub async fn get_permissions_handler(
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    match get_permissions(pool.get_ref()).await {
        Ok(permissions) => HttpResponse::Ok().json(permissions),
        Err(_) => HttpResponse::InternalServerError().body("Failed to retrieve permissions"),
    }
}

// Retrieve a permission by ID
pub async fn get_permission_by_id_handler(
    pool: web::Data<sqlx::PgPool>,
    path: web::Path<GetPermissionByIdPath>,
) -> impl Responder {
    let permission_id = path.id;

    match crate::services::permission_service::get_permission_by_id(pool.get_ref(), permission_id).await {
        Ok(Some(permission)) => HttpResponse::Ok().json(permission),
        Ok(None) => HttpResponse::NotFound().body("Permission not found"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to retrieve permission"),
    }
}

// Handler for updating a permission
pub async fn update_permission_handler(
    pool: web::Data<sqlx::PgPool>,
    path: web::Path<GetPermissionByIdPath>,
    request: web::Json<UpdatePermissionRequest>,
) -> impl Responder {
    let permission_id = path.id;

    match crate::services::permission_service::update_permission(
        pool.get_ref(),
        permission_id,
        request.code.as_deref(),
        request.description.as_deref(),
    ).await {
        Ok(permission) => HttpResponse::Ok().json(permission),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().body("Permission not found"),
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("Permission_code_key") => {
            HttpResponse::BadRequest().body("Permission with this code already exists")
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to update permission"),
    }
}

// Handler for deleting a permission
pub async fn delete_permission_handler(
    pool: web::Data<sqlx::PgPool>,
    path: web::Path<GetPermissionByIdPath>,
) -> impl Responder {
    let permission_id = path.id;

    match crate::services::permission_service::delete_permission(pool.get_ref(), permission_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().body("Permission not found"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to delete permission"),
    }
}