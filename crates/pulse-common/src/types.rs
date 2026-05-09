use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CollectRequest {
    Pageview {
        payload: PageviewPayload,
    },
    Event {
        payload: EventPayload,
    },
    Identify {
        payload: IdentifyPayload,
    },
    #[serde(rename = "web_vital")]
    WebVital {
        payload: WebVitalPayload,
    },
    #[serde(rename = "scroll_depth")]
    ScrollDepth {
        payload: ScrollDepthPayload,
    },
    #[serde(rename = "search_query")]
    SearchQuery {
        payload: SearchQueryPayload,
    },
    Outlink {
        payload: OutlinkPayload,
    },
    #[serde(rename = "js_error")]
    JsError {
        payload: JsErrorPayload,
    },
    Log {
        payload: LogPayload,
    },
    #[serde(rename = "click_event")]
    ClickEvent {
        payload: ClickEventPayload,
    },
    #[serde(rename = "survey_response")]
    SurveyResponse {
        payload: SurveyResponsePayload,
    },
    #[serde(rename = "session_replay")]
    SessionReplay {
        payload: SessionReplayPayload,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectEnvelope {
    #[serde(flatten)]
    pub request: CollectRequest,
    pub visitor_id: String,
    #[serde(default)]
    pub timestamp: Option<i64>,
    #[serde(default)]
    pub consent_mode: Option<String>,
    #[serde(default)]
    pub consent_granted: Option<bool>,
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
    #[serde(default)]
    pub utm_source: Option<String>,
    #[serde(default)]
    pub utm_medium: Option<String>,
    #[serde(default)]
    pub utm_campaign: Option<String>,
    #[serde(default)]
    pub utm_content: Option<String>,
    #[serde(default)]
    pub utm_term: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    pub name: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub revenue_amount: Option<f64>,
    #[serde(default)]
    pub revenue_currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifyPayload {
    #[serde(default)]
    pub user_id: Option<String>,
    pub traits: serde_json::Value,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub account_name: Option<String>,
    #[serde(default)]
    pub account_traits: Option<serde_json::Value>,
    #[serde(default)]
    pub account_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebVitalPayload {
    pub name: String,
    pub value: f64,
    #[serde(default)]
    pub rating: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollDepthPayload {
    pub path: String,
    pub max_depth: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryPayload {
    pub query: String,
    #[serde(default)]
    pub results_count: Option<i32>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlinkPayload {
    pub url: String,
    pub link_type: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsErrorPayload {
    pub message: String,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub lineno: Option<i32>,
    #[serde(default)]
    pub colno: Option<i32>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPayload {
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickEventPayload {
    pub path: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub element_selector: Option<String>,
    #[serde(default)]
    pub viewport_width: Option<i32>,
    #[serde(default)]
    pub viewport_height: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyResponsePayload {
    pub survey_id: String,
    pub answers: serde_json::Value,
    #[serde(default)]
    pub completed: Option<bool>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReplayPayload {
    pub events: serde_json::Value,
    #[serde(default)]
    pub started_at: Option<i64>,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub entry_page: Option<String>,
    #[serde(default)]
    pub screen: Option<String>,
    #[serde(default)]
    pub is_complete: Option<bool>,
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
