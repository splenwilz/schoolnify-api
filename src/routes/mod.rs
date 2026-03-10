use axum::Router;

use crate::state::AppState;

mod auth;
mod health;

pub fn build(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/api/v1/auth", auth::router(state))
        .nest("/health", health::router())
}
