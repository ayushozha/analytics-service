use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct DsarDeleteResult {
    pub visitor_id: String,
    pub deleted: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PrivacySettings {
    pub project_id: Uuid,
    pub anonymize_ip: bool,
    pub respect_dnt: bool,
    pub bot_filtering: bool,
    pub consent_required: bool,
    pub allowed_consent_modes: Vec<String>,
    pub blocked_user_agents: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestPrivacyDecision {
    pub accepted: bool,
    pub reason: Option<String>,
}

const PRIVACY_COLUMNS: &str = "project_id, anonymize_ip, respect_dnt, bot_filtering, \
    consent_required, allowed_consent_modes, blocked_user_agents, created_at, updated_at";

pub async fn get_privacy_settings(db: &PgPool, project_id: Uuid) -> AppResult<PrivacySettings> {
    let settings = sqlx::query_as(&format!(
        "SELECT {PRIVACY_COLUMNS} FROM privacy_settings WHERE project_id = $1"
    ))
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    Ok(settings.unwrap_or_else(|| default_privacy_settings(project_id)))
}

pub async fn upsert_privacy_settings(
    db: &PgPool,
    project_id: Uuid,
    anonymize_ip: bool,
    respect_dnt: bool,
    bot_filtering: bool,
    consent_required: bool,
    allowed_consent_modes: &[String],
    blocked_user_agents: &[String],
) -> AppResult<PrivacySettings> {
    validate_consent_modes(allowed_consent_modes)?;

    let settings = sqlx::query_as(&format!(
        "INSERT INTO privacy_settings \
         (project_id, anonymize_ip, respect_dnt, bot_filtering, consent_required, allowed_consent_modes, blocked_user_agents) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (project_id) DO UPDATE SET \
           anonymize_ip = EXCLUDED.anonymize_ip, \
           respect_dnt = EXCLUDED.respect_dnt, \
           bot_filtering = EXCLUDED.bot_filtering, \
           consent_required = EXCLUDED.consent_required, \
           allowed_consent_modes = EXCLUDED.allowed_consent_modes, \
           blocked_user_agents = EXCLUDED.blocked_user_agents, \
           updated_at = NOW() \
         RETURNING {PRIVACY_COLUMNS}"
    ))
    .bind(project_id)
    .bind(anonymize_ip)
    .bind(respect_dnt)
    .bind(bot_filtering)
    .bind(consent_required)
    .bind(allowed_consent_modes)
    .bind(blocked_user_agents)
    .fetch_one(db)
    .await?;

    Ok(settings)
}

pub fn ingest_privacy_decision(
    settings: &PrivacySettings,
    user_agent: &str,
    dnt_enabled: bool,
    consent_mode: Option<&str>,
    consent_granted: Option<bool>,
) -> IngestPrivacyDecision {
    if settings.respect_dnt && dnt_enabled {
        return skip("dnt");
    }

    if settings.bot_filtering && is_bot_user_agent(user_agent, &settings.blocked_user_agents) {
        return skip("bot");
    }

    if consent_granted == Some(false) {
        return skip("consent_denied");
    }

    if settings.consent_required {
        if consent_granted != Some(true) {
            return skip("consent_required");
        }
        let Some(mode) = consent_mode else {
            return skip("consent_mode_required");
        };
        if !settings.allowed_consent_modes.is_empty()
            && !settings
                .allowed_consent_modes
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(mode))
        {
            return skip("consent_mode_not_allowed");
        }
    }

    IngestPrivacyDecision {
        accepted: true,
        reason: None,
    }
}

pub fn anonymize_ip(ip: &str) -> String {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(addr)) => {
            let octets = addr.octets();
            Ipv4Addr::new(octets[0], octets[1], octets[2], 0).to_string()
        }
        Ok(IpAddr::V6(addr)) => {
            let segments = addr.segments();
            Ipv6Addr::new(segments[0], segments[1], segments[2], 0, 0, 0, 0, 0).to_string()
        }
        Err(_) => ip.to_string(),
    }
}

pub fn is_bot_user_agent(user_agent: &str, custom_patterns: &[String]) -> bool {
    let ua = user_agent.to_ascii_lowercase();
    if ua.trim().is_empty() {
        return false;
    }

    let default_patterns = [
        "bot",
        "crawler",
        "spider",
        "slurp",
        "bingpreview",
        "headlesschrome",
        "phantomjs",
        "pingdom",
        "uptimerobot",
        "datadog",
        "newrelic",
        "statuscake",
        "monitoring",
        "facebookexternalhit",
        "discordbot",
        "slackbot",
        "linkedinbot",
        "twitterbot",
    ];

    default_patterns.iter().any(|pattern| ua.contains(pattern))
        || custom_patterns
            .iter()
            .map(|pattern| pattern.to_ascii_lowercase())
            .any(|pattern| !pattern.trim().is_empty() && ua.contains(pattern.trim()))
}

pub fn strip_geo_precision(geo: &mut crate::services::geo::GeoResult) {
    geo.region = None;
    geo.city = None;
}

pub async fn export_visitor_data(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
) -> AppResult<serde_json::Value> {
    let user_profile = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM user_profiles WHERE project_id = $1 AND visitor_id = $2) t",
        project_id,
        visitor_id,
    )
    .await?;
    let user_aliases = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM user_aliases WHERE project_id = $1 AND visitor_id = $2) t",
        project_id,
        visitor_id,
    )
    .await?;
    let sessions = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM sessions WHERE project_id = $1 AND visitor_id = $2 ORDER BY first_at DESC) t",
        project_id,
        visitor_id,
    )
    .await?;
    let pageviews = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM pageviews WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t",
        project_id,
        visitor_id,
    )
    .await?;
    let events = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM events WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t",
        project_id,
        visitor_id,
    )
    .await?;
    let goal_conversions = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM goal_conversions WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t",
        project_id,
        visitor_id,
    )
    .await?;
    let survey_responses = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM survey_responses WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t",
        project_id,
        visitor_id,
    )
    .await?;
    let experiment_assignments = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM experiment_assignments WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t",
        project_id,
        visitor_id,
    )
    .await?;
    let session_recordings = fetch_json_array(
        db,
        "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM \
         (SELECT * FROM session_recordings WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t",
        project_id,
        visitor_id,
    )
    .await?;
    let telemetry = serde_json::json!({
        "web_vitals": fetch_json_array(db, "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM (SELECT * FROM web_vitals WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t", project_id, visitor_id).await?,
        "scroll_depths": fetch_json_array(db, "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM (SELECT * FROM scroll_depths WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t", project_id, visitor_id).await?,
        "search_queries": fetch_json_array(db, "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM (SELECT * FROM search_queries WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t", project_id, visitor_id).await?,
        "outlinks": fetch_json_array(db, "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM (SELECT * FROM outlinks WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t", project_id, visitor_id).await?,
        "js_errors": fetch_json_array(db, "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM (SELECT * FROM js_errors WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t", project_id, visitor_id).await?,
        "log_entries": fetch_json_array(db, "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM (SELECT * FROM log_entries WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t", project_id, visitor_id).await?,
        "click_events": fetch_json_array(db, "SELECT COALESCE(jsonb_agg(to_jsonb(t)), '[]'::jsonb) FROM (SELECT * FROM click_events WHERE project_id = $1 AND visitor_id = $2 ORDER BY created_at DESC) t", project_id, visitor_id).await?,
    });

    Ok(serde_json::json!({
        "project_id": project_id,
        "visitor_id": visitor_id,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "identity": {
            "profiles": user_profile,
            "aliases": user_aliases,
        },
        "sessions": sessions,
        "pageviews": pageviews,
        "events": events,
        "goal_conversions": goal_conversions,
        "survey_responses": survey_responses,
        "experiment_assignments": experiment_assignments,
        "session_recordings": session_recordings,
        "telemetry": telemetry,
    }))
}

pub async fn delete_visitor_data(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
) -> AppResult<DsarDeleteResult> {
    let tables = [
        "session_recordings",
        "survey_responses",
        "experiment_assignments",
        "goal_conversions",
        "web_vitals",
        "scroll_depths",
        "search_queries",
        "outlinks",
        "js_errors",
        "log_entries",
        "click_events",
        "pageviews",
        "events",
        "sessions",
        "user_aliases",
        "user_profiles",
    ];

    let mut deleted = serde_json::Map::new();
    for table in tables {
        let query = format!("DELETE FROM {table} WHERE project_id = $1 AND visitor_id = $2");
        let result = sqlx::query(&query)
            .bind(project_id)
            .bind(visitor_id)
            .execute(db)
            .await?;
        deleted.insert(table.to_string(), serde_json::json!(result.rows_affected()));
    }

    Ok(DsarDeleteResult {
        visitor_id: visitor_id.to_string(),
        deleted: serde_json::Value::Object(deleted),
    })
}

fn default_privacy_settings(project_id: Uuid) -> PrivacySettings {
    let now = Utc::now();
    PrivacySettings {
        project_id,
        anonymize_ip: true,
        respect_dnt: true,
        bot_filtering: true,
        consent_required: false,
        allowed_consent_modes: vec![
            "analytics".to_string(),
            "measurement".to_string(),
            "all".to_string(),
        ],
        blocked_user_agents: Vec::new(),
        created_at: now,
        updated_at: now,
    }
}

fn skip(reason: &str) -> IngestPrivacyDecision {
    IngestPrivacyDecision {
        accepted: false,
        reason: Some(reason.to_string()),
    }
}

fn validate_consent_modes(modes: &[String]) -> AppResult<()> {
    for mode in modes {
        if mode.trim().is_empty() {
            return Err(AppError::BadRequest(
                "allowed_consent_modes cannot contain empty values".to_string(),
            ));
        }
    }
    Ok(())
}

async fn fetch_json_array(
    db: &PgPool,
    sql: &str,
    project_id: Uuid,
    visitor_id: &str,
) -> AppResult<serde_json::Value> {
    let row: (serde_json::Value,) = sqlx::query_as(sql)
        .bind(project_id)
        .bind(visitor_id)
        .fetch_one(db)
        .await?;
    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use super::{
        anonymize_ip, default_privacy_settings, ingest_privacy_decision, is_bot_user_agent,
    };

    #[test]
    fn anonymizes_ipv4_and_ipv6() {
        assert_eq!(anonymize_ip("203.0.113.42"), "203.0.113.0");
        assert_eq!(
            anonymize_ip("2001:db8:abcd:1234:5678:9abc:def0:1111"),
            "2001:db8:abcd::"
        );
    }

    #[test]
    fn detects_default_and_custom_bots() {
        assert!(is_bot_user_agent("Googlebot/2.1", &[]));
        assert!(is_bot_user_agent(
            "SyntheticBrowser",
            &["syntheticbrowser".to_string()]
        ));
        assert!(!is_bot_user_agent("Mozilla/5.0 Safari/605.1.15", &[]));
    }

    #[test]
    fn enforces_consent_and_dnt() {
        let mut settings = default_privacy_settings(uuid::Uuid::nil());
        assert!(!ingest_privacy_decision(&settings, "Mozilla", true, None, None).accepted);

        settings.respect_dnt = false;
        settings.consent_required = true;
        assert!(!ingest_privacy_decision(&settings, "Mozilla", false, None, None).accepted);
        assert!(
            !ingest_privacy_decision(&settings, "Mozilla", false, Some("ads"), Some(true),)
                .accepted
        );
        assert!(
            ingest_privacy_decision(&settings, "Mozilla", false, Some("analytics"), Some(true),)
                .accepted
        );
    }
}
