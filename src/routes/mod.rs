use axum::Router;

use crate::state::AppState;

mod auth;
mod health;
mod organization;

pub fn build(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/api/v1/auth", auth::router(state.clone()))
        .nest("/api/v1/organizations", organization::router(state))
        .nest("/health", health::router())
}
