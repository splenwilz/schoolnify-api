use axum::Router;

use crate::state::AppState;

mod auth;
mod health;
mod schools;
mod students;

pub fn build(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/api/v1/auth", auth::router(state.clone()))
        .nest("/api/v1/schools", schools::router(state.clone()))
        .nest("/api/v1/students", students::router(state))
        .nest("/health", health::router())
}
