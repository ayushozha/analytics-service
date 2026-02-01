use std::sync::Arc;

use maxminddb::Reader;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::config::Config;
use crate::services::umami_client::UmamiClient;

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub geoip: Option<Reader<Vec<u8>>>,
    pub umami: Option<UmamiClient>,
}

impl AppState {
    pub fn redis_key(&self, suffix: &str) -> String {
        format!("{}{}", self.config.redis_key_prefix, suffix)
    }
}
