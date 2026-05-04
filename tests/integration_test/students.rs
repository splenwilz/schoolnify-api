use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use chrono::Datelike;
use http_body_util::BodyExt;
use schoolnify_api::state::AppState;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::MockServer;

use super::common::fixtures::*;
use super::common::jwt::*;
use super::common::state::*;

const TEST_BOUNDARY: &str = "----schoolnifyTestBoundary";

struct TestSchool {
    workos_id: String,
    org_id: Uuid,
    token: String,
}

async fn setup_school(state: &AppState, mock_server: &MockServer, role: &str) -> TestSchool {
    let email = unique_email();
    let workos_id = unique_workos_id();
    let workos_org_id = unique_workos_org_id();
    let slug = unique_slug("students");

    let (_user_id, org_id) = seed_user_with_org(
        &state.db_pool,
        &workos_id,
        &email,
        "Test Students School",
        &slug,
        &workos_org_id,
        role,
    )
    .await;

    seed_school_setup(
        &state.db_pool,
        org_id,
        json!({
            "identity": { "admission_number_prefix": "INF" },
            "grade_levels": {
                "grade_levels": ["Primary 1", "Primary 2", "JSS 1", "JSS 2"]
            }
        }),
    )
    .await;

    let token = sign_test_jwt(&workos_id, None, &mock_server.uri());

    TestSchool {
        workos_id,
        org_id,
        token,
    }
}

fn min_student(grade: &str) -> serde_json::Value {
    json!({
        "first_name": "Ada",
        "last_name": "Lovelace",
        "date_of_birth": "2017-12-10",
        "gender": "female",
        "grade_level": grade,
    })
}

async fn multipart_post(
    app: Router,
    uri: &str,
    parts: Vec<(&str, Option<&str>, Vec<u8>)>,
    token: &str,
) -> (StatusCode, serde_json::Value) {
    let mut body: Vec<u8> = Vec::new();
    for (name, filename, data) in parts {
        body.extend_from_slice(format!("--{TEST_BOUNDARY}\r\n").as_bytes());
        let disposition = match filename {
            Some(f) => format!(
                "Content-Disposition: form-data; name=\"{name}\"; filename=\"{f}\"\r\n\
                 Content-Type: text/csv\r\n\r\n"
            ),
            None => format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n"),
        };
        body.extend_from_slice(disposition.as_bytes());
        body.extend_from_slice(&data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{TEST_BOUNDARY}--\r\n").as_bytes());

    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={TEST_BOUNDARY}"),
        )
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes).to_string() }))
    };
    (status, json)
}

// ── Tests ───────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_create_student_succeeds_with_minimum_fields() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;
    let app = test_router(state.clone());

    let (status, body) = post_json_auth(
        app,
        "/api/v1/students",
        min_student("Primary 1"),
        &school.token,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    let admission = body["admission_number"].as_str().unwrap();
    let year = chrono::Utc::now().format("%Y").to_string();
    assert_eq!(admission, format!("INF/{year}/001"));
    assert_eq!(body["status"], "active");
    assert_eq!(body["fee_status"], "unknown");
    assert!(body["gpa"].is_null());
    assert!(body["attendance_rate"].is_null());
}

#[tokio::test]
#[serial]
async fn test_create_student_increments_admission_number() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;
    let year = chrono::Utc::now().format("%Y").to_string();

    let app1 = test_router(state.clone());
    let (s1, b1) = post_json_auth(app1, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["admission_number"], format!("INF/{year}/001"));

    let app2 = test_router(state.clone());
    let (s2, b2) = post_json_auth(app2, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(b2["admission_number"], format!("INF/{year}/002"));

    let app3 = test_router(state.clone());
    let (s3, b3) = post_json_auth(app3, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    assert_eq!(s3, StatusCode::CREATED);
    assert_eq!(b3["admission_number"], format!("INF/{year}/003"));
}

#[tokio::test]
#[serial]
async fn test_create_student_rejects_unknown_grade_level() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;
    let app = test_router(state.clone());

    let (status, body) = post_json_auth(
        app,
        "/api/v1/students",
        min_student("Senior High"),
        &school.token,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("Senior High"), "expected msg about grade_level, got {msg}");
}

#[tokio::test]
#[serial]
async fn test_admission_sequence_resets_on_new_year() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;
    let year = chrono::Utc::now().format("%Y").to_string();

    // Pre-seed school_configs as if last year's school had reached seq 50.
    // No students exist yet, so the dupe-admission_number check won't fire.
    sqlx::query(
        r#"
        INSERT INTO school_configs (org_id, admission_number_prefix, admission_number_seq_year, admission_number_next_seq)
        VALUES ($1, 'INF', $2, 51)
        ON CONFLICT (org_id) DO UPDATE
        SET admission_number_seq_year = EXCLUDED.admission_number_seq_year,
            admission_number_next_seq = EXCLUDED.admission_number_next_seq
        "#,
    )
    .bind(school.org_id)
    .bind((chrono::Utc::now().date_naive().year() - 1) as i16)
    .execute(&state.db_pool)
    .await
    .unwrap();

    let app = test_router(state.clone());
    let (status, body) = post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    // New year detected → seq resets to 001 (not 51).
    assert_eq!(body["admission_number"], format!("INF/{year}/001"));
}

#[tokio::test]
#[serial]
async fn test_list_students_filters_by_grade_and_default_status_active() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    // Seed three students in different grades.
    for grade in ["Primary 1", "Primary 1", "Primary 2"] {
        let app = test_router(state.clone());
        let _ = post_json_auth(app, "/api/v1/students", min_student(grade), &school.token).await;
    }

    let app = test_router(state.clone());
    let (status, body) =
        get_auth(app, "/api/v1/students?grade_level=Primary%201", &school.token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["pagination"]["total"], 2);

    // Whole-school summary ignores filters
    assert_eq!(body["summary"]["total_students"], 3);
    assert_eq!(body["summary"]["active"], 3);
    assert!(body["summary"]["average_gpa"].is_null());
}

#[tokio::test]
#[serial]
async fn test_get_student_returns_guardians_and_includes() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let payload = json!({
        "first_name": "Chidera",
        "last_name": "Okonkwo",
        "date_of_birth": "2017-03-15",
        "gender": "female",
        "grade_level": "Primary 1",
        "guardians": [
            {
                "first_name": "Emeka",
                "last_name": "Okonkwo",
                "phone": "+2348012345678",
                "is_primary": true
            },
            {
                "first_name": "Adaeze",
                "last_name": "Okonkwo",
                "phone": "+2348099999999"
            }
        ]
    });

    let app = test_router(state.clone());
    let (status, body) = post_json_auth(app, "/api/v1/students", payload, &school.token).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["guardians"].as_array().unwrap().len(), 2);

    let app = test_router(state.clone());
    let (s2, b2) = get_auth(
        app,
        &format!("/api/v1/students/{id}?include=recent_payments,recent_attendance"),
        &school.token,
    )
    .await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(b2["guardians"].as_array().unwrap().len(), 2);
    assert_eq!(b2["recent_payments"].as_array().unwrap().len(), 0);
    assert_eq!(b2["recent_attendance"].as_array().unwrap().len(), 0);
}

#[tokio::test]
#[serial]
async fn test_delete_student_soft_deletes_and_writes_history() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let app = test_router(state.clone());
    let (_, body) =
        post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    let id = body["id"].as_str().unwrap().to_string();

    let app = test_router(state.clone());
    let (status, _) = delete_auth(app, &format!("/api/v1/students/{id}"), &school.token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Default list excludes withdrawn
    let app = test_router(state.clone());
    let (_, list) = get_auth(app, "/api/v1/students", &school.token).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 0);

    // Explicit ?status=withdrawn includes them
    let app = test_router(state.clone());
    let (_, list) = get_auth(app, "/api/v1/students?status=withdrawn", &school.token).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1);

    // Status history row written
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM student_status_history WHERE student_id = $1",
    )
    .bind(Uuid::parse_str(&id).unwrap())
    .fetch_one(&state.db_pool)
    .await
    .unwrap();
    assert_eq!(count, 1);

    // withdrawn_at set
    let withdrawn_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT withdrawn_at FROM students WHERE id = $1")
            .bind(Uuid::parse_str(&id).unwrap())
            .fetch_one(&state.db_pool)
            .await
            .unwrap();
    assert!(withdrawn_at.is_some());
}

#[tokio::test]
#[serial]
async fn test_change_status_writes_history() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let app = test_router(state.clone());
    let (_, body) =
        post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    let id = body["id"].as_str().unwrap().to_string();

    let app = test_router(state.clone());
    let (status, body) = patch_json_auth(
        app,
        &format!("/api/v1/students/{id}/status"),
        json!({ "status": "transferred", "reason": "Moved abroad" }),
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["student"]["status"], "transferred");
    assert_eq!(body["status_change"]["from_status"], "active");
    assert_eq!(body["status_change"]["to_status"], "transferred");
    assert_eq!(body["status_change"]["reason"], "Moved abroad");
}

#[tokio::test]
#[serial]
async fn test_change_class_writes_history() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let app = test_router(state.clone());
    let (_, body) =
        post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    let id = body["id"].as_str().unwrap().to_string();

    let app = test_router(state.clone());
    let (status, body) = patch_json_auth(
        app,
        &format!("/api/v1/students/{id}/class"),
        json!({ "grade_level": "Primary 2", "section": "B" }),
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["grade_level"], "Primary 2");
    assert_eq!(body["section"], "B");

    let row: (String, Option<String>, String) = sqlx::query_as(
        "SELECT to_grade_level, to_section, change_kind FROM student_class_history WHERE student_id = $1",
    )
    .bind(Uuid::parse_str(&id).unwrap())
    .fetch_one(&state.db_pool)
    .await
    .unwrap();
    assert_eq!(row.0, "Primary 2");
    assert_eq!(row.1, Some("B".into()));
    assert_eq!(row.2, "manual");
}

#[tokio::test]
#[serial]
async fn test_promote_bulk_in_single_transaction() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    // Three students.
    let mut ids = vec![];
    for _ in 0..3 {
        let app = test_router(state.clone());
        let (_, body) = post_json_auth(
            app,
            "/api/v1/students",
            min_student("Primary 1"),
            &school.token,
        )
        .await;
        ids.push(body["id"].as_str().unwrap().to_string());
    }

    let app = test_router(state.clone());
    let (status, body) = post_json_auth(
        app,
        "/api/v1/students/promote",
        json!({
            "decisions": [
                { "student_id": ids[0], "action": "promote", "to_grade": "Primary 2", "to_section": "A" },
                { "student_id": ids[1], "action": "retain", "reason": "needs revision" },
                { "student_id": ids[2], "action": "graduate" }
            ]
        }),
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["promoted"], 1);
    assert_eq!(body["retained"], 1);
    assert_eq!(body["graduated"], 1);
    let batch_id = body["batch_id"].as_str().unwrap().to_string();

    // All three class_history rows share the batch_id.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM student_class_history WHERE promotion_batch_id = $1",
    )
    .bind(Uuid::parse_str(&batch_id).unwrap())
    .fetch_one(&state.db_pool)
    .await
    .unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
#[serial]
async fn test_promote_failure_rolls_back_all() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let app = test_router(state.clone());
    let (_, body) =
        post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    let real_id = body["id"].as_str().unwrap().to_string();
    let fake_id = Uuid::new_v4().to_string();

    let app = test_router(state.clone());
    let (status, _body) = post_json_auth(
        app,
        "/api/v1/students/promote",
        json!({
            "decisions": [
                { "student_id": real_id, "action": "promote", "to_grade": "Primary 2" },
                { "student_id": fake_id, "action": "promote", "to_grade": "Primary 2" }
            ]
        }),
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // No class_history rows were written — the tx rolled back.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM student_class_history WHERE student_id = $1",
    )
    .bind(Uuid::parse_str(&real_id).unwrap())
    .fetch_one(&state.db_pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
#[serial]
async fn test_bulk_import_with_skip_invalid_imports_valid_rows() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let csv = b"first_name,last_name,date_of_birth,gender,grade_level\n\
                Ada,Lovelace,2017-12-10,female,Primary 1\n\
                Bad,Row,not-a-date,female,Primary 1\n\
                Grace,Hopper,2018-09-09,female,Primary 2\n";
    let mapping = json!({
        "first_name": "first_name",
        "last_name": "last_name",
        "date_of_birth": "date_of_birth",
        "gender": "gender",
        "grade_level": "grade_level"
    })
    .to_string();

    let app = test_router(state.clone());
    let (status, body) = multipart_post(
        app,
        "/api/v1/students/bulk-import",
        vec![
            ("file", Some("students.csv"), csv.to_vec()),
            ("mapping", None, mapping.into_bytes()),
            ("skip_invalid", None, b"true".to_vec()),
        ],
        &school.token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["imported"], 2);
    assert!(!body["errors"].as_array().unwrap().is_empty());
    assert_eq!(body["imported_students"].as_array().unwrap().len(), 2);
}

#[tokio::test]
#[serial]
async fn test_bulk_import_without_skip_invalid_returns_422() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let csv = b"first_name,last_name,date_of_birth,gender,grade_level\n\
                Ada,Lovelace,2017-12-10,female,Primary 1\n\
                Bad,Row,not-a-date,female,Primary 1\n";
    let mapping = json!({
        "first_name": "first_name",
        "last_name": "last_name",
        "date_of_birth": "date_of_birth",
        "gender": "gender",
        "grade_level": "grade_level"
    })
    .to_string();

    let app = test_router(state.clone());
    let (status, body) = multipart_post(
        app,
        "/api/v1/students/bulk-import",
        vec![
            ("file", Some("students.csv"), csv.to_vec()),
            ("mapping", None, mapping.into_bytes()),
            ("skip_invalid", None, b"false".to_vec()),
        ],
        &school.token,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["imported"], 0);

    // No students were inserted.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM students WHERE org_id = $1")
        .bind(school.org_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
#[serial]
async fn test_export_returns_csv_with_correct_columns() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let app = test_router(state.clone());
    let _ = post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;

    let app = test_router(state.clone());
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/students/export")
        .header("authorization", format!("Bearer {}", school.token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let ct = response.headers().get("content-type").unwrap().to_str().unwrap().to_string();
    assert!(ct.starts_with("text/csv"), "got {ct}");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() >= 2, "expected at least header + one row, got: {text}");

    // Exact header, in column order. Catches accidental column reorder/rename.
    assert_eq!(
        lines[0],
        "Admission No,First Name,Last Name,Middle Name,Grade,Section,Gender,DOB,Status,Boarding,Fee Status,Guardian Name,Guardian Phone,Guardian Email"
    );

    // Exact data row for the seeded "Ada Lovelace" student. Most fields are
    // empty (no middle_name/section/boarding/guardians) so the row is largely
    // commas; this asserts that empty fields land in the right positions.
    let year = chrono::Utc::now().format("%Y").to_string();
    let expected_row = format!(
        "INF/{year}/001,Ada,Lovelace,,Primary 1,,female,2017-12-10,active,,unknown,,,"
    );
    assert!(
        lines.iter().any(|l| *l == expected_row),
        "expected row {expected_row:?} in CSV; got lines: {lines:?}"
    );
}

#[tokio::test]
#[serial]
async fn test_delete_is_idempotent() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let app = test_router(state.clone());
    let (_, body) =
        post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    let id = body["id"].as_str().unwrap().to_string();

    let app = test_router(state.clone());
    let (s1, _) = delete_auth(app, &format!("/api/v1/students/{id}"), &school.token).await;
    assert_eq!(s1, StatusCode::NO_CONTENT);

    // Second DELETE on already-withdrawn student → still 204 (idempotent).
    let app = test_router(state.clone());
    let (s2, _) = delete_auth(app, &format!("/api/v1/students/{id}"), &school.token).await;
    assert_eq!(s2, StatusCode::NO_CONTENT);

    // Only one history row was written (no duplicate audit on second DELETE).
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM student_status_history WHERE student_id = $1",
    )
    .bind(Uuid::parse_str(&id).unwrap())
    .fetch_one(&state.db_pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
#[serial]
async fn test_list_with_extreme_page_returns_empty_without_panic() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    // Seed one student so total > 0.
    let app = test_router(state.clone());
    let _ = post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;

    // Defensive: an out-of-range `page` must not overflow the offset calculation.
    let app = test_router(state.clone());
    let (status, body) = get_auth(
        app,
        &format!("/api/v1/students?page={}", i64::MAX),
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["pagination"]["page"], i64::MAX);
    // Whole-school summary still includes the seeded student.
    assert_eq!(body["summary"]["total_students"], 1);
}

#[tokio::test]
#[serial]
async fn test_non_admin_cannot_create_student() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    // Seed a non-admin user (role = "user") in an org with grade levels configured.
    let school = setup_school(&state, &mock_server, "user").await;
    let app = test_router(state.clone());

    let (status, body) = post_json_auth(
        app,
        "/api/v1/students",
        min_student("Primary 1"),
        &school.token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
}

#[tokio::test]
#[serial]
async fn test_non_admin_can_still_list_students() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "user").await;
    let app = test_router(state.clone());

    let (status, _) = get_auth(app, "/api/v1/students", &school.token).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn test_bulk_import_savepoints_isolate_duplicate_admission() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    // Pre-existing student with admission_number "INF/EXISTING/001".
    let app = test_router(state.clone());
    let (s, _) = post_json_auth(
        app,
        "/api/v1/students",
        json!({
            "first_name": "Pre",
            "last_name": "Existing",
            "date_of_birth": "2017-01-01",
            "gender": "male",
            "grade_level": "Primary 1",
            "admission_number": "INF/EXISTING/001"
        }),
        &school.token,
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    // CSV with three rows: one collides with the existing admission_number,
    // two are valid. With skip_invalid=true the two valid rows should still be
    // imported — the savepoint isolates the unique-violation per row.
    let csv = b"first_name,last_name,date_of_birth,gender,grade_level,admission_number\n\
                Ada,Lovelace,2017-12-10,female,Primary 1,INF/EXISTING/001\n\
                Grace,Hopper,2018-09-09,female,Primary 2,\n\
                Linus,Torvalds,2019-04-04,male,Primary 1,\n";
    let mapping = json!({
        "first_name": "first_name",
        "last_name": "last_name",
        "date_of_birth": "date_of_birth",
        "gender": "gender",
        "grade_level": "grade_level",
        "admission_number": "admission_number"
    })
    .to_string();

    let app = test_router(state.clone());
    let (status, body) = multipart_post(
        app,
        "/api/v1/students/bulk-import",
        vec![
            ("file", Some("students.csv"), csv.to_vec()),
            ("mapping", None, mapping.into_bytes()),
            ("skip_invalid", None, b"true".to_vec()),
        ],
        &school.token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["imported"], 2, "two valid rows imported despite the dup");
    assert!(!body["errors"].as_array().unwrap().is_empty());
    let err_msg = body["errors"][0]["message"].as_str().unwrap_or("");
    assert!(
        err_msg.contains("INF/EXISTING/001"),
        "expected duplicate-admission_number message, got {err_msg}"
    );

    // Total students in the school: 1 (pre-existing) + 2 (imported) = 3.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM students WHERE org_id = $1")
        .bind(school.org_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
#[serial]
async fn test_promote_rejects_duplicate_student_id() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let app = test_router(state.clone());
    let (_, body) =
        post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school.token).await;
    let id = body["id"].as_str().unwrap().to_string();

    // Two contradictory decisions for the same student.
    let app = test_router(state.clone());
    let (status, body) = post_json_auth(
        app,
        "/api/v1/students/promote",
        json!({
            "decisions": [
                { "student_id": id, "action": "promote", "to_grade": "Primary 2" },
                { "student_id": id, "action": "graduate" }
            ]
        }),
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    let msg = body["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("duplicate student_id"), "got: {msg}");
}

#[tokio::test]
#[serial]
async fn test_bulk_import_rejects_invalid_mapping_target() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    let csv = b"first_name,phonee\nAda,12345\n";
    // "phonee" is a typo of "phone" — must be rejected up front instead of
    // silently dropping the column.
    let mapping = json!({
        "first_name": "first_name",
        "phonee": "phonee"
    })
    .to_string();

    let app = test_router(state.clone());
    let (status, body) = multipart_post(
        app,
        "/api/v1/students/bulk-import",
        vec![
            ("file", Some("students.csv"), csv.to_vec()),
            ("mapping", None, mapping.into_bytes()),
            ("skip_invalid", None, b"true".to_vec()),
        ],
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}

#[tokio::test]
#[serial]
async fn test_bulk_import_rejects_duplicate_target() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    // Two distinct headers point at the same target — one would silently win
    // during row parsing, so the API must reject up front.
    let csv = b"First Name,Given Name\nAda,Lovelace\n";
    let mapping = json!({
        "First Name": "first_name",
        "Given Name": "first_name"
    })
    .to_string();

    let app = test_router(state.clone());
    let (status, body) = multipart_post(
        app,
        "/api/v1/students/bulk-import",
        vec![
            ("file", Some("students.csv"), csv.to_vec()),
            ("mapping", None, mapping.into_bytes()),
            ("skip_invalid", None, b"true".to_vec()),
        ],
        &school.token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    let msg = body["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("Duplicate mapping target"), "got: {msg}");
    // Both source headers should be named so the user can fix the CSV.
    assert!(msg.contains("First Name") && msg.contains("Given Name"), "got: {msg}");
}

#[tokio::test]
#[serial]
async fn test_export_sanitizes_csv_formula_injection() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school = setup_school(&state, &mock_server, "admin").await;

    // Seed a student whose first name starts with "=" — would be interpreted
    // as a formula by Excel/Sheets without sanitization.
    let app = test_router(state.clone());
    let (s, _) = post_json_auth(
        app,
        "/api/v1/students",
        json!({
            "first_name": "=cmd|'/c calc'!A1",
            "last_name": "Evil",
            "date_of_birth": "2017-12-10",
            "gender": "female",
            "grade_level": "Primary 1"
        }),
        &school.token,
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    let app = test_router(state.clone());
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/students/export")
        .header("authorization", format!("Bearer {}", school.token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();

    // The injection-prone first_name must be prefixed with `'` so spreadsheet
    // apps treat it as a string literal instead of a formula. The cell ends up
    // as `'=cmd|...`, which appears after the comma boundary in the CSV.
    assert!(
        text.contains(",'=cmd|"),
        "expected sanitized cell starting with `'=cmd|`; got:\n{text}"
    );
    // The raw `,=cmd...` (no leading single quote) must not appear, otherwise
    // the export would still execute as a formula.
    assert!(
        !text.contains(",=cmd|"),
        "raw '=cmd|' leaked into export: {text}"
    );
}

#[tokio::test]
#[serial]
async fn test_cross_tenant_access_returns_404() {
    let mock_server = MockServer::start().await;
    mount_jwks_endpoint(&mock_server).await;
    let state = test_app_state(&mock_server).await;
    let school_a = setup_school(&state, &mock_server, "admin").await;
    let school_b = setup_school(&state, &mock_server, "admin").await;

    // School A creates a student.
    let app = test_router(state.clone());
    let (_, body) =
        post_json_auth(app, "/api/v1/students", min_student("Primary 1"), &school_a.token).await;
    let id = body["id"].as_str().unwrap().to_string();

    // School B tries to read it.
    let app = test_router(state.clone());
    let (status, _) = get_auth(app, &format!("/api/v1/students/{id}"), &school_b.token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Suppress unused-warning on workos_id field.
    let _ = (&school_a.workos_id, &school_b.workos_id);
}
