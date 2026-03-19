use axum::http::StatusCode;
use serial_test::serial;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::workos_mocks::*;
use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_verify_email_success_returns_200_with_tokens() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();

    let access = unique_token("access");
    let refresh = unique_token("refresh");
    mock_authenticate_email_verification_success(&workos_id, &email, &access, &refresh)
        .mount(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/api/v1/auth/verify-email",
        serde_json::json!({
            "code": "123456",
            "pending_authentication_token": "pnd_test_verify"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Email verified successfully");
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert_eq!(body["user"]["email"], email);
}

#[tokio::test]
#[serial]
async fn test_verify_email_invalid_code_returns_400() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/user_management/authenticate"))
        .and(wiremock::matchers::body_string_contains("email-verification:code"))
        .respond_with(wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "code": "invalid_code",
            "message": "The verification code is invalid or expired."
        })))
        .mount(&mock_server)
        .await;

    let (status, _body) = post_json(
        app,
        "/api/v1/auth/verify-email",
        serde_json::json!({
            "code": "000000",
            "pending_authentication_token": "pnd_test_invalid"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn test_resend_verification_returns_200() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    mock_send_verification_email_success().mount(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/api/v1/auth/resend-verification",
        serde_json::json!({
            "user_id": "user_test_123"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Verification email sent");
}
