mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{make_app, register_user, req};

/// Fetch the UUID of the registered user by calling GET /api/users/me.
async fn my_id(app: &axum::Router, token: &str) -> Uuid {
    let (_, body) = req(app, "GET", "/api/users/me", None, Some(token)).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

#[sqlx::test]
async fn create_and_get_game(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "player1", "Password1!").await;
    let player_id = my_id(&app, &token).await;

    let (status, body) = req(
        &app,
        "POST",
        "/api/games",
        Some(json!({
            "mode": "stack",
            "seats": [
                { "kind": "human", "user_id": player_id, "name": "Player 1" },
                { "kind": "ai", "difficulty": "pikin", "name": "Easy Bot" }
            ]
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create game: {body}");
    let game_id = body["game_id"].as_str().unwrap();

    let (status, body) = req(
        &app,
        "GET",
        &format!("/api/games/{game_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "playing");
    assert_eq!(body["seats"].as_array().unwrap().len(), 2);
}

#[sqlx::test]
async fn create_game_requires_self_as_seat(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "alpha", "Password1!").await;
    let (_, _)       = register_user(&app, "beta",  "Password1!").await;
    let beta_id = {
        // get beta's id by searching
        let (_, body) = req(&app, "GET", "/api/users/beta", None, Some(&token_a)).await;
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    };

    let (status, _) = req(
        &app,
        "POST",
        "/api/games",
        Some(json!({
            "mode": "stack",
            "seats": [
                { "kind": "human", "user_id": beta_id, "name": "Beta" },
                { "kind": "ai", "difficulty": "pikin", "name": "Bot" }
            ]
        })),
        Some(&token_a),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn cancel_game(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "canceler", "Password1!").await;
    let uid = my_id(&app, &token).await;

    let (_, body) = req(
        &app,
        "POST",
        "/api/games",
        Some(json!({
            "mode": "no_stack",
            "seats": [
                { "kind": "human", "user_id": uid, "name": "Me" },
                { "kind": "ai", "difficulty": "chief", "name": "AI" }
            ]
        })),
        Some(&token),
    )
    .await;
    let game_id = body["game_id"].as_str().unwrap();

    let (status, _) = req(
        &app,
        "DELETE",
        &format!("/api/games/{game_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Game is now abandoned
    let (status, body) = req(
        &app,
        "GET",
        &format!("/api/games/{game_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "abandoned");
}

#[sqlx::test]
async fn cancel_game_non_participant_is_forbidden(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "owner", "Password1!").await;
    let (token_b, _) = register_user(&app, "intruder", "Password1!").await;
    let uid_a = my_id(&app, &token_a).await;

    let (_, body) = req(
        &app,
        "POST",
        "/api/games",
        Some(json!({
            "mode": "stack",
            "seats": [
                { "kind": "human", "user_id": uid_a, "name": "Owner" },
                { "kind": "ai", "difficulty": "pikin", "name": "Bot" }
            ]
        })),
        Some(&token_a),
    )
    .await;
    let game_id = body["game_id"].as_str().unwrap();

    let (status, _) = req(
        &app,
        "DELETE",
        &format!("/api/games/{game_id}"),
        None,
        Some(&token_b),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn get_game_not_found(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "someone", "Password1!").await;

    let random_id = Uuid::new_v4();
    let (status, _) = req(
        &app,
        "GET",
        &format!("/api/games/{random_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn get_game_non_participant_is_forbidden(pool: PgPool) {
    let app = make_app(pool);
    let (token_a, _) = register_user(&app, "host", "Password1!").await;
    let (token_b, _) = register_user(&app, "outsider", "Password1!").await;
    let uid_a = my_id(&app, &token_a).await;

    let (_, body) = req(
        &app,
        "POST",
        "/api/games",
        Some(json!({
            "mode": "stack",
            "seats": [
                { "kind": "human", "user_id": uid_a },
                { "kind": "ai", "difficulty": "pikin", "name": "Bot" }
            ]
        })),
        Some(&token_a),
    )
    .await;
    let game_id = body["game_id"].as_str().unwrap();

    // A user with no seat in the game cannot read its details.
    let (status, _) = req(
        &app,
        "GET",
        &format!("/api/games/{game_id}"),
        None,
        Some(&token_b),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn create_game_rejects_unknown_user_seat(pool: PgPool) {
    let app = make_app(pool);
    let (token, _) = register_user(&app, "creator2", "Password1!").await;
    let uid = my_id(&app, &token).await;

    // A second human seat referencing a user that does not exist is rejected.
    let (status, _) = req(
        &app,
        "POST",
        "/api/games",
        Some(json!({
            "mode": "stack",
            "seats": [
                { "kind": "human", "user_id": uid },
                { "kind": "human", "user_id": Uuid::new_v4() }
            ]
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
