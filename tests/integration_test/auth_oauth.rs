use axum::http::{Method, StatusCode};
use serial_test::serial;
use wiremock::MockServer;

use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_authorize_returns_url() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, body) = send(
        app,
        Method::GET,
        "/api/v1/auth/authorize?provider=GoogleOAuth",
        None,
        vec![],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["authorization_url"].is_string());
}

#[tokio::test]
#[serial]
async fn test_callback_without_state_cookie_returns_400() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, _body) = send(
        app,
        Method::GET,
        "/api/v1/auth/callback?code=test_code",
        None,
        vec![],
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}
