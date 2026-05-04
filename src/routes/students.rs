use axum::middleware as axum_mw;
use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::limit::RequestBodyLimitLayer;

use crate::handlers::students;
use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    // Bulk import accepts CSV uploads — up to ~5000 rows plus multipart overhead.
    let upload = Router::new()
        .route("/bulk-import", post(students::bulk_import))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024));

    // All other student endpoints have small JSON bodies; keep them at 1MB.
    let standard = Router::new()
        .route(
            "/",
            get(students::list_students).post(students::create_student),
        )
        .route("/promote", post(students::promote))
        .route("/export", get(students::export))
        .route(
            "/{id}",
            get(students::get_student)
                .patch(students::patch_student)
                .delete(students::delete_student),
        )
        .route("/{id}/status", patch(students::change_status))
        .route("/{id}/class", patch(students::change_class))
        .layer(RequestBodyLimitLayer::new(1024 * 1024));

    standard.merge(upload).layer(axum_mw::from_fn_with_state(
        state,
        crate::middleware::auth::require_auth,
    ))
}
