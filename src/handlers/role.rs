use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use crate::services::role_service::{create_role, update_role, delete_role, get_role_by_id};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct GetRoleByIdPath {
    id: Uuid,
}

// Handler for creating a new role
pub async fn create_role_handler(
    pool: web::Data<sqlx::PgPool>,
    request: web::Json<CreateRoleRequest>,
) -> impl Responder {
    
    match create_role(pool.get_ref(), &request.name, request.description.as_deref()).await {
        Ok(role) => HttpResponse::Created().json(role),
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("Role_name_key") => {
            HttpResponse::BadRequest().body("Role with this name already exists")
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to create role"),
    }
}

// Handler for retrieving all roles
pub async fn get_roles_handler(
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    match crate::services::role_service::get_roles(pool.get_ref()).await {
        Ok(roles) => HttpResponse::Ok().json(roles),
        Err(_) => HttpResponse::InternalServerError().body("Failed to retrieve roles"),
    }
}

pub async fn get_role_by_id_handler(
    pool: web::Data<sqlx::PgPool>,
    path: web::Path<GetRoleByIdPath>,
) -> impl Responder {
    let role_id = path.id;

    match get_role_by_id(pool.get_ref(), role_id).await {
        Ok(Some(role)) => HttpResponse::Ok().json(role),
        Ok(None) => HttpResponse::NotFound().body("Role not found"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to retrieve role"),
    }
}

// Handler for updating an existing role
pub async fn update_role_handler(
    pool: web::Data<sqlx::PgPool>,
    path: web::Path<Uuid>,  // Role ID passed as a path parameter
    request: web::Json<UpdateRoleRequest>,
) -> impl Responder {
    let role_id = path.into_inner();
    let update_data = request.into_inner();

    match update_role(
        pool.get_ref(),
        role_id,
        update_data.name.as_deref(),
        update_data.description.as_deref(),
    )
    .await
    {
        Ok(role) => HttpResponse::Ok().json(role),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().body("Role not found"),
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("Role_name_key") => {
            HttpResponse::BadRequest().body("Role with this name already exists")
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to update role"),
    }
}

// Handler for deleting a role
pub async fn delete_role_handler(
    pool: web::Data<sqlx::PgPool>,
    path: web::Path<Uuid>,  // Role ID passed as a path parameter
) -> impl Responder {
    let role_id = path.into_inner();

    match delete_role(pool.get_ref(), role_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().body("Role not found"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to delete role"),
    }
}