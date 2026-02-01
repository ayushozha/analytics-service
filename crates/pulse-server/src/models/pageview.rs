use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedPageview {
    pub project_id: Uuid,
    pub session_id: Uuid,
    pub visitor_id: String,
    pub path: String,
    pub title: Option<String>,
    pub referrer: Option<String>,
    pub referrer_domain: Option<String>,
    pub query_params: Option<serde_json::Value>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}
