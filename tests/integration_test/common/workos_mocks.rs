use wiremock::matchers::{body_string_contains, method, path, path_regex};
use wiremock::{Mock, ResponseTemplate};

/// Mock: POST /user_management/users → 201 (user created)
pub fn mock_create_user_success(email: &str, workos_user_id: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/users"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": workos_user_id,
            "email": email,
            "first_name": "Test",
            "last_name": "User",
            "email_verified": false,
            "profile_picture_url": null,
            "metadata": {}
        })))
}

/// Mock: POST /user_management/users → 409 (email exists)
pub fn mock_create_user_conflict() -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/users"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "code": "user_creation_error",
            "message": "An account with this email already exists."
        })))
}

/// Mock: POST /user_management/authenticate (password grant) → 200 success
pub fn mock_authenticate_password_success(
    workos_user_id: &str,
    email: &str,
    access_token: &str,
    refresh_token: &str,
) -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("\"grant_type\":\"password\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {
                "id": workos_user_id,
                "email": email,
                "first_name": "Test",
                "last_name": "User",
                "email_verified": true,
                "profile_picture_url": null,
                "metadata": {}
            },
            "access_token": access_token,
            "refresh_token": refresh_token
        })))
}

/// Mock: POST /user_management/authenticate (password grant) → 403 email verification required
pub fn mock_authenticate_password_needs_verification(pending_token: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("\"grant_type\":\"password\""))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "code": "email_verification_required",
            "pending_authentication_token": pending_token,
            "email_verification_id": "ev_test_123"
        })))
}

/// Mock: POST /user_management/authenticate (password grant) → 400 invalid credentials
pub fn mock_authenticate_password_invalid() -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("\"grant_type\":\"password\""))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "code": "invalid_credentials",
            "message": "Invalid email or password"
        })))
}

/// Mock: POST /user_management/authenticate (email verification grant) → 200
pub fn mock_authenticate_email_verification_success(
    workos_user_id: &str,
    email: &str,
    access_token: &str,
    refresh_token: &str,
) -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("email-verification:code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {
                "id": workos_user_id,
                "email": email,
                "first_name": "Test",
                "last_name": "User",
                "email_verified": true,
                "profile_picture_url": null,
                "metadata": {}
            },
            "access_token": access_token,
            "refresh_token": refresh_token
        })))
}

/// Mock: POST /user_management/authenticate (email verification grant) → 400
pub fn mock_authenticate_email_verification_failure() -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("email-verification:code"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "code": "invalid_code",
            "message": "The verification code is invalid or expired."
        })))
}

/// Mock: POST /user_management/authenticate (refresh_token grant) → 200
pub fn mock_refresh_token_success(
    workos_user_id: &str,
    email: &str,
    new_access_token: &str,
    new_refresh_token: &str,
) -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("\"grant_type\":\"refresh_token\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {
                "id": workos_user_id,
                "email": email,
                "first_name": "Test",
                "last_name": "User",
                "email_verified": true,
                "profile_picture_url": null,
                "metadata": {}
            },
            "access_token": new_access_token,
            "refresh_token": new_refresh_token
        })))
}

/// Mock: POST /user_management/authenticate (authorization_code grant) → 200
pub fn mock_authenticate_code_success(
    workos_user_id: &str,
    email: &str,
    access_token: &str,
    refresh_token: &str,
) -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("\"grant_type\":\"authorization_code\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {
                "id": workos_user_id,
                "email": email,
                "first_name": "Test",
                "last_name": "User",
                "email_verified": true,
                "profile_picture_url": null,
                "metadata": {}
            },
            "access_token": access_token,
            "refresh_token": refresh_token
        })))
}

/// Mock: POST /organizations → 201
pub fn mock_create_org_success(org_name: &str, workos_org_id: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path("/organizations"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": workos_org_id,
            "name": org_name,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })))
}

/// Mock: POST /user_management/organization_memberships → 201
pub fn mock_create_membership_success() -> Mock {
    Mock::given(method("POST"))
        .and(path("/user_management/organization_memberships"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "om_test_123",
            "user_id": "user_test",
            "organization_id": "org_test",
            "role": { "slug": "admin" },
            "status": "active"
        })))
}

/// Mock: DELETE /user_management/users/{id} → 200
pub fn mock_delete_user_success(workos_user_id: &str) -> Mock {
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/user_management/users/{workos_user_id}"
        )))
        .respond_with(ResponseTemplate::new(200))
}

/// Mock: DELETE /organizations/{id} → 200
pub fn mock_delete_org_success(workos_org_id: &str) -> Mock {
    Mock::given(method("DELETE"))
        .and(path(format!("/organizations/{workos_org_id}")))
        .respond_with(ResponseTemplate::new(200))
}

/// Mock: POST /user_management/users/{id}/email_verification/send → 200
pub fn mock_send_verification_email_success() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/user_management/users/.+/email_verification/send$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user_test"
        })))
}

/// Mock: POST /user_management/authenticate (refresh with org) → 200
pub fn mock_refresh_with_org_success(
    workos_user_id: &str,
    email: &str,
    new_access_token: &str,
    new_refresh_token: &str,
) -> Mock {
    // This matches the refresh_token grant that also includes organization_id
    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("\"grant_type\":\"refresh_token\""))
        .and(body_string_contains("\"organization_id\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {
                "id": workos_user_id,
                "email": email,
                "first_name": "Test",
                "last_name": "User",
                "email_verified": true,
                "profile_picture_url": null,
                "metadata": {}
            },
            "access_token": new_access_token,
            "refresh_token": new_refresh_token
        })))
}
