use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedWebVital {
    pub project_id: Uuid,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub path: Option<String>,
    pub metric_name: String,
    pub metric_value: f64,
    pub rating: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedScrollDepth {
    pub project_id: Uuid,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub path: String,
    pub max_depth: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedSearchQuery {
    pub project_id: Uuid,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub query: String,
    pub results_count: Option<i32>,
    pub path: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedOutlink {
    pub project_id: Uuid,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub url: String,
    pub link_type: String,
    pub path: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedJsError {
    pub project_id: Uuid,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub message: String,
    pub stack: Option<String>,
    pub filename: Option<String>,
    pub lineno: Option<i32>,
    pub colno: Option<i32>,
    pub path: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedClickEvent {
    pub project_id: Uuid,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub path: String,
    pub x: f64,
    pub y: f64,
    pub element_selector: Option<String>,
    pub viewport_width: Option<i32>,
    pub viewport_height: Option<i32>,
    pub created_at: DateTime<Utc>,
}
