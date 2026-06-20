mod common;

use axum::http::StatusCode;
use sqlx::PgPool;

use common::{make_app, register_user, req};

#[sqlx::test]
async fn send_and_list_friend_request(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "alice", "Password1!").await;
    register_user(&app, "bob", "Password1!").await;

    let (status, _) = req(
        &app,
        "POST",
        "/api/friends/request/bob",
        None,
        Some(&token_a),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // alice's friend list is still empty (not yet accepted)
    let (status, body) = req(&app, "GET", "/api/friends", None, Some(&token_a)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn accept_friend_request(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "alice", "Password1!").await;
    let (token_b, _) = register_user(&app, "bob", "Password1!").await;

    req(&app, "POST", "/api/friends/request/bob", None, Some(&token_a)).await;

    let (status, _) = req(
        &app,
        "POST",
        "/api/friends/request/alice/accept",
        None,
        Some(&token_b),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = req(&app, "GET", "/api/friends", None, Some(&token_a)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["username"], "bob");
}

#[sqlx::test]
async fn decline_friend_request(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "alice", "Password1!").await;
    let (token_b, _) = register_user(&app, "bob", "Password1!").await;

    req(&app, "POST", "/api/friends/request/bob", None, Some(&token_a)).await;

    let (status, _) = req(
        &app,
        "POST",
        "/api/friends/request/alice/decline",
        None,
        Some(&token_b),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Neither side has friends
    let (_, body) = req(&app, "GET", "/api/friends", None, Some(&token_a)).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn remove_friend(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "alice", "Password1!").await;
    let (token_b, _) = register_user(&app, "bob", "Password1!").await;

    req(&app, "POST", "/api/friends/request/bob", None, Some(&token_a)).await;
    req(&app, "POST", "/api/friends/request/alice/accept", None, Some(&token_b)).await;

    let (status, _) = req(
        &app,
        "DELETE",
        "/api/friends/bob",
        None,
        Some(&token_a),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = req(&app, "GET", "/api/friends", None, Some(&token_a)).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn incoming_requests_visible_to_addressee(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "alice", "Password1!").await;
    let (token_b, _) = register_user(&app, "bob", "Password1!").await;

    req(&app, "POST", "/api/friends/request/bob", None, Some(&token_a)).await;

    let (status, body) = req(&app, "GET", "/api/friends/requests", None, Some(&token_b)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["username"], "alice");
}
