use axum::middleware as axum_mw;
use axum::routing::post;
use axum::Router;

use crate::handlers::auth;
use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(auth::create_organization))
        .layer(axum_mw::from_fn_with_state(
            state,
            crate::middleware::auth::require_auth,
        ))
}
