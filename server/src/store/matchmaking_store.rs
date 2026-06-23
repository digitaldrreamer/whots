use std::time::{SystemTime, UNIX_EPOCH};

use redis::AsyncCommands;
use uuid::Uuid;

use crate::game::types::GameMode;

const QUEUE_TTL_SECS: i64 = 300; // auto-expire stale entries after 5 min

fn queue_key(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Stack   => "matchmaking:stack",
        GameMode::NoStack => "matchmaking:no_stack",
    }
}

/// Add `user_id` to the queue for `mode`. Score = unix timestamp (FIFO order).
pub async fn join(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
    mode: GameMode,
) -> anyhow::Result<()> {
    let score = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs_f64();
    let key = queue_key(mode);
    let _: () = conn.zadd(key, user_id.to_string(), score).await?;
    let _: () = conn.expire(key, QUEUE_TTL_SECS).await?;
    Ok(())
}

/// Remove `user_id` from all mode queues.
pub async fn leave(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> anyhow::Result<()> {
    let s = user_id.to_string();
    let _: () = conn.zrem("matchmaking:stack", s.as_str()).await?;
    let _: () = conn.zrem("matchmaking:no_stack", s.as_str()).await?;
    Ok(())
}

/// Pop the oldest waiting user in `mode` that isn't `exclude`. Returns their id.
pub async fn pop_opponent(
    conn: &mut redis::aio::MultiplexedConnection,
    exclude: Uuid,
    mode: GameMode,
) -> anyhow::Result<Option<Uuid>> {
    let key = queue_key(mode);
    let members: Vec<String> = conn.zrange(key, 0isize, -1isize).await?;
    for member in members {
        if let Ok(id) = member.parse::<Uuid>() {
            if id != exclude {
                let _: () = conn.zrem(key, &member).await?;
                return Ok(Some(id));
            }
        }
    }
    Ok(None)
}

/// Return the mode the user is currently queued for, if any.
pub async fn queued_mode(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> anyhow::Result<Option<GameMode>> {
    let s = user_id.to_string();
    for (key, mode) in [
        ("matchmaking:stack",    GameMode::Stack),
        ("matchmaking:no_stack", GameMode::NoStack),
    ] {
        let score: Option<f64> = conn.zscore(key, &s).await?;
        if score.is_some() {
            return Ok(Some(mode));
        }
    }
    Ok(None)
}
