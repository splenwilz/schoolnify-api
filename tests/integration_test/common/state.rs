use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use schoolnify_api::state::AppState;
use tower::ServiceExt;
use wiremock::MockServer;

use super::config::test_config;
use super::db::setup_test_db;

/// Build a test AppState using the wiremock server as WorkOS backend.
pub async fn test_app_state(mock_server: &MockServer) -> AppState {
    let pool = setup_test_db().await;
    let config = test_config(&mock_server.uri());
    AppState::new(config, pool)
}

/// Build a Router from an AppState.
pub fn test_router(state: AppState) -> Router {
    schoolnify_api::build_router(state)
}

/// Build a Router from a MockServer (convenience).
pub async fn test_app(mock_server: &MockServer) -> Router {
    let state = test_app_state(mock_server).await;
    test_router(state)
}

/// Send a request and return (status, json body).
pub async fn send(
    app: Router,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
    headers: Vec<(&str, &str)>,
) -> (StatusCode, serde_json::Value) {
    let body_str = body.map(|b| serde_json::to_string(&b).unwrap()).unwrap_or_default();

    let mut builder = Request::builder().method(method).uri(uri);

    if !body_str.is_empty() {
        builder = builder.header("content-type", "application/json");
    }

    for (name, value) in &headers {
        builder = builder.header(*name, *value);
    }

    let request = builder.body(Body::from(body_str)).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = if bytes.is_empty() {
        serde_json::json!(null)
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!("Failed to parse response as JSON (status={status}): {e}\nBody: {}", String::from_utf8_lossy(&bytes))
        })
    };

    (status, json)
}

/// Convenience: POST with JSON body.
pub async fn post_json(
    app: Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    send(app, Method::POST, uri, Some(body), vec![]).await
}

/// Convenience: POST with JSON body and Bearer token.
pub async fn post_json_auth(
    app: Router,
    uri: &str,
    body: serde_json::Value,
    token: &str,
) -> (StatusCode, serde_json::Value) {
    let auth = format!("Bearer {token}");
    send(app, Method::POST, uri, Some(body), vec![("authorization", &auth)]).await
}

/// Convenience: GET with Bearer token.
pub async fn get_auth(
    app: Router,
    uri: &str,
    token: &str,
) -> (StatusCode, serde_json::Value) {
    let auth = format!("Bearer {token}");
    send(app, Method::GET, uri, None, vec![("authorization", &auth)]).await
}

/// Convenience: DELETE with Bearer token.
pub async fn delete_auth(
    app: Router,
    uri: &str,
    token: &str,
) -> (StatusCode, serde_json::Value) {
    let auth = format!("Bearer {token}");
    send(app, Method::DELETE, uri, None, vec![("authorization", &auth)]).await
}

/// Convenience: POST with a cookie.
pub async fn post_with_cookie(
    app: Router,
    uri: &str,
    cookie_name: &str,
    cookie_value: &str,
) -> (StatusCode, serde_json::Value) {
    let cookie = format!("{cookie_name}={cookie_value}");
    send(app, Method::POST, uri, None, vec![("cookie", &cookie)]).await
}
