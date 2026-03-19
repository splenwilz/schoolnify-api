use axum::http::{Method, StatusCode};
use serial_test::serial;
use wiremock::MockServer;

use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_health_returns_200_when_db_healthy() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, body) = send(app, Method::GET, "/health", None, vec![]).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["checks"]["database"], "up");
}

#[tokio::test]
#[serial]
async fn test_health_response_includes_version() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, body) = send(app, Method::GET, "/health", None, vec![]).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["version"].is_string());
    assert!(!body["version"].as_str().unwrap().is_empty());
}
