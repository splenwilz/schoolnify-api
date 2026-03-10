use axum::middleware as axum_mw;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::auth;
use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    let public = Router::new()
        .route("/signup", post(auth::signup))
        .route("/verify-email", post(auth::verify_email))
        .route("/resend-verification", post(auth::resend_verification))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/refresh", post(auth::refresh))
        .route("/authorize", get(auth::authorize))
        .route("/callback", get(auth::callback))
        .route("/admin-signup", post(auth::admin_signup));

    let protected = Router::new()
        .route("/me", get(auth::me).delete(auth::delete_account))
        .route("/create-organization", post(auth::create_organization))
        .layer(axum_mw::from_fn_with_state(
            state,
            crate::middleware::auth::require_auth,
        ));

    public.merge(protected)
}
