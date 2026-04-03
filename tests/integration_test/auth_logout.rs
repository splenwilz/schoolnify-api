use axum::http::{Method, StatusCode};
use serial_test::serial;
use wiremock::MockServer;

use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_logout_returns_200() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, body) = send(app, Method::POST, "/api/v1/auth/logout", None, vec![]).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Logged out successfully");
}
