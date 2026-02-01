use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Extension;
use chrono::Utc;
use serde_json::json;
use std::net::SocketAddr;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::models::event::BufferedEvent;
use crate::models::pageview::BufferedPageview;
use crate::services::{geo, ingestion, session, ua};
use crate::state::SharedState;
use pulse_common::types::CollectEnvelope;

pub async fn collect(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::Json(envelope): axum::Json<CollectEnvelope>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("ingest")?;

    // Extract client info
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    // Parse User-Agent
    let parsed_ua = ua::parse_user_agent(user_agent);

    // GeoIP lookup
    let geo_result = if let Some(ref reader) = state.geoip {
        geo::lookup_ip(reader, &client_ip)
    } else {
        geo::GeoResult::default()
    };

    let now = envelope
        .timestamp
        .map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or_else(Utc::now))
        .unwrap_or_else(Utc::now);

    match envelope.request {
        pulse_common::types::CollectRequest::Pageview { payload } => {
            // Resolve or create session
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &parsed_ua,
                &geo_result,
                payload.screen.as_deref(),
                payload.language.as_deref(),
                None, // hostname extracted from path if needed
                Some(&payload.path),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            // Extract referrer domain
            let referrer_domain = payload.referrer.as_ref().and_then(|r| {
                url::Url::parse(r).ok().and_then(|u| u.host_str().map(|h| h.to_string()))
            });

            let pageview = BufferedPageview {
                project_id: auth.project_id,
                session_id,
                visitor_id: envelope.visitor_id,
                path: payload.path.clone(),
                title: payload.title,
                referrer: payload.referrer,
                referrer_domain,
                query_params: None,
                duration_ms: None,
                created_at: now,
            };

            // Push to buffer and update realtime
            ingestion::push_pageview(&state, &pageview)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            ingestion::update_realtime(&state, auth.project_id, &pageview.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // Update session counts (fire-and-forget)
            let db = state.db.clone();
            let path = payload.path;
            tokio::spawn(async move {
                let _ = session::update_session_counts(&db, session_id, true, Some(&path)).await;
            });
        }

        pulse_common::types::CollectRequest::Event { payload } => {
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &parsed_ua,
                &geo_result,
                None,
                None,
                None,
                payload.path.as_deref(),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            let event = BufferedEvent {
                project_id: auth.project_id,
                session_id,
                visitor_id: envelope.visitor_id,
                event_name: payload.name,
                event_data: payload.data,
                path: payload.path,
                created_at: now,
            };

            ingestion::push_event(&state, &event)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            ingestion::update_realtime(&state, auth.project_id, &event.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let db = state.db.clone();
            tokio::spawn(async move {
                let _ = session::update_session_counts(&db, session_id, false, None).await;
            });
        }

        pulse_common::types::CollectRequest::Identify { payload: _ } => {
            // Identify is stored as session traits — can be extended later
            // For now, just acknowledge the request
        }
    }

    Ok(axum::Json(json!({ "ok": true })))
}
