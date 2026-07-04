pub mod auth;
pub mod friends;
pub mod games;
pub mod health;
pub mod invites;
pub mod matchmaking;
pub mod notifications;
pub mod passkey;
pub mod rooms;
pub mod users;
pub mod ws;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use crate::state::AppState;

pub fn all_routes() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth_routes())
        .nest("/users", user_routes())
        .nest("/friends", friend_routes())
        .nest("/games", game_routes())
        .nest("/matchmaking", matchmaking_routes())
        .nest("/notifications", notification_routes())
        .nest("/rooms", room_routes())
        .nest("/invites", invite_routes())
        .nest("/ws", ws_routes())
}

fn invite_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(invites::create))
        .route("/:token/redeem", post(invites::redeem))
}

fn auth_routes() -> Router<AppState> {
    // 5 requests per minute per IP — 1 per 12 s with a burst of 5.
    // NOTE: use `.period(12s)`, not `.per_second(12)`. The latter permits 12
    // requests every second (~720/min), 144× the intended rate.
    let conf = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(12))
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    Router::new()
        .route("/guest", post(auth::guest))
        .route("/register", post(auth::register))
        .route("/upgrade", post(auth::upgrade))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh))
        .route("/logout", delete(auth::logout))
        .route("/forgot-password", post(auth::forgot_password))
        .route("/reset-password", post(auth::reset_password))
        .route("/verify-email", post(auth::verify_email))
        .route("/resend-verification", post(auth::resend_verification))
        .route("/passkey/register/start", post(passkey::register_start))
        .route("/passkey/register/finish", post(passkey::register_finish))
        .route("/passkey/login/start", post(passkey::login_start))
        .route("/passkey/login/finish", post(passkey::login_finish))
        .layer(GovernorLayer { config: conf })
}

fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(users::me).put(users::update_me))
        .route("/me/games", get(users::my_games))
        // Username search removed — friend discovery is invite-link only.
        .route("/:username", get(users::get_by_username))
        .route("/contacts/upload", post(users::upload_contact_hashes))
        .route("/contacts/matches", get(users::contact_matches))
}

fn friend_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(friends::list))
        .route("/requests", get(friends::incoming_requests))
        .route("/request/:username", post(friends::send_request))
        .route("/request/:username/accept", post(friends::accept_request))
        .route("/request/:username/decline", post(friends::decline_request))
        .route("/:username", delete(friends::remove))
}

fn game_routes() -> Router<AppState> {
    // 10 creates/cancels per minute per IP — 1 per 6 s with burst of 3.
    // `.period(6s)` throttles to 1 request per 6 s; `.per_second(6)` would
    // instead allow 6 per second.
    let conf = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(6))
            .burst_size(3)
            .finish()
            .unwrap(),
    );
    Router::new()
        .route("/", post(games::create))
        .route("/:id", get(games::get_by_id).delete(games::cancel))
        .route("/:id/accept", post(games::accept))
        .route("/:id/decline", post(games::decline))
        .layer(GovernorLayer { config: conf })
}

fn room_routes() -> Router<AppState> {
    // Lobby actions are interactive (adding AIs, inviting friends, joining), so a
    // lenient limit: burst of 10, then ~1 every 2 s per IP.
    let conf = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(2))
            .burst_size(10)
            .finish()
            .unwrap(),
    );
    Router::new()
        .route("/", post(rooms::create))
        .route("/:id", get(rooms::get_room))
        .route("/:id/invite", post(rooms::invite))
        .route("/:id/join", post(rooms::join))
        .route("/:id/leave", post(rooms::leave))
        .route("/:id/ai", post(rooms::add_ai))
        .route("/:id/ai/:index", delete(rooms::remove_ai))
        .route("/:id/start", post(rooms::start))
        .layer(GovernorLayer { config: conf })
}

fn matchmaking_routes() -> Router<AppState> {
    Router::new()
        .route("/join", post(matchmaking::join))
        .route("/queue", delete(matchmaking::leave))
        .route("/status", get(matchmaking::status))
}

fn notification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(notifications::list).delete(notifications::mark_all_read),
        )
        .route("/count", get(notifications::unread_count))
        .route("/:id", patch(notifications::mark_one_read))
}

fn ws_routes() -> Router<AppState> {
    Router::new()
        .route("/game/:game_id", get(ws::game_socket))
        .route("/notify", get(notifications::notify_socket))
}
