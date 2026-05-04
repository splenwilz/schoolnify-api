use axum::middleware as axum_mw;
use axum::routing::get;
use axum::Router;
use tower_http::limit::RequestBodyLimitLayer;

use crate::handlers::school_setup;
use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    let public = Router::new()
        .route("/{slug}/public", get(school_setup::get_public_branding));

    let protected = Router::new()
        .route(
            "/setup",
            get(school_setup::get_setup).patch(school_setup::patch_setup),
        )
        .layer(axum_mw::from_fn_with_state(
            state,
            crate::middleware::auth::require_auth,
        ));

    public
        .merge(protected)
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
}
