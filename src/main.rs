use actix_web::{web, App, HttpServer, middleware::Logger};
use crate::middlewares::auth_middleware::AuthMiddleware;
use crate::handlers::auth::{login_handler, refresh_token_handler, logout_handler};
// use crate::handlers::logout_handler;
// use crate::handlers::auth::logout_handler;
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
