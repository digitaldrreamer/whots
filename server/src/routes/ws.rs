use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{auth::AuthUser, error::AppError, state::AppState};

/// WebSocket upgrade for a game room.
/// All game events, chat, and WebRTC signalling go through this socket.
pub async fn game_socket(
    ws: WebSocketUpgrade,
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(game_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, claims, state, game_id)))
}

async fn handle_socket(
    mut socket: WebSocket,
    claims: crate::auth::Claims,
    _state: AppState,
    game_id: uuid::Uuid,
) {
    tracing::info!(user = %claims.sub, game = %game_id, "WebSocket connected");

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    handle_event(&mut socket, event, &claims, game_id).await;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    tracing::info!(user = %claims.sub, game = %game_id, "WebSocket disconnected");
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientEvent {
    // Game actions
    PlayCard  { card: serde_json::Value },
    Draw,
    // WebRTC signalling (friends-only gate enforced before reaching here)
    RtcOffer  { to: uuid::Uuid, sdp: String },
    RtcAnswer { to: uuid::Uuid, sdp: String },
    RtcIce    { to: uuid::Uuid, candidate: String },
    // Chat
    ChatMessage { text: String },
    // Feature toggles
    SetVideo { enabled: bool },
    SetAudio { enabled: bool },
    SetChat  { enabled: bool },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerEvent<'a> {
    GameState { state: &'a serde_json::Value },
    RtcSignal { from: uuid::Uuid, kind: &'a str, payload: &'a str },
    ChatMessage { from: uuid::Uuid, text: &'a str },
    Error { message: &'a str },
}

async fn handle_event(
    socket: &mut WebSocket,
    event: ClientEvent,
    claims: &crate::auth::Claims,
    _game_id: uuid::Uuid,
) {
    // Placeholder — full game loop and signalling relay implemented in Phase 4
    match event {
        ClientEvent::RtcOffer { to, sdp } => {
            tracing::debug!(from = %claims.sub, to = %to, "RTC offer");
            // TODO: verify mutual friendship, relay to `to`
        }
        ClientEvent::RtcAnswer { to, sdp } => {
            tracing::debug!(from = %claims.sub, to = %to, "RTC answer");
            // TODO: relay to `to`
        }
        ClientEvent::RtcIce { to, candidate } => {
            // TODO: relay to `to`
        }
        ClientEvent::ChatMessage { text } => {
            // TODO: broadcast to room, enforce chat-enabled flag
        }
        ClientEvent::PlayCard { card } => {
            // TODO: validate move, update game state, broadcast
        }
        ClientEvent::Draw => {
            // TODO: draw card, update state, broadcast
        }
        ClientEvent::SetVideo { enabled } => {
            // Client-side toggle; server notes preference
        }
        ClientEvent::SetAudio { enabled } => {
            // Client-side toggle; server notes preference
        }
        ClientEvent::SetChat { enabled } => {
            // TODO: store preference, stop routing chat to this user when false
        }
    }
}
