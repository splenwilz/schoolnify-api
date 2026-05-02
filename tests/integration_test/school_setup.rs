use axum::http::{Method, StatusCode};
use serial_test::serial;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::jwt::*;
use super::common::state::*;

#[tokio::test]
#[serial]
async fn test_get_setup_returns_null_when_no_setup() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("setup");

    seed_user_with_org(&state.db_pool, &workos_id, &email, "Setup School", &slug, &workos_org_id, "admin").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, body) = get_auth(app, "/api/v1/schools/setup", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_null());
    assert_eq!(body["completion"]["total_sections"], 12);
    assert_eq!(body["completion"]["completed_sections"], 0);
}

#[tokio::test]
#[serial]
async fn test_patch_setup_saves_partial_data() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("patch");

    seed_user_with_org(&state.db_pool, &workos_id, &email, "Patch School", &slug, &workos_org_id, "admin").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, body) = patch_json_auth(
        app,
        "/api/v1/schools/setup",
        serde_json::json!({
            "identity": { "school_type": "secondary", "motto": "Learn well" }
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["identity"]["school_type"], "secondary");
    assert_eq!(body["data"]["identity"]["motto"], "Learn well");
    assert_eq!(body["completion"]["completed_sections"], 1);
}

#[tokio::test]
#[serial]
async fn test_patch_setup_merges_with_existing() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("merge");

    let (_user_id, org_id) = seed_user_with_org(
        &state.db_pool, &workos_id, &email, "Merge School", &slug, &workos_org_id, "admin"
    ).await;

    // Seed existing data
    seed_school_setup(&state.db_pool, org_id, serde_json::json!({
        "identity": { "school_type": "primary", "motto": "Old motto" }
    })).await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    // PATCH branding — should NOT overwrite identity
    let (status, body) = patch_json_auth(
        app,
        "/api/v1/schools/setup",
        serde_json::json!({
            "branding": { "primary_color": "#0891B2", "secondary_color": "#10B981" }
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    // Identity still present
    assert_eq!(body["data"]["identity"]["school_type"], "primary");
    // Branding added
    assert_eq!(body["data"]["branding"]["primary_color"], "#0891B2");
    assert_eq!(body["completion"]["completed_sections"], 2);
}

#[tokio::test]
#[serial]
async fn test_patch_setup_forbidden_for_non_admin() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("nonadmin");

    // Seed user with "user" role, not "admin"
    seed_user_with_org(&state.db_pool, &workos_id, &email, "NonAdmin School", &slug, &workos_org_id, "user").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, _body) = patch_json_auth(
        app,
        "/api/v1/schools/setup",
        serde_json::json!({"identity": {"motto": "test"}}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_patch_setup_requires_auth() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, _body) = send(
        app,
        Method::PATCH,
        "/api/v1/schools/setup",
        Some(serde_json::json!({"identity": {"motto": "test"}})),
        vec![("content-type", "application/json")],
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_get_setup_requires_auth() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, _body) = send(app, Method::GET, "/api/v1/schools/setup", None, vec![]).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_get_setup_available_to_non_admin() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("reader");

    // user role, not admin
    seed_user_with_org(&state.db_pool, &workos_id, &email, "Reader School", &slug, &workos_org_id, "user").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, _body) = get_auth(app, "/api/v1/schools/setup", &token).await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn test_get_public_branding_returns_branding() {
    let mock_server = MockServer::start().await;
    let state = test_app_state(&mock_server).await;

    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("public");

    // Seed org and setup data directly
    let org_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO organizations (workos_org_id, name, slug) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&workos_org_id)
    .bind("Public Branding School")
    .bind(&slug)
    .fetch_one(&state.db_pool)
    .await
    .unwrap();

    seed_school_setup(&state.db_pool, org_id, serde_json::json!({
        "identity": { "motto": "Excellence", "logo_url": "https://example.com/logo.png" },
        "branding": { "primary_color": "#FF0000", "secondary_color": "#00FF00" }
    })).await;

    let app = test_router(state.clone());

    let (status, body) = send(
        app,
        Method::GET,
        &format!("/api/v1/schools/{slug}/public"),
        None,
        vec![],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Public Branding School");
    assert_eq!(body["slug"], slug);
    assert_eq!(body["motto"], "Excellence");
    assert_eq!(body["logo_url"], "https://example.com/logo.png");
    assert_eq!(body["primary_color"], "#FF0000");
    assert_eq!(body["secondary_color"], "#00FF00");
}

#[tokio::test]
#[serial]
async fn test_get_public_branding_404_for_unknown_slug() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, _body) = send(
        app,
        Method::GET,
        "/api/v1/schools/nonexistent-school/public",
        None,
        vec![],
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn test_patch_setup_rejects_non_object_body() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("reject");

    seed_user_with_org(&state.db_pool, &workos_id, &email, "Reject School", &slug, &workos_org_id, "admin").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    // Send an array instead of an object
    let (status, _body) = patch_json_auth(
        app,
        "/api/v1/schools/setup",
        serde_json::json!([1, 2, 3]),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn test_patch_setup_idempotent() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("idempotent");

    seed_user_with_org(&state.db_pool, &workos_id, &email, "Idempotent School", &slug, &workos_org_id, "admin").await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let payload = serde_json::json!({"identity": {"school_type": "secondary", "motto": "Same"}});

    // First PATCH
    let app1 = test_router(state.clone());
    let (status1, body1) = patch_json_auth(app1, "/api/v1/schools/setup", payload.clone(), &token).await;
    assert_eq!(status1, StatusCode::OK);

    // Second identical PATCH
    let app2 = test_router(state.clone());
    let (status2, body2) = patch_json_auth(app2, "/api/v1/schools/setup", payload, &token).await;
    assert_eq!(status2, StatusCode::OK);

    // Data should be identical
    assert_eq!(body1["data"], body2["data"]);
}

#[tokio::test]
#[serial]
async fn test_get_setup_completion_reflects_filled_sections() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;

    let state = test_app_state(&mock_server).await;
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("complete");

    let (_user_id, org_id) = seed_user_with_org(
        &state.db_pool, &workos_id, &email, "Complete School", &slug, &workos_org_id, "admin"
    ).await;

    seed_school_setup(&state.db_pool, org_id, serde_json::json!({
        "identity": { "school_type": "secondary", "motto": "Learn" },
        "branding": { "primary_color": "#000", "secondary_color": "#FFF" },
        "location": { "country": "Nigeria", "timezone": "Africa/Lagos" }
    })).await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());
    let app = test_router(state.clone());

    let (status, body) = get_auth(app, "/api/v1/schools/setup", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["completion"]["completed_sections"], 3);
    assert_eq!(body["completion"]["total_sections"], 12);

    // Check specific sections
    let sections = body["completion"]["sections"].as_array().unwrap();
    let identity = sections.iter().find(|s| s["name"] == "identity").unwrap();
    assert_eq!(identity["complete"], true);
    assert!(identity["missing_fields"].as_array().unwrap().is_empty());

    let grading = sections.iter().find(|s| s["name"] == "grading").unwrap();
    assert_eq!(grading["complete"], false);
    assert!(!grading["missing_fields"].as_array().unwrap().is_empty());
}
