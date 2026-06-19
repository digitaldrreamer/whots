use std::{sync::Arc, time::Duration};

use axum::{routing::get, http::Method, Router};
use dashmap::DashMap;
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod auth;
mod config;
mod error;
mod game;
mod mail;
mod models;
mod routes;
mod state;
mod store;

use config::Config;
use routes::ws::RoomRegistry;
use state::AppState;
use store::notification_store::NotifyTxMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "whots_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let port   = config.port;
    let origin = config.frontend_url.parse::<axum::http::HeaderValue>()?;

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;
    tracing::info!("connected to PostgreSQL");

    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("migrations applied");

    let redis = redis::Client::open(config.redis_url.as_str())?;
    tracing::info!("Redis client ready");

    let rooms:      Arc<RoomRegistry> = Arc::new(DashMap::new());
    let notify_txs: Arc<NotifyTxMap>  = Arc::new(DashMap::new());

    let state = AppState {
        db,
        config: Arc::new(config),
        redis,
        rooms:      Arc::clone(&rooms),
        notify_txs: Arc::clone(&notify_txs),
    };

    tokio::spawn(cleanup_task(state.db.clone(), state.redis.clone(), Arc::clone(&rooms)));

    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any)
        .allow_credentials(true);

    let app = Router::new()
        .route("/health", get(routes::health::health))   // no auth, top-level
        .nest("/api", routes::all_routes())
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on port {port}");

    axum::serve(listener, app).await?;
    Ok(())
}

// ── Abandoned-game cleanup ─────────────────────────────────────────────────────

async fn cleanup_task(db: sqlx::PgPool, redis_client: redis::Client, rooms: Arc<RoomRegistry>) {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;
        match abandoned_game_ids(&db).await {
            Ok(ids) if !ids.is_empty() => {
                tracing::info!("evicting {} abandoned games", ids.len());
                if let Ok(mut conn) = redis_client.get_multiplexed_tokio_connection().await {
                    for id in ids {
                        // Remove from in-memory registry first — this drops RoomHandle,
                        // which drops move_tx, causing the driver task to exit cleanly.
                        rooms.remove(&id);
                        let _ = store::game_store::delete(&mut conn, id).await;
                    }
                }
            }
            Ok(_)  => {}
            Err(e) => tracing::warn!("cleanup task error: {e}"),
        }
    }
}

async fn abandoned_game_ids(db: &sqlx::PgPool) -> anyhow::Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        "UPDATE games SET status = 'abandoned'
         WHERE status = 'playing'
           AND last_activity_at < NOW() - INTERVAL '30 minutes'
         RETURNING id",
    )
    .fetch_all(db)
    .await?;
    Ok(ids)
}
