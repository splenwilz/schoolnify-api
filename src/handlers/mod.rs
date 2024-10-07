mod tenants;
mod users;

use actix_web::web;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    tenants::init_routes(cfg);
    users::init_routes(cfg);
}
