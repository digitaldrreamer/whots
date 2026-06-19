use std::sync::Arc;

use axum::{http::Method, Router};
use dashmap::DashMap;
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod config;
mod error;
mod game;
mod models;
mod routes;
mod state;
mod store;

use config::Config;
use state::AppState;

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

    let state = AppState {
        db,
        config: Arc::new(config),
        redis,
        rooms: Arc::new(DashMap::new()),
    };

    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any)
        .allow_credentials(true);

    let app = Router::new()
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
