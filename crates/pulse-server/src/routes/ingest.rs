use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Extension;
use chrono::Utc;
use serde_json::json;
use std::net::SocketAddr;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::models::buffered::{
    BufferedClickEvent, BufferedJsError, BufferedOutlink, BufferedScrollDepth, BufferedSearchQuery,
    BufferedWebVital,
};
use crate::models::event::BufferedEvent;
use crate::models::pageview::BufferedPageview;
use crate::services::{geo, ingestion, modules, session, ua};
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
                utm_source: payload.utm_source,
                utm_medium: payload.utm_medium,
                utm_campaign: payload.utm_campaign,
                utm_content: payload.utm_content,
                utm_term: payload.utm_term,
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
                revenue_amount: payload.revenue_amount,
                revenue_currency: payload.revenue_currency,
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

        pulse_common::types::CollectRequest::WebVital { payload } => {
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

            // Check if module is enabled; if disabled, silently accept
            if modules::is_module_enabled(&state, auth.project_id, "web_vitals").await? {
                let vital = BufferedWebVital {
                    project_id: auth.project_id,
                    visitor_id: envelope.visitor_id.clone(),
                    session_id,
                    path: payload.path,
                    metric_name: payload.name,
                    metric_value: payload.value,
                    rating: payload.rating,
                    created_at: now,
                };

                ingestion::push_web_vital(&state, &vital)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::ScrollDepth { payload } => {
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &parsed_ua,
                &geo_result,
                None,
                None,
                None,
                Some(&payload.path),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if modules::is_module_enabled(&state, auth.project_id, "scroll_depth").await? {
                let scroll = BufferedScrollDepth {
                    project_id: auth.project_id,
                    visitor_id: envelope.visitor_id.clone(),
                    session_id,
                    path: payload.path,
                    max_depth: payload.max_depth,
                    created_at: now,
                };

                ingestion::push_scroll_depth(&state, &scroll)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::SearchQuery { payload } => {
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

            if modules::is_module_enabled(&state, auth.project_id, "search_queries").await? {
                let search = BufferedSearchQuery {
                    project_id: auth.project_id,
                    visitor_id: envelope.visitor_id.clone(),
                    session_id,
                    query: payload.query,
                    results_count: payload.results_count,
                    path: payload.path,
                    created_at: now,
                };

                ingestion::push_search_query(&state, &search)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::Outlink { payload } => {
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

            if modules::is_module_enabled(&state, auth.project_id, "outlinks").await? {
                let outlink = BufferedOutlink {
                    project_id: auth.project_id,
                    visitor_id: envelope.visitor_id.clone(),
                    session_id,
                    url: payload.url,
                    link_type: payload.link_type,
                    path: payload.path,
                    created_at: now,
                };

                ingestion::push_outlink(&state, &outlink)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::JsError { payload } => {
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

            if modules::is_module_enabled(&state, auth.project_id, "js_errors").await? {
                let js_error = BufferedJsError {
                    project_id: auth.project_id,
                    visitor_id: envelope.visitor_id.clone(),
                    session_id,
                    message: payload.message,
                    stack: payload.stack,
                    filename: payload.filename,
                    lineno: payload.lineno,
                    colno: payload.colno,
                    path: payload.path,
                    browser: parsed_ua.browser.clone(),
                    os: parsed_ua.os.clone(),
                    created_at: now,
                };

                ingestion::push_js_error(&state, &js_error)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::ClickEvent { payload } => {
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &parsed_ua,
                &geo_result,
                None,
                None,
                None,
                Some(&payload.path),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if modules::is_module_enabled(&state, auth.project_id, "click_events").await? {
                let click = BufferedClickEvent {
                    project_id: auth.project_id,
                    visitor_id: envelope.visitor_id.clone(),
                    session_id,
                    path: payload.path,
                    x: payload.x,
                    y: payload.y,
                    element_selector: payload.element_selector,
                    viewport_width: payload.viewport_width,
                    viewport_height: payload.viewport_height,
                    created_at: now,
                };

                ingestion::push_click_event(&state, &click)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::SurveyResponse { payload: _ } => {
            // SurveyResponse is acknowledged but does not have a dedicated buffer/table yet.
            // The data is accepted to avoid client-side errors when the module is enabled.
            // Storage will be implemented when the survey_responses table is created.
        }
    }

    Ok(axum::Json(json!({ "ok": true })))
}
