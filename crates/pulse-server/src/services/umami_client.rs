use std::sync::Arc;

use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};

use crate::state::AppState;

const TOKEN_CACHE_TTL: u64 = 3300; // 55 minutes (tokens last 60min)

#[derive(Clone)]
pub struct UmamiClient {
    http: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiStats {
    pub pageviews: UmamiMetric,
    pub visitors: UmamiMetric,
    pub visits: UmamiMetric,
    pub bounces: UmamiMetric,
    pub totaltime: UmamiMetric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiMetric {
    pub value: i64,
    pub change: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiPageview {
    pub x: String, // path
    pub y: i64,    // count
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiReferrer {
    pub x: String, // referrer domain
    pub y: i64,    // count
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiEvent {
    pub x: String, // event name
    pub y: i64,    // count
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiBrowser {
    pub x: String,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiOS {
    pub x: String,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiCountry {
    pub x: String, // country code
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiTimeseriesPoint {
    pub x: String, // date string
    pub y: i64,    // count
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmamiActiveVisitors {
    pub x: i64, // count
}

impl UmamiClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    async fn get_token(&self, state: &Arc<AppState>) -> Result<String, UmamiError> {
        let cache_key = state.redis_key("umami:token");
        let mut redis = state.redis.clone();

        // Check cache
        let cached: Option<String> = redis.get(&cache_key).await.unwrap_or(None);
        if let Some(token) = cached {
            return Ok(token);
        }

        // Authenticate
        debug!("Authenticating with Umami at {}", self.base_url);
        let resp = self
            .http
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&serde_json::json!({
                "username": self.username,
                "password": self.password,
            }))
            .send()
            .await
            .map_err(|e| UmamiError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(UmamiError::Auth(format!("{status}: {body}")));
        }

        let auth: AuthResponse = resp
            .json()
            .await
            .map_err(|e| UmamiError::Parse(e.to_string()))?;

        // Cache token
        let _: () = redis
            .set_ex(&cache_key, &auth.token, TOKEN_CACHE_TTL)
            .await
            .unwrap_or(());

        Ok(auth.token)
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        state: &Arc<AppState>,
        path: &str,
    ) -> Result<T, UmamiError> {
        let token = self.get_token(state).await?;

        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| UmamiError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(UmamiError::Api(format!("{status}: {body}")));
        }

        resp.json()
            .await
            .map_err(|e| UmamiError::Parse(e.to_string()))
    }

    pub async fn get_stats(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
        start_at: i64,
        end_at: i64,
    ) -> Result<UmamiStats, UmamiError> {
        self.get(
            state,
            &format!(
                "/api/websites/{website_id}/stats?startAt={start_at}&endAt={end_at}"
            ),
        )
        .await
    }

    pub async fn get_pageviews(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
        start_at: i64,
        end_at: i64,
        limit: i64,
    ) -> Result<Vec<UmamiPageview>, UmamiError> {
        self.get(
            state,
            &format!(
                "/api/websites/{website_id}/metrics?startAt={start_at}&endAt={end_at}&type=url&limit={limit}"
            ),
        )
        .await
    }

    pub async fn get_referrers(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
        start_at: i64,
        end_at: i64,
        limit: i64,
    ) -> Result<Vec<UmamiReferrer>, UmamiError> {
        self.get(
            state,
            &format!(
                "/api/websites/{website_id}/metrics?startAt={start_at}&endAt={end_at}&type=referrer&limit={limit}"
            ),
        )
        .await
    }

    pub async fn get_browsers(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
        start_at: i64,
        end_at: i64,
        limit: i64,
    ) -> Result<Vec<UmamiBrowser>, UmamiError> {
        self.get(
            state,
            &format!(
                "/api/websites/{website_id}/metrics?startAt={start_at}&endAt={end_at}&type=browser&limit={limit}"
            ),
        )
        .await
    }

    pub async fn get_os(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
        start_at: i64,
        end_at: i64,
        limit: i64,
    ) -> Result<Vec<UmamiOS>, UmamiError> {
        self.get(
            state,
            &format!(
                "/api/websites/{website_id}/metrics?startAt={start_at}&endAt={end_at}&type=os&limit={limit}"
            ),
        )
        .await
    }

    pub async fn get_countries(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
        start_at: i64,
        end_at: i64,
        limit: i64,
    ) -> Result<Vec<UmamiCountry>, UmamiError> {
        self.get(
            state,
            &format!(
                "/api/websites/{website_id}/metrics?startAt={start_at}&endAt={end_at}&type=country&limit={limit}"
            ),
        )
        .await
    }

    pub async fn get_events(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
        start_at: i64,
        end_at: i64,
        limit: i64,
    ) -> Result<Vec<UmamiEvent>, UmamiError> {
        self.get(
            state,
            &format!(
                "/api/websites/{website_id}/metrics?startAt={start_at}&endAt={end_at}&type=event&limit={limit}"
            ),
        )
        .await
    }

    pub async fn get_active_visitors(
        &self,
        state: &Arc<AppState>,
        website_id: &str,
    ) -> Result<UmamiActiveVisitors, UmamiError> {
        self.get(
            state,
            &format!("/api/websites/{website_id}/active"),
        )
        .await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UmamiError {
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("Request failed: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
}
