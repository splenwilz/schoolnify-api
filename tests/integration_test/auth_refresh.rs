use axum::http::{Method, StatusCode};
use serial_test::serial;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::workos_mocks::*;
use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_refresh_success_returns_200() {
    let mock_server = MockServer::start().await;
    let state = test_app_state(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();
    let user_id = seed_user(&state.db_pool, &workos_id, &email).await;
    let raw_refresh = seed_refresh_token(&state.db_pool, user_id).await;

    let new_access = unique_token("access");
    let new_refresh = unique_token("refresh");
    mock_refresh_token_success(&workos_id, &email, &new_access, &new_refresh)
        .mount(&mock_server).await;

    let app = test_router(state.clone());

    let (status, body) = post_with_cookie(app, "/api/v1/auth/refresh", "refresh_token", &raw_refresh).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Token refreshed");
}

#[tokio::test]
#[serial]
async fn test_refresh_without_cookie_returns_401() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, _body) = send(app, Method::POST, "/api/v1/auth/refresh", None, vec![]).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_refresh_deactivated_user_returns_403() {
    let mock_server = MockServer::start().await;
    let state = test_app_state(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();
    let user_id = seed_user(&state.db_pool, &workos_id, &email).await;
    let raw_refresh = seed_refresh_token(&state.db_pool, user_id).await;

    sqlx::query("UPDATE users SET is_active = false WHERE id = $1")
        .bind(user_id)
        .execute(&state.db_pool)
        .await
        .expect("Failed to deactivate test user");

    let app = test_router(state.clone());

    let (status, _body) = post_with_cookie(app, "/api/v1/auth/refresh", "refresh_token", &raw_refresh).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}
