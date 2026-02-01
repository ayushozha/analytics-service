use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Deserialize)]
pub struct CreateProject {
    pub name: String,
    pub domain: Option<String>,
    pub umami_website_id: Option<String>,
    pub umami_share_url: Option<String>,
    #[serde(default = "default_settings")]
    pub settings: serde_json::Value,
}

fn default_settings() -> serde_json::Value {
    serde_json::json!({ "retention_days": 365 })
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKeyRow {
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

#[derive(Debug, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_scopes() -> Vec<String> {
    vec!["ingest".to_string()]
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub key: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedKey {
    pub project_id: Uuid,
    pub scopes: Vec<String>,
}
