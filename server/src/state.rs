use std::sync::Arc;
use dashmap::DashMap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, routes::ws::RoomHandle};

#[derive(Clone)]
pub struct AppState {
    pub db:     PgPool,
    pub config: Arc<Config>,
    pub redis:  redis::Client,
    pub rooms:  Arc<DashMap<Uuid, RoomHandle>>,
}
