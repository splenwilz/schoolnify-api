use axum::http::StatusCode;
use serial_test::serial;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::jwt::*;
use super::common::workos_mocks::*;
use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_admin_signup_verification_required_returns_202() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let email = unique_email();
    let workos_id = unique_workos_id();

    mock_create_user_success(&email, &workos_id).mount(&mock_server).await;
    mock_authenticate_password_needs_verification("pnd_admin").mount(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/api/v1/auth/admin-signup",
        serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "first_name": "Admin",
            "last_name": "User",
            "school_name": "Test Academy"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["pending_authentication_token"], "pnd_admin");
    assert_eq!(body["school_name"], "Test Academy");
}

#[tokio::test]
#[serial]
async fn test_create_organization_success_returns_201() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let user_id = seed_user(&state.db_pool, &workos_id, &email).await;
    let raw_refresh = seed_refresh_token(&state.db_pool, user_id).await;

    mock_create_org_success("New School", &workos_org_id).mount(&mock_server).await;
    mock_create_membership_success().mount(&mock_server).await;
    let new_access = unique_token("access");
    let new_refresh = unique_token("refresh");
    mock_refresh_with_org_success(&workos_id, &email, &new_access, &new_refresh)
        .mount(&mock_server).await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, body) = post_json_auth(
        app,
        "/api/v1/auth/create-organization",
        serde_json::json!({
            "school_name": "New School",
            "refresh_token": raw_refresh
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["organization"]["name"], "New School");
    assert!(body["subdomain_url"].is_string());
}

#[tokio::test]
#[serial]
async fn test_create_organization_already_in_org_returns_409() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("existing");

    seed_user_with_org(&state.db_pool, &workos_id, &email, "Existing School", &slug, &workos_org_id, "admin").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, _body) = post_json_auth(
        app,
        "/api/v1/auth/create-organization",
        serde_json::json!({
            "school_name": "Another School"
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
#[serial]
async fn test_establish_session_success() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("session");

    seed_user_with_org(&state.db_pool, &workos_id, &email, "Session School", &slug, &workos_org_id, "admin").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, body) = post_json_auth(
        app,
        "/api/v1/auth/establish-session",
        serde_json::json!({
            "organization_slug": slug
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Session established");
}

#[tokio::test]
#[serial]
async fn test_establish_session_wrong_org_returns_403() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();

    seed_user(&state.db_pool, &workos_id, &email).await;

    let other_org_id = unique_workos_org_id();
    let other_slug = unique_slug("other");
    sqlx::query("INSERT INTO organizations (workos_org_id, name, slug) VALUES ($1, $2, $3)")
        .bind(&other_org_id)
        .bind("Other School")
        .bind(&other_slug)
        .execute(&state.db_pool)
        .await
        .unwrap();

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, _body) = post_json_auth(
        app,
        "/api/v1/auth/establish-session",
        serde_json::json!({
            "organization_slug": other_slug
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_establish_session_unknown_slug_returns_404() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    seed_user(&state.db_pool, &workos_id, &email).await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, _body) = post_json_auth(
        app,
        "/api/v1/auth/establish-session",
        serde_json::json!({
            "organization_slug": "nonexistent-school"
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}
