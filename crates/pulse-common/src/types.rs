use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CollectRequest {
    Pageview { payload: PageviewPayload },
    Event { payload: EventPayload },
    Identify { payload: IdentifyPayload },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectEnvelope {
    #[serde(flatten)]
    pub request: CollectRequest,
    pub visitor_id: String,
    #[serde(default)]
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageviewPayload {
    pub path: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub referrer: Option<String>,
    #[serde(default)]
    pub screen: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    pub name: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifyPayload {
    pub traits: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub domain: Option<String>,
    pub umami_website_id: Option<String>,
    pub umami_share_url: Option<String>,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub project_id: Uuid,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
