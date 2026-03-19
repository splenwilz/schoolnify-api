use axum::http::{Method, StatusCode};
use serial_test::serial;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::jwt::*;
use super::common::workos_mocks::*;
use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_me_returns_user_profile() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    seed_user(&state.db_pool, &workos_id, &email).await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, body) = get_auth(app, "/api/v1/auth/me", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["email"], email);
}

#[tokio::test]
#[serial]
async fn test_me_without_auth_returns_401() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, _body) = send(app, Method::GET, "/api/v1/auth/me", None, vec![]).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_delete_account_success() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    seed_user(&state.db_pool, &workos_id, &email).await;

    mock_delete_user_success(&workos_id).mount(&mock_server).await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, body) = delete_auth(app, "/api/v1/auth/me", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Account deleted successfully");
}

#[tokio::test]
#[serial]
async fn test_delete_account_sole_admin_deletes_org() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = format!("del-org-{}", &workos_id[5..13]);

    let (_user_id, org_id) = seed_user_with_org(
        &state.db_pool, &workos_id, &email, "Delete Org School", &slug, &workos_org_id, "admin"
    ).await;

    mock_delete_org_success(&workos_org_id).mount(&mock_server).await;
    mock_delete_user_success(&workos_id).mount(&mock_server).await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, _body) = delete_auth(app, "/api/v1/auth/me", &token).await;

    assert_eq!(status, StatusCode::OK);

    let org = state.organization_service.find_by_id(org_id).await.unwrap();
    assert!(org.is_none());
}
