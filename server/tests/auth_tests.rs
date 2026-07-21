mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use common::{guest_token, make_app, register_user, req};

#[sqlx::test]
async fn health_returns_ok(pool: PgPool) {
    let app = make_app(pool);
    let (status, body) = req(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[sqlx::test]
async fn guest_creates_account(pool: PgPool) {
    let app = make_app(pool);
    let (status, body) = req(
        &app,
        "POST",
        "/api/auth/guest",
        Some(json!({ "username": "guest_test" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert_eq!(body["user"]["username"], "guest_test");
    assert_eq!(body["user"]["is_guest"], true);
}

/// A taken guest name must not block play — it auto-suffixes instead. Rejecting
/// the duplicate used to lock guests out entirely: they'd make a guest, log out,
/// and be unable to return, because the name was taken and guests can't log in.
#[sqlx::test]
async fn guest_duplicate_username_suffixes(pool: PgPool) {
    let app = make_app(pool);
    guest_token(&app, "dupuser").await;
    let (status, body) = req(
        &app,
        "POST",
        "/api/auth/guest",
        Some(json!({ "username": "dupuser" })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let username = body["user"]["username"].as_str().unwrap();
    assert_ne!(username, "dupuser", "second guest must get a distinct username");
    assert!(
        username.starts_with("dupuser-") && username["dupuser-".len()..].parse::<u32>().is_ok(),
        "expected a `dupuser-<number>` suffix, got {username:?}"
    );
    // The name they typed survives as the display name.
    assert_eq!(body["user"]["display_name"], "dupuser");
    assert_eq!(body["user"]["is_guest"], true);
}

#[sqlx::test]
async fn register_and_login(pool: PgPool) {
    let app = make_app(pool);
    let (access, _) = register_user(&app, "alice", "Password1!").await;
    assert!(!access.is_empty());

    let (status, body) = req(
        &app,
        "POST",
        "/api/auth/login",
        Some(json!({ "identifier": "alice", "password": "Password1!" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
}

#[sqlx::test]
async fn login_wrong_password(pool: PgPool) {
    let app = make_app(pool);
    register_user(&app, "bob", "Password1!").await;

    let (status, _) = req(
        &app,
        "POST",
        "/api/auth/login",
        Some(json!({ "identifier": "bob", "password": "Wrong!" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn refresh_token_rotates(pool: PgPool) {
    let app = make_app(pool);
    let (_, refresh) = register_user(&app, "carol", "Password1!").await;

    let (status, body) = req(
        &app,
        "POST",
        "/api/auth/refresh",
        Some(json!({ "refresh_token": refresh })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
    // New refresh token must differ from the old one
    assert_ne!(body["refresh_token"].as_str().unwrap(), refresh);
}

#[sqlx::test]
async fn logout_invalidates_refresh_token(pool: PgPool) {
    let app = make_app(pool);
    let (access, refresh) = register_user(&app, "dave", "Password1!").await;

    let (status, _) = req(
        &app,
        "DELETE",
        "/api/auth/logout",
        Some(json!({ "refresh_token": refresh })),
        Some(&access),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Second refresh with same token must fail
    let (status, _) = req(
        &app,
        "POST",
        "/api/auth/refresh",
        Some(json!({ "refresh_token": refresh })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn protected_route_without_token_is_unauthorized(pool: PgPool) {
    let app = make_app(pool);
    let (status, _) = req(&app, "GET", "/api/users/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
