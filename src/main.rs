use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use sqlx::PgPool;

mod models;
mod handlers;

use handlers::create_tenant;

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
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
