//! Integration tests for the public site and the admin panel.
//!
//! The tests spin up the real application against a temporary SQLite database
//! so that routing, templates, sessions and CSRF are all exercised together.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::ServiceExt;

use personal_site::auth::session;
use personal_site::config::Config;
use personal_site::handlers;
use personal_site::mail::Mailer;
use personal_site::settings::{self, SettingsHandle};
use personal_site::state::AppState;

/// Build a fully wired application backed by an in-memory database.
async fn test_app() -> (axum::Router, AppState) {
    let config = Config::from_env().expect("config");
    let pool: SqlitePool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let settings = SettingsHandle::new(settings::load(&pool).await.expect("settings"));
    let state = AppState::new(
        pool.clone(),
        Arc::new(config),
        settings,
        Mailer::new(Default::default()),
    );

    (handlers::router(state.clone()), state)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .expect("response");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

async fn post_form(app: &axum::Router, uri: &str, fields: &[(&str, &str)]) -> (StatusCode, String) {
    let body = fields
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

fn urlencode(input: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

#[tokio::test]
async fn public_pages_render() {
    let (app, _state) = test_app().await;

    for path in [
        "/",
        "/about",
        "/projects",
        "/blog",
        "/experience",
        "/skills",
        "/services",
        "/testimonials",
        "/awards",
        "/speaking",
        "/publications",
        "/gallery",
        "/contact",
        "/resume",
        "/search",
    ] {
        let (status, body) = get(&app, path).await;
        assert_eq!(status, StatusCode::OK, "{path} returned {status}");
        assert!(
            body.contains("<!doctype html>"),
            "{path} is not an HTML page"
        );
    }
}

#[tokio::test]
async fn feeds_and_sitemaps_are_valid() {
    let (app, _state) = test_app().await;

    let (status, body) = get(&app, "/feed.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<rss"));
    assert!(body.contains("<channel>"));

    let (status, body) = get(&app, "/sitemap.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<urlset"));

    let (status, body) = get(&app, "/robots.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Sitemap:"));
}

#[tokio::test]
async fn unknown_paths_return_404() {
    let (app, _state) = test_app().await;
    let (status, _body) = get(&app, "/definitely-not-a-page").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_requires_authentication() {
    let (app, _state) = test_app().await;
    let (status, _body) = get(&app, "/admin").await;
    assert_eq!(status, StatusCode::SEE_OTHER, "guests should be redirected");
}

#[tokio::test]
async fn login_rejects_bad_credentials() {
    let (app, _state) = test_app().await;

    let (status, _body) = get(&app, "/admin/login").await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = post_form(
        &app,
        "/admin/login",
        &[("username", "admin"), ("password", "wrong-password")],
    )
    .await;

    assert_eq!(status, StatusCode::OK, "the login form re-renders");
    assert!(body.contains("do not match our records"));
}

#[tokio::test]
async fn sessions_are_created_and_cleared() {
    let (app, state) = test_app().await;

    // Seed a user with a known password.
    let hash = personal_site::auth::password::hash_password("correct horse battery").unwrap();
    sqlx::query(
        "INSERT INTO users (username, email, password_hash, role, display_name)
         VALUES ('tester', 'tester@example.com', ?, 'owner', 'Tester')",
    )
    .bind(&hash)
    .execute(&state.db)
    .await
    .unwrap();

    let (session_id, csrf, _expires) = session::create_session(
        &state.db,
        &state.config,
        1,
        Some("127.0.0.1"),
        Some("test-agent"),
        false,
    )
    .await
    .unwrap();

    assert!(!csrf.is_empty());

    let cookie = format!("{}={}", session::SESSION_COOKIE, session_id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "a valid session grants access"
    );

    session::destroy_session(&state.db, &session_id).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::SEE_OTHER,
        "a revoked session must not grant access"
    );
}

#[tokio::test]
async fn contact_form_validates_input() {
    let (app, _state) = test_app().await;

    let (status, body) = post_form(
        &app,
        "/contact",
        &[("name", "A"), ("email", "not-an-email"), ("message", "hi")],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("valid e-mail address"));
    assert!(body.contains("at least a sentence"));
}

#[tokio::test]
async fn admin_post_creation_requires_csrf() {
    let (app, state) = test_app().await;

    let hash = personal_site::auth::password::hash_password("correct horse battery").unwrap();
    sqlx::query(
        "INSERT INTO users (username, email, password_hash, role, display_name)
         VALUES ('tester', 'tester@example.com', ?, 'owner', 'Tester')",
    )
    .bind(&hash)
    .execute(&state.db)
    .await
    .unwrap();

    let (session_id, csrf, _expires) = session::create_session(
        &state.db,
        &state.config,
        1,
        Some("127.0.0.1"),
        Some("test-agent"),
        false,
    )
    .await
    .unwrap();

    let cookie = format!("{}={}", session::SESSION_COOKIE, session_id);

    // Without a CSRF token the request must be rejected.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/posts/new")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("title=No+CSRF&content=body"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // With the correct token the post is created.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/posts/new")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "title=Created+by+test&content=%23+Hello&status=published&csrf_token={}",
                    urlencode(&csrf)
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE title = 'Created by test'")
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}
