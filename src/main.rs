use actix_web::{web, App, HttpServer, middleware::Logger};
use crate::middlewares::auth_middleware::AuthMiddleware;
use crate::handlers::auth::{login_handler, refresh_token_handler, logout_handler};
use crate::handlers::role::{create_role_handler, get_roles_handler, update_role_handler, get_role_by_id_handler, delete_role_handler};
use crate::handlers::permission::{create_permission_handler, get_permissions_handler, get_permission_by_id_handler, update_permission_handler, delete_permission_handler};

mod middlewares;
mod config;
mod handlers;
mod models;
mod services;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let pool = config::create_db_pool().await;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))  // Share DB pool
            .wrap(Logger::default())                
             // Public routes (no AuthMiddleware applied)
            .service(
                web::resource("/login").route(web::post().to(login_handler))  // Unprotected login route
            )
            .service(
                web::resource("/refresh_token").route(web::post().to(refresh_token_handler))  // Refresh token route
            )
            .service(
                web::resource("/logout").route(web::post().to(logout_handler))  // Logout route
            )
            // Role management routes
            .service(
                web::scope("/roles")
                    // Apply AuthMiddleware to protect these routes
                    .wrap(AuthMiddleware)
                    .route("", web::post().to(create_role_handler))
                    .route("", web::get().to(get_roles_handler))
                    .route("/{id}", web::get().to(get_role_by_id_handler))
                    .route("/{id}", web::put().to(update_role_handler))
                    .route("/{id}", web::delete().to(delete_role_handler))
            )
            // Permission management routes
            .service(
                web::scope("/permissions")
                    .wrap(AuthMiddleware)  // Protect these routes
                    .route("", web::post().to(create_permission_handler))  // POST /permissions
                    .route("", web::get().to(get_permissions_handler))    // GET /permissions
                    .route("/{id}", web::get().to(get_permission_by_id_handler))  // GET /permissions/{id}
                    .route("/{id}", web::put().to(update_permission_handler))      // PUT /permissions/{id}
                    .route("/{id}", web::delete().to(delete_permission_handler))  // DELETE /permissions/{id}
            )          

            // Protected routes with AuthMiddleware applied
            .service(
                web::scope("")  // Apply AuthMiddleware to everything else
                    .wrap(AuthMiddleware)
                    .configure(handlers::init_routes)  // Configure protected routes
            )
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    
    .await
}


// For without AuthMiddleware
// mod config;
// mod handlers;
// mod models;
// mod services;


// use actix_web::{web, App, HttpServer};

// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     dotenv::dotenv().ok();

//     let pool = config::create_db_pool().await;

//     HttpServer::new(move || {
//         App::new()
//             .app_data(web::Data::new(pool.clone()))  // Share DB pool
//             .configure(handlers::init_routes)  // Configure all routes
//     })
//     .bind(("127.0.0.1", 8081))?
//     .run()
//     .await
// }
