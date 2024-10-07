use actix_web::{web, HttpResponse, Responder};
use crate::services::user_service;
use uuid::Uuid;

#[derive(serde::Deserialize)]
struct UpdateUserRequest {
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    contact_phone: Option<String>,
    address: Option<String>,
}

// Initialize all routes for users
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/create_user")
            .route(web::post().to(create_user_handler))  // Create user
    )
    .service(
        web::resource("/users")
            .route(web::get().to(get_users_handler))  // Get all users
    )
    .service(
        web::resource("/user/{id}")
            .route(web::get().to(get_user_by_id_handler))  // Get user by ID
            .route(web::put().to(update_user_handler))  // Update user by ID
            .route(web::delete().to(delete_user_handler))  // Delete user by ID
    )
    .service(
        web::resource("/user/email/{email}")
            .route(web::get().to(get_user_by_email_handler))  // Get user by email
    );
}


// Handler for creating a new user
async fn create_user_handler(pool: web::Data<sqlx::PgPool>, user: web::Json<user_service::UserRequest>) -> impl Responder {
    match user_service::create_user(&pool, &user).await {
        Ok(user_id) => HttpResponse::Ok().body(format!("User created with ID: {}", user_id)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for fetching users
async fn get_users_handler(pool: web::Data<sqlx::PgPool>) -> impl Responder {
    match user_service::get_users(&pool).await {
        Ok(users) => HttpResponse::Ok().json(users),  // Return users as JSON
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for fetching a user by ID
async fn get_user_by_id_handler(
    pool: web::Data<sqlx::PgPool>,
    user_id: web::Path<Uuid>,
) -> impl Responder {
    match user_service::get_user_by_id(&pool, *user_id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user), // Return user as JSON
        Ok(None) => HttpResponse::NotFound().body("User not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for fetching a user by email
async fn get_user_by_email_handler(
    pool: web::Data<sqlx::PgPool>,
    user_email: web::Path<String>,
) -> impl Responder {
    match user_service::get_user_by_email(&pool, &user_email).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),  // Return user as JSON
        Ok(None) => HttpResponse::NotFound().body("User not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for updating a user
async fn update_user_handler(
    pool: web::Data<sqlx::PgPool>,
    user_id: web::Path<Uuid>,
    user: web::Json<UpdateUserRequest>,
) -> impl Responder {
    match user_service::update_user(
        &pool,
        *user_id,
        user.email.as_deref(),
        user.first_name.as_deref(),
        user.last_name.as_deref(),
        user.contact_phone.as_deref(),
        user.address.as_deref(),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().body("User updated successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for deleting a user by ID
async fn delete_user_handler(
    pool: web::Data<sqlx::PgPool>,
    user_id: web::Path<Uuid>,
) -> impl Responder {
    match user_service::delete_user(&pool, *user_id).await {
        Ok(_) => HttpResponse::Ok().body("User deleted successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}


