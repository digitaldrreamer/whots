mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use common::{make_app, register_user, req};

#[sqlx::test]
async fn me_returns_current_user(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "alice", "Password1!").await;

    let (status, body) = req(&app, "GET", "/api/users/me", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "alice");
}

#[sqlx::test]
async fn update_me_changes_display_name(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "bob", "Password1!").await;

    let (status, body) = req(
        &app,
        "PUT",
        "/api/users/me",
        Some(json!({ "display_name": "Bobby B" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["display_name"], "Bobby B");
}

#[sqlx::test]
async fn get_by_username(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "carol", "Password1!").await;

    let (status, body) = req(&app, "GET", "/api/users/carol", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "carol");
}

#[sqlx::test]
async fn get_by_username_not_found(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "dave", "Password1!").await;

    let (status, _) = req(&app, "GET", "/api/users/nobody", None, Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn search_users(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "searchme", "Password1!").await;

    let (status, body) = req(
        &app,
        "GET",
        "/api/users/search?q=search",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u["username"] == "searchme"));
}

#[sqlx::test]
async fn my_games_empty_initially(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "gamer", "Password1!").await;

    let (status, body) = req(&app, "GET", "/api/users/me/games", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}
