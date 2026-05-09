use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct Session {
    pub id: Uuid,
    pub project_id: Uuid,
    pub visitor_id: String,
    pub hostname: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub device: Option<String>,
    pub screen: Option<String>,
    pub language: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub first_at: DateTime<Utc>,
    pub last_at: DateTime<Utc>,
    pub is_bounce: bool,
    pub entry_page: Option<String>,
    pub exit_page: Option<String>,
    pub pageview_count: i32,
    pub event_count: i32,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCache {
    pub session_id: Uuid,
    pub pageview_count: i32,
    pub event_count: i32,
}
