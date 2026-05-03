use axum::middleware as axum_mw;
use axum::routing::{get, patch, post};
use axum::Router;

use crate::handlers::students;
use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(students::list_students).post(students::create_student),
        )
        .route("/promote", post(students::promote))
        .route("/bulk-import", post(students::bulk_import))
        .route("/export", get(students::export))
        .route(
            "/{id}",
            get(students::get_student)
                .patch(students::patch_student)
                .delete(students::delete_student),
        )
        .route("/{id}/status", patch(students::change_status))
        .route("/{id}/class", patch(students::change_class))
        .layer(axum_mw::from_fn_with_state(
            state,
            crate::middleware::auth::require_auth,
        ))
}
