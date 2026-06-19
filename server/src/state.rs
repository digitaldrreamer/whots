use std::sync::Arc;

use dashmap::DashMap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::Config,
    routes::ws::RoomHandle,
    store::notification_store::NotifyTxMap,
};

#[derive(Clone)]
pub struct AppState {
    pub db:          PgPool,
    pub config:      Arc<Config>,
    pub redis:       redis::Client,
    pub rooms:       Arc<DashMap<Uuid, RoomHandle>>,
    pub notify_txs:  Arc<NotifyTxMap>,
}
