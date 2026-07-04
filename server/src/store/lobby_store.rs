use anyhow::Result;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::types::{Difficulty, GameMode};

const TTL_SECS: u64 = 1800; // 30-min safety-net TTL for a pre-game lobby

/// A human who has actually joined the lobby (host first). Only these become
/// human seats when the game starts — invited-but-absent friends never do, which
/// is what keeps the game driver from stalling on a seat nobody occupies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyMember {
    pub user_id: Uuid,
    pub username: String,
}

/// A pre-game room the host composes: joined humans + AI seats + invites.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lobby {
    pub id: Uuid,
    pub host_id: Uuid,
    pub mode: GameMode,
    pub members: Vec<LobbyMember>,
    pub ais: Vec<Difficulty>,
    pub invited: Vec<Uuid>,
}

fn key(id: Uuid) -> String {
    format!("lobby:{id}")
}

pub async fn save(conn: &mut redis::aio::MultiplexedConnection, lobby: &Lobby) -> Result<()> {
    let json = serde_json::to_string(lobby)?;
    conn.set_ex::<_, _, ()>(key(lobby.id), json, TTL_SECS).await?;
    Ok(())
}

pub async fn load(
    conn: &mut redis::aio::MultiplexedConnection,
    id: Uuid,
) -> Result<Option<Lobby>> {
    let raw: Option<String> = conn.get(key(id)).await?;
    match raw {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

pub async fn delete(conn: &mut redis::aio::MultiplexedConnection, id: Uuid) -> Result<()> {
    conn.del::<_, ()>(key(id)).await?;
    Ok(())
}
