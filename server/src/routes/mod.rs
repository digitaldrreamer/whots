pub mod auth;
pub mod friends;
pub mod games;
pub mod users;
pub mod ws;

use axum::{routing::{delete, get, post}, Router};
use crate::state::AppState;

pub fn all_routes() -> Router<AppState> {
    Router::new()
        .nest("/auth",    auth_routes())
        .nest("/users",   user_routes())
        .nest("/friends", friend_routes())
        .nest("/games",   game_routes())
        .nest("/ws",      ws_routes())
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/guest",    post(auth::guest))
        .route("/register", post(auth::register))
        .route("/login",    post(auth::login))
        .route("/refresh",  post(auth::refresh))
        .route("/logout",   delete(auth::logout))
}

fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/me",               get(users::me).put(users::update_me))
        .route("/search",           get(users::search))
        .route("/:username",        get(users::get_by_username))
        .route("/contacts/upload",  post(users::upload_contact_hashes))
        .route("/contacts/matches", get(users::contact_matches))
}

fn friend_routes() -> Router<AppState> {
    Router::new()
        .route("/",                             get(friends::list))
        .route("/requests",                     get(friends::incoming_requests))
        .route("/request/:username",            post(friends::send_request))
        .route("/request/:username/accept",     post(friends::accept_request))
        .route("/request/:username/decline",    post(friends::decline_request))
        .route("/:username",                    delete(friends::remove))
}

fn game_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(games::create))
}

fn ws_routes() -> Router<AppState> {
    Router::new()
        .route("/game/:game_id", get(ws::game_socket))
}
