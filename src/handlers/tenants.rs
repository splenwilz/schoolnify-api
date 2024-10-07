use actix_web::{web, HttpResponse, Responder};
use crate::services::tenant_service;
use uuid::Uuid;

#[derive(serde::Deserialize)]
struct UpdateTenantRequest {
    name: Option<String>,
    domain: Option<String>,
    address: Option<String>,
    contact_email: Option<String>,
    contact_phone: Option<String>,
    logo_url: Option<String>,
    timezone: Option<String>,
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/tenants")
            .route(web::get().to(get_tenants_handler))
            .route(web::post().to(create_tenant_handler))
    ) 
    .service(
        web::resource("/tenant/{id}")
            .route(web::get().to(get_tenant_by_id_handler)) 
            .route(web::put().to(update_tenant_handler))  // Update tenant
            .route(web::delete().to(delete_tenant_handler))  // Delete tenant by ID
    )
    .service(
        web::resource("/tenant/name/{name}")
            .route(web::get().to(get_tenant_by_name_handler))  // Get tenant by name
            .route(web::delete().to(delete_tenant_by_name_handler))  // Delete tenant by name
    )
    .service(
        web::resource("/tenant/domain/{domain}")
            .route(web::get().to(get_tenant_by_domain_handler))  // Get tenant by domain
            .route(web::delete().to(delete_tenant_by_domain_handler))  // Delete tenant by domain
    );
}


async fn create_tenant_handler(pool: web::Data<sqlx::PgPool>, tenant: web::Json<tenant_service::TenantRequest>) -> impl Responder {
    match tenant_service::create_tenant(&pool, &tenant).await {
        Ok(tenant_id) => HttpResponse::Ok().body(format!("Tenant created with ID BG: {}", tenant_id)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for fetching all tenants
async fn get_tenants_handler(pool: web::Data<sqlx::PgPool>) -> impl Responder {
    match tenant_service::get_tenants(&pool).await {
        Ok(tenants) => HttpResponse::Ok().json(tenants),  // Return tenants as JSON
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

async fn get_tenant_by_id_handler(
    pool: web::Data<sqlx::PgPool>,
    tenant_id: web::Path<Uuid>,
) -> impl Responder {
    match tenant_service::get_tenant_by_id(&pool, *tenant_id).await {
        Ok(Some(tenant)) => HttpResponse::Ok().json(tenant), // Return tenant as JSON
        Ok(None) => HttpResponse::NotFound().body("Tenant not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for fetching a tenant by name
async fn get_tenant_by_name_handler(
    pool: web::Data<sqlx::PgPool>,
    tenant_name: web::Path<String>,
) -> impl Responder {
    match tenant_service::get_tenant_by_name(&pool, &tenant_name).await {
        Ok(Some(tenant)) => HttpResponse::Ok().json(tenant), // Return tenant as JSON
        Ok(None) => HttpResponse::NotFound().body("Tenant not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}
// Handler for fetching a tenant by domain
async fn get_tenant_by_domain_handler(
    pool: web::Data<sqlx::PgPool>,
    tenant_domain: web::Path<String>,
) -> impl Responder {
    match tenant_service::get_tenant_by_domain(&pool, &tenant_domain).await {
        Ok(Some(tenant)) => HttpResponse::Ok().json(tenant), // Return tenant as JSON
        Ok(None) => HttpResponse::NotFound().body("Tenant not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}



// Handler for updating a tenant
async fn update_tenant_handler(
    pool: web::Data<sqlx::PgPool>,
    tenant_id: web::Path<Uuid>,
    tenant: web::Json<UpdateTenantRequest>,
) -> impl Responder {
    match tenant_service::update_tenant(
        &pool,
        *tenant_id,
        tenant.name.as_deref(),
        tenant.domain.as_deref(),
        tenant.address.as_deref(),
        tenant.contact_email.as_deref(),
        tenant.contact_phone.as_deref(),
        tenant.logo_url.as_deref(),
        tenant.timezone.as_deref(),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().body("Tenant updated successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for deleting a tenant by ID
async fn delete_tenant_handler(
    pool: web::Data<sqlx::PgPool>,
    tenant_id: web::Path<Uuid>,
) -> impl Responder {
    match tenant_service::delete_tenant(&pool, *tenant_id).await {
        Ok(_) => HttpResponse::Ok().body("Tenant deleted successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}


// Handler for deleting a tenant by name
async fn delete_tenant_by_name_handler(
    pool: web::Data<sqlx::PgPool>,
    tenant_name: web::Path<String>,
) -> impl Responder {
    match tenant_service::delete_tenant_by_name(&pool, &tenant_name).await {
        Ok(_) => HttpResponse::Ok().body("Tenant deleted successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}

// Handler for deleting a tenant by domain
async fn delete_tenant_by_domain_handler(
    pool: web::Data<sqlx::PgPool>,
    tenant_domain: web::Path<String>,
) -> impl Responder {
    match tenant_service::delete_tenant_by_domain(&pool, &tenant_domain).await {
        Ok(_) => HttpResponse::Ok().body("Tenant deleted successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error: {}", err)),
    }
}





