use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use sqlx::PgPool;
use chrono::NaiveDate;
use serde::Deserialize;
mod models;
mod handlers;

use crate::handlers::create_user;
use handlers::create_tenant;
use crate::handlers::get_tenants;
use crate::handlers::get_users;

// Struct for handling JSON request data
#[derive(serde::Deserialize)]
struct TenantRequest {
    name: String,
    domain: Option<String>,
    address: String,
    contact_email: String,
    contact_phone: Option<String>,
    logo_url: Option<String>,
    timezone: String,
}

#[derive(Deserialize)]
struct UserRequest {
    email: String,
    password_hash: String,
    first_name: String,
    last_name: String,
    date_of_birth: Option<String>,  // As string in the request, later parsed to NaiveDate
    gender: Option<String>,
    contact_phone: Option<String>,
    address: Option<String>,
}

// Handler function for the `create_tenant` endpoint
async fn create_tenant_handler(
    pool: web::Data<PgPool>,
    tenant: web::Json<TenantRequest>,
) -> impl Responder {
    match create_tenant(
        &pool,
        &tenant.name,
        tenant.domain.as_deref(),
        &tenant.address,
        &tenant.contact_email,
        tenant.contact_phone.as_deref(),
        tenant.logo_url.as_deref(),
        &tenant.timezone,
    )
    .await
    {
        Ok(tenant_id) => HttpResponse::Ok().body(format!("Tenant created with ID: {}", tenant_id)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for fetching tenants
async fn get_tenants_handler(pool: web::Data<PgPool>) -> impl Responder {
    match get_tenants(&pool).await {
        Ok(tenants) => HttpResponse::Ok().json(tenants),  // Return tenants as JSON
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for creating a new user
async fn create_user_handler(
    pool: web::Data<PgPool>,
    user: web::Json<UserRequest>,
) -> impl Responder {
    // Convert date_of_birth to NaiveDate if provided
    let date_of_birth = if let Some(dob_str) = &user.date_of_birth {
        NaiveDate::parse_from_str(dob_str, "%Y-%m-%d").ok()
    } else {
        None
    };

    match create_user(
        &pool,
        &user.email,
        &user.password_hash,
        &user.first_name,
        &user.last_name,
        date_of_birth,
        user.gender.as_deref(),
        user.contact_phone.as_deref(),
        user.address.as_deref(),
    )
    .await
    {
        Ok(user_id) => HttpResponse::Ok().body(format!("User created with ID: {}", user_id)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to database");

    println!("Connected to the database!");

    // Start the HTTP server and bind it to port 8080
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))  // Share the DB pool with handlers
            .route("/create_tenant", web::post().to(create_tenant_handler)) // POST route for creating tenant
            .route("/tenants", web::get().to(get_tenants_handler))
            .route("/create_user", web::post().to(create_user_handler))
            .route("/users", web::get().to(get_users_handler))
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}


// Handler for fetching users
async fn get_users_handler(pool: web::Data<PgPool>) -> impl Responder {
    match get_users(&pool).await {
        Ok(users) => HttpResponse::Ok().json(users),  // Return users as JSON
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}
