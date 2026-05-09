use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionRecording {
    pub id: Uuid,
    pub project_id: Uuid,
    pub session_id: Uuid,
    pub visitor_id: String,
    pub events_data: serde_json::Value,
    pub events_count: i32,
    pub started_at: DateTime<Utc>,
    pub duration_ms: Option<i64>,
    pub entry_page: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub device: Option<String>,
    pub country: Option<String>,
    pub screen: Option<String>,
    pub is_complete: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionRecordingSummary {
    pub id: Uuid,
    pub session_id: Uuid,
    pub visitor_id: String,
    pub events_count: i32,
    pub started_at: DateTime<Utc>,
    pub duration_ms: Option<i64>,
    pub entry_page: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub device: Option<String>,
    pub country: Option<String>,
    pub screen: Option<String>,
    pub is_complete: Option<bool>,
    pub created_at: DateTime<Utc>,
}

const RECORDING_COLUMNS: &str = "id, project_id, session_id, visitor_id, events_data, \
    events_count, started_at, duration_ms, entry_page, browser, os, device, country, \
    screen, is_complete, created_at";

pub async fn record_replay_events(
    db: &PgPool,
    project_id: Uuid,
    session_id: Uuid,
    visitor_id: &str,
    events: &serde_json::Value,
    started_at: DateTime<Utc>,
    duration_ms: Option<i64>,
    entry_page: Option<&str>,
    browser: Option<&str>,
    os: Option<&str>,
    device: Option<&str>,
    country: Option<&str>,
    screen: Option<&str>,
    is_complete: bool,
) -> AppResult<SessionRecording> {
    let events_data = if events.is_array() {
        events.clone()
    } else {
        serde_json::json!([events])
    };
    let events_count = events_data
        .as_array()
        .map(|items| items.len() as i32)
        .unwrap_or(0);

    let recording: SessionRecording = sqlx::query_as(&format!(
        "INSERT INTO session_recordings \
         (project_id, session_id, visitor_id, events_data, events_count, started_at, \
          duration_ms, entry_page, browser, os, device, country, screen, is_complete) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
         ON CONFLICT (project_id, session_id) DO UPDATE SET \
             visitor_id = EXCLUDED.visitor_id, \
             events_data = CASE \
                 WHEN jsonb_typeof(session_recordings.events_data) = 'array' \
                 THEN session_recordings.events_data \
                 ELSE '[]'::jsonb \
             END || EXCLUDED.events_data, \
             events_count = session_recordings.events_count + EXCLUDED.events_count, \
             started_at = LEAST(session_recordings.started_at, EXCLUDED.started_at), \
             duration_ms = GREATEST(COALESCE(session_recordings.duration_ms, 0), COALESCE(EXCLUDED.duration_ms, 0)), \
             entry_page = COALESCE(session_recordings.entry_page, EXCLUDED.entry_page), \
             browser = COALESCE(session_recordings.browser, EXCLUDED.browser), \
             os = COALESCE(session_recordings.os, EXCLUDED.os), \
             device = COALESCE(session_recordings.device, EXCLUDED.device), \
             country = COALESCE(session_recordings.country, EXCLUDED.country), \
             screen = COALESCE(session_recordings.screen, EXCLUDED.screen), \
             is_complete = COALESCE(session_recordings.is_complete, false) OR COALESCE(EXCLUDED.is_complete, false) \
         RETURNING {RECORDING_COLUMNS}"
    ))
    .bind(project_id)
    .bind(session_id)
    .bind(visitor_id)
    .bind(events_data)
    .bind(events_count)
    .bind(started_at)
    .bind(duration_ms)
    .bind(entry_page)
    .bind(browser)
    .bind(os)
    .bind(device)
    .bind(country)
    .bind(screen)
    .bind(is_complete)
    .fetch_one(db)
    .await?;

    Ok(recording)
}

pub async fn list_recordings(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<SessionRecordingSummary>> {
    let recordings: Vec<SessionRecordingSummary> = sqlx::query_as(
        "SELECT id, session_id, visitor_id, events_count, started_at, duration_ms, \
         entry_page, browser, os, device, country, screen, is_complete, created_at \
         FROM session_recordings \
         WHERE project_id = $1 AND started_at >= $2 AND started_at <= $3 \
         ORDER BY started_at DESC \
         LIMIT $4 OFFSET $5",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(recordings)
}

pub async fn get_recording(
    db: &PgPool,
    project_id: Uuid,
    recording_id: Uuid,
) -> AppResult<SessionRecording> {
    let recording: Option<SessionRecording> = sqlx::query_as(&format!(
        "SELECT {RECORDING_COLUMNS} FROM session_recordings \
         WHERE id = $1 AND project_id = $2"
    ))
    .bind(recording_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    recording.ok_or_else(|| AppError::NotFound("Session recording not found".to_string()))
}
