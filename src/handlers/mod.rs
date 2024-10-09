mod tenants;
pub mod users;
pub mod auth;
pub mod role;
pub mod permission;



use actix_web::web;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    tenants::init_routes(cfg);
    users::init_routes(cfg);
}