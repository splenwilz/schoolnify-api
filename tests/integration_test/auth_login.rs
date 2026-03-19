use axum::http::StatusCode;
use serial_test::serial;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::jwt::*;
use super::common::workos_mocks::*;
use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_login_success_returns_200_with_user() {
    let mock_server = MockServer::start().await;
    let state = test_app_state(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();
    seed_user(&state.db_pool, &workos_id, &email).await;

    let access_token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let refresh = unique_token("refresh");
    mock_authenticate_password_success(&workos_id, &email, &access_token, &refresh)
        .mount(&mock_server).await;

    let app = test_router(state.clone());

    let (status, body) = post_json(
        app,
        "/api/v1/auth/login",
        serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Login successful");
    assert_eq!(body["user"]["email"], email);
    assert!(body["access_token"].is_string());
}

#[tokio::test]
#[serial]
async fn test_login_invalid_credentials_returns_401() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    mock_authenticate_password_invalid().mount(&mock_server).await;

    let (status, _body) = post_json(
        app,
        "/api/v1/auth/login",
        serde_json::json!({
            "email": "wrong@example.com",
            "password": "WrongPass123!"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_login_verification_required_returns_403() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    mock_authenticate_password_needs_verification("pnd_test_login")
        .mount(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/api/v1/auth/login",
        serde_json::json!({
            "email": "unverified@example.com",
            "password": "SecurePass123!"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body["pending_authentication_token"].is_string());
}

#[tokio::test]
#[serial]
async fn test_login_returns_subdomain_url_when_user_has_org() {
    let mock_server = MockServer::start().await;
    let state = test_app_state(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = format!("test-school-{}", &workos_id[5..13]);

    seed_user_with_org(&state.db_pool, &workos_id, &email, "Test School", &slug, &workos_org_id, "admin").await;

    let access_token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let refresh = unique_token("refresh");
    mock_authenticate_password_success(&workos_id, &email, &access_token, &refresh)
        .mount(&mock_server).await;

    let app = test_router(state.clone());

    let (status, body) = post_json(
        app,
        "/api/v1/auth/login",
        serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["subdomain_url"].is_string());
    assert!(body["subdomain_url"].as_str().unwrap().contains(&slug));
}
