use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedEvent {
    pub project_id: Uuid,
    pub session_id: Uuid,
    pub visitor_id: String,
    pub event_name: String,
    pub event_data: Option<serde_json::Value>,
    pub path: Option<String>,
    pub created_at: DateTime<Utc>,
}
