pub mod auth;
pub mod config;
pub mod error;
pub mod game;
pub mod mail;
pub mod models;
pub mod routes;
pub mod state;
pub mod store;

use std::{sync::Arc, time::Duration};

use axum::{http::Method, routing::get, Router};
use dashmap::DashMap;
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowHeaders, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use config::Config;
use routes::ws::RoomRegistry;
use state::AppState;
use store::notification_store::NotifyTxMap;

/// Build the application router (extracted so integration tests can call it).
pub fn make_router(state: AppState, origin: axum::http::HeaderValue) -> Router {
    // `allow_credentials(true)` cannot be combined with a wildcard
    // `Access-Control-Allow-Headers: *` — tower-http panics on that pairing.
    // Mirror the request's requested headers instead, which is valid with
    // credentials and covers the Content-Type / Authorization the client sends.
    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    Router::new()
        .route("/health", get(routes::health::health))
        .nest("/api", routes::all_routes())
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "whots_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let port = config.port;
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

    let rooms: Arc<RoomRegistry> = Arc::new(DashMap::new());
    let notify_txs: Arc<NotifyTxMap> = Arc::new(DashMap::new());

    // WebAuthn (passkeys). RP id = the bare domain; origin = the full app URL.
    let rp_origin = url::Url::parse(&config.app_url)?;
    let rp_id = rp_origin
        .domain()
        .ok_or_else(|| anyhow::anyhow!("APP_URL has no domain for the passkey RP id"))?;
    let webauthn = Arc::new(
        webauthn_rs::WebauthnBuilder::new(rp_id, &rp_origin)?
            .rp_name("Whot!")
            .build()?,
    );
    tracing::info!(%rp_id, "WebAuthn ready");

    let state = AppState {
        db,
        config: Arc::new(config),
        redis,
        rooms: Arc::clone(&rooms),
        notify_txs: Arc::clone(&notify_txs),
        webauthn,
    };

    tokio::spawn(cleanup_task(
        state.db.clone(),
        state.redis.clone(),
        Arc::clone(&rooms),
    ));

    let app = make_router(state, origin);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on port {port}");

    // ConnectInfo must be populated so the rate limiter's PeerIpKeyExtractor can
    // read the client IP — without it every rate-limited route 500s.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn cleanup_task(db: sqlx::PgPool, redis_client: redis::Client, rooms: Arc<RoomRegistry>) {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;
        match abandoned_game_ids(&db).await {
            Ok(ids) if !ids.is_empty() => {
                tracing::info!("evicting {} abandoned games", ids.len());
                if let Ok(mut conn) = redis_client.get_multiplexed_tokio_connection().await {
                    for id in ids {
                        rooms.remove(&id);
                        let _ = store::game_store::delete(&mut conn, id).await;
                    }
                }
            }
            Ok(_) => {}
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
