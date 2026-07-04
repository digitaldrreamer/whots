use anyhow::Result;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::game::types::GameState;

const TTL_SECS: u64 = 7200; // 2-hour safety-net TTL

fn key(game_id: Uuid) -> String {
    format!("game:{game_id}")
}

pub async fn save(conn: &mut redis::aio::MultiplexedConnection, state: &GameState) -> Result<()> {
    let json = serde_json::to_string(state)?;
    conn.set_ex::<_, _, ()>(key(state.id), json, TTL_SECS)
        .await?;
    Ok(())
}

pub async fn load(
    conn: &mut redis::aio::MultiplexedConnection,
    game_id: Uuid,
) -> Result<Option<GameState>> {
    let raw: Option<String> = conn.get(key(game_id)).await?;
    match raw {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

pub async fn delete(conn: &mut redis::aio::MultiplexedConnection, game_id: Uuid) -> Result<()> {
    conn.del::<_, ()>(key(game_id)).await?;
    Ok(())
}
