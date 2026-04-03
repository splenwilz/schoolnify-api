use axum::http::StatusCode;
use serial_test::serial;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::workos_mocks::*;
use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_signup_with_email_verification_returns_201() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();
    let pending_token = "pnd_test_signup";

    mock_create_user_success(&email, &workos_id).mount(&mock_server).await;
    mock_authenticate_password_needs_verification(pending_token).mount(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/api/v1/auth/signup",
        serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "first_name": "Test",
            "last_name": "User"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["pending_authentication_token"], pending_token);
    assert!(body["message"].as_str().unwrap().contains("verification"));
}

#[tokio::test]
#[serial]
async fn test_signup_without_verification_returns_201_with_tokens() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();

    mock_create_user_success(&email, &workos_id).mount(&mock_server).await;
    let access = unique_token("access");
    let refresh = unique_token("refresh");
    mock_authenticate_password_success(&workos_id, &email, &access, &refresh)
        .mount(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/api/v1/auth/signup",
        serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "first_name": "Test",
            "last_name": "User"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["message"], "Account created successfully");
    assert!(body["access_token"].is_string());
    assert!(body["user"]["email"].as_str().unwrap() == email);
}

#[tokio::test]
#[serial]
async fn test_signup_duplicate_email_returns_409() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    mock_create_user_conflict().mount(&mock_server).await;

    let (status, _body) = post_json(
        app,
        "/api/v1/auth/signup",
        serde_json::json!({
            "email": "duplicate@example.com",
            "password": "SecurePass123!"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
}
