use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Extension;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::models::buffered::{
    BufferedClickEvent, BufferedJsError, BufferedLogEntry, BufferedOutlink, BufferedScrollDepth,
    BufferedSearchQuery, BufferedWebVital,
};
use crate::models::event::BufferedEvent;
use crate::models::pageview::BufferedPageview;
use crate::services::{
    alerts, destinations, error_tracking, geo, goals, governance, identity, ingestion, modules,
    privacy, session, session_replay, surveys, ua,
};
use crate::state::SharedState;
use pulse_common::types::{CollectEnvelope, CollectRequest};
use uuid::Uuid;

const MAX_COLLECT_BATCH_SIZE: usize = 100;

#[derive(Debug, Deserialize)]
pub struct CollectBatchEnvelope {
    #[serde(default, alias = "batch")]
    pub events: Vec<CollectEnvelope>,
}

#[derive(Debug)]
struct CollectOutcome {
    tracked: bool,
    reason: Option<String>,
}

#[derive(Debug, Clone)]
struct IngestRequestContext {
    user_agent: String,
    parsed_ua: ua::ParsedUA,
    geo_result: geo::GeoResult,
    privacy_settings: privacy::PrivacySettings,
    dnt_enabled: bool,
}

pub async fn collect(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::Json(envelope): axum::Json<CollectEnvelope>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("ingest")?;
    let context = build_ingest_context(&state, auth.project_id, &headers, addr).await?;
    let outcome = process_collect_envelope(&state, &auth, &context, envelope).await?;

    if outcome.tracked {
        Ok(axum::Json(json!({ "ok": true, "tracked": true })))
    } else {
        Ok(axum::Json(json!({
            "ok": true,
            "tracked": false,
            "reason": outcome.reason,
        })))
    }
}

pub async fn collect_batch(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::Json(batch): axum::Json<CollectBatchEnvelope>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("ingest")?;

    if batch.events.is_empty() {
        return Err(AppError::BadRequest(
            "Batch requires at least one event".to_string(),
        ));
    }
    if batch.events.len() > MAX_COLLECT_BATCH_SIZE {
        return Err(AppError::BadRequest(format!(
            "Batch supports at most {MAX_COLLECT_BATCH_SIZE} events"
        )));
    }

    let context = build_ingest_context(&state, auth.project_id, &headers, addr).await?;
    let received = batch.events.len();
    let mut tracked = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();

    for (index, envelope) in batch.events.into_iter().enumerate() {
        match process_collect_envelope(&state, &auth, &context, envelope).await {
            Ok(outcome) if outcome.tracked => tracked += 1,
            Ok(_) => skipped += 1,
            Err(err) => errors.push(json!({
                "index": index,
                "error": collect_error_message(&err),
            })),
        }
    }

    Ok(axum::Json(json!({
        "ok": errors.is_empty(),
        "received": received,
        "tracked": tracked,
        "skipped": skipped,
        "failed": errors.len(),
        "errors": errors,
    })))
}

fn collect_error_message(err: &AppError) -> String {
    match err {
        AppError::Database(_) | AppError::Redis(_) | AppError::Internal(_) => {
            "Internal server error".to_string()
        }
        _ => err.to_string(),
    }
}

async fn build_ingest_context(
    state: &SharedState,
    project_id: Uuid,
    headers: &HeaderMap,
    addr: SocketAddr,
) -> AppResult<IngestRequestContext> {
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let raw_client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let privacy_settings = privacy::get_privacy_settings(&state.db, project_id).await?;
    let dnt_enabled = headers
        .get("dnt")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "1")
        || headers
            .get("sec-gpc")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v == "1");

    let client_ip = if privacy_settings.anonymize_ip {
        privacy::anonymize_ip(&raw_client_ip)
    } else {
        raw_client_ip
    };

    // Parse User-Agent
    let parsed_ua = ua::parse_user_agent(&user_agent);

    // GeoIP lookup
    let mut geo_result = if let Some(ref reader) = state.geoip {
        geo::lookup_ip(reader, &client_ip)
    } else {
        geo::GeoResult::default()
    };
    if privacy_settings.anonymize_ip {
        privacy::strip_geo_precision(&mut geo_result);
    }

    Ok(IngestRequestContext {
        user_agent,
        parsed_ua,
        geo_result,
        privacy_settings,
        dnt_enabled,
    })
}

async fn process_collect_envelope(
    state: &SharedState,
    auth: &AuthenticatedProject,
    context: &IngestRequestContext,
    envelope: CollectEnvelope,
) -> AppResult<CollectOutcome> {
    let decision = privacy::ingest_privacy_decision(
        &context.privacy_settings,
        &context.user_agent,
        context.dnt_enabled,
        envelope.consent_mode.as_deref(),
        envelope.consent_granted,
    );
    if !decision.accepted {
        return Ok(CollectOutcome {
            tracked: false,
            reason: decision.reason,
        });
    }

    let now = envelope
        .timestamp
        .map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or_else(Utc::now))
        .unwrap_or_else(Utc::now);

    route_to_destinations_async(
        &state,
        auth.project_id,
        &envelope.visitor_id,
        now,
        &envelope.request,
    );

    match envelope.request {
        pulse_common::types::CollectRequest::Pageview { payload } => {
            // Resolve or create session
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &context.parsed_ua,
                &context.geo_result,
                payload.screen.as_deref(),
                payload.language.as_deref(),
                None, // hostname extracted from path if needed
                Some(&payload.path),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            // Extract referrer domain
            let referrer_domain = payload.referrer.as_ref().and_then(|r| {
                url::Url::parse(r)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.to_string()))
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

            if modules::is_module_enabled(&state, auth.project_id, "goals").await? {
                if let Err(e) = goals::evaluate_pageview_goals(
                    &state.db,
                    auth.project_id,
                    &pageview.path,
                    &pageview.visitor_id,
                    session_id,
                )
                .await
                {
                    tracing::warn!("Failed to evaluate pageview goals: {e}");
                }
            }

            // Update session counts (fire-and-forget)
            let db = state.db.clone();
            let path = payload.path;
            tokio::spawn(async move {
                let _ = session::update_session_counts(&db, session_id, true, Some(&path)).await;
            });

            evaluate_alerts_async(&state, auth.project_id);
        }

        pulse_common::types::CollectRequest::Event { payload } => {
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &context.parsed_ua,
                &context.geo_result,
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

            if modules::is_module_enabled(&state, auth.project_id, "governance").await? {
                let validation = governance::validate_event_payload(
                    &state.db,
                    auth.project_id,
                    &event.visitor_id,
                    &event.event_name,
                    event.event_data.as_ref(),
                    now,
                )
                .await?;

                if !validation.accepted {
                    let message = validation
                        .violations
                        .iter()
                        .map(|violation| violation.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(AppError::BadRequest(format!(
                        "Event failed tracking plan validation: {message}"
                    )));
                }
            }

            ingestion::push_event(&state, &event)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            ingestion::update_realtime(&state, auth.project_id, &event.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            if modules::is_module_enabled(&state, auth.project_id, "goals").await? {
                if let Err(e) = goals::evaluate_event_goals(
                    &state.db,
                    auth.project_id,
                    &event.event_name,
                    event.event_data.as_ref(),
                    &event.visitor_id,
                    session_id,
                    event.revenue_amount,
                )
                .await
                {
                    tracing::warn!("Failed to evaluate event goals: {e}");
                }
            }

            let db = state.db.clone();
            tokio::spawn(async move {
                let _ = session::update_session_counts(&db, session_id, false, None).await;
            });

            evaluate_alerts_async(&state, auth.project_id);
        }

        pulse_common::types::CollectRequest::Identify { payload } => {
            identity::identify_user(
                &state.db,
                auth.project_id,
                &envelope.visitor_id,
                payload.user_id.as_deref(),
                &payload.traits,
                payload.account_id.as_deref(),
                payload.account_name.as_deref(),
                payload.account_traits.as_ref(),
                payload.account_role.as_deref(),
                now,
            )
            .await?;

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::WebVital { payload } => {
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &context.parsed_ua,
                &context.geo_result,
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

            evaluate_alerts_async(&state, auth.project_id);
        }

        pulse_common::types::CollectRequest::ScrollDepth { payload } => {
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &context.parsed_ua,
                &context.geo_result,
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
                &context.parsed_ua,
                &context.geo_result,
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
                &context.parsed_ua,
                &context.geo_result,
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
                &context.parsed_ua,
                &context.geo_result,
                None,
                None,
                None,
                payload.path.as_deref(),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if modules::is_module_enabled(&state, auth.project_id, "js_errors").await? {
                let fingerprint = error_tracking::error_fingerprint(
                    &payload.message,
                    payload.filename.as_deref(),
                    payload.lineno,
                    payload.stack.as_deref(),
                );
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
                    browser: context.parsed_ua.browser.clone(),
                    os: context.parsed_ua.os.clone(),
                    release: payload.release,
                    environment: payload.environment,
                    fingerprint,
                    created_at: now,
                };

                ingestion::push_js_error(&state, &js_error)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            evaluate_alerts_async(&state, auth.project_id);
        }

        pulse_common::types::CollectRequest::Log { payload } => {
            let level = error_tracking::normalize_log_level(&payload.level)?;
            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &context.parsed_ua,
                &context.geo_result,
                None,
                None,
                None,
                payload.path.as_deref(),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if modules::is_module_enabled(&state, auth.project_id, "logs").await? {
                let log_entry = BufferedLogEntry {
                    project_id: auth.project_id,
                    visitor_id: envelope.visitor_id.clone(),
                    session_id,
                    level,
                    message: payload.message,
                    body: error_tracking::log_body(payload.body),
                    path: payload.path,
                    browser: context.parsed_ua.browser.clone(),
                    os: context.parsed_ua.os.clone(),
                    release: payload.release,
                    environment: payload.environment,
                    created_at: now,
                };

                ingestion::push_log_entry(&state, &log_entry)
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
                &context.parsed_ua,
                &context.geo_result,
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

        pulse_common::types::CollectRequest::SessionReplay { payload } => {
            let started_at = payload
                .started_at
                .map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or(now))
                .unwrap_or(now);

            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &context.parsed_ua,
                &context.geo_result,
                payload.screen.as_deref(),
                None,
                None,
                payload.entry_page.as_deref(),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if modules::is_module_enabled(&state, auth.project_id, "session_replay").await? {
                session_replay::record_replay_events(
                    &state.db,
                    auth.project_id,
                    session_id,
                    &envelope.visitor_id,
                    &payload.events,
                    started_at,
                    payload.duration_ms,
                    payload.entry_page.as_deref(),
                    context.parsed_ua.browser.as_deref(),
                    context.parsed_ua.os.as_deref(),
                    context.parsed_ua.device.as_deref(),
                    context.geo_result.country.as_deref(),
                    payload.screen.as_deref(),
                    payload.is_complete.unwrap_or(false),
                )
                .await?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        pulse_common::types::CollectRequest::SurveyResponse { payload } => {
            let survey_id = Uuid::parse_str(&payload.survey_id)
                .map_err(|_| AppError::BadRequest("Invalid survey_id".to_string()))?;

            let session_id = session::resolve_session(
                &state,
                auth.project_id,
                &envelope.visitor_id,
                &context.parsed_ua,
                &context.geo_result,
                None,
                None,
                None,
                payload.path.as_deref(),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if modules::is_module_enabled(&state, auth.project_id, "surveys").await? {
                surveys::record_response(
                    &state.db,
                    auth.project_id,
                    survey_id,
                    &envelope.visitor_id,
                    Some(session_id),
                    &payload.answers,
                    payload.completed.unwrap_or(true),
                    payload.path.as_deref(),
                )
                .await
                .map_err(|e| match e {
                    sqlx::Error::RowNotFound => {
                        AppError::NotFound("Active survey not found".to_string())
                    }
                    other => AppError::Internal(other.to_string()),
                })?;
            }

            ingestion::update_realtime(&state, auth.project_id, &envelope.visitor_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }

    Ok(CollectOutcome {
        tracked: true,
        reason: None,
    })
}

fn evaluate_alerts_async(state: &SharedState, project_id: Uuid) {
    let state = state.clone();
    tokio::spawn(async move {
        match modules::is_module_enabled(&state, project_id, "alerts").await {
            Ok(true) => alerts::evaluate_alerts(&state, project_id).await,
            Ok(false) => {}
            Err(e) => tracing::warn!("Failed to check alerts module before evaluation: {e}"),
        }
    });
}

fn route_to_destinations_async(
    state: &SharedState,
    project_id: Uuid,
    visitor_id: &str,
    timestamp: chrono::DateTime<Utc>,
    request: &CollectRequest,
) {
    let event_type = collect_event_type(request).to_string();
    let payload = json!({
        "event": event_type.clone(),
        "project_id": project_id,
        "visitor_id": visitor_id,
        "timestamp": timestamp.to_rfc3339(),
        "data": request,
    });
    let state = state.clone();
    tokio::spawn(async move {
        match modules::is_module_enabled(&state, project_id, "destinations").await {
            Ok(true) => {
                if let Err(e) =
                    destinations::enqueue_event(&state.db, project_id, &event_type, payload).await
                {
                    tracing::warn!("Failed to enqueue destination delivery: {e}");
                }
            }
            Ok(false) => {}
            Err(e) => tracing::warn!("Failed to check destinations module: {e}"),
        }
    });
}

fn collect_event_type(request: &CollectRequest) -> &'static str {
    match request {
        CollectRequest::Pageview { .. } => "pageview",
        CollectRequest::Event { .. } => "event",
        CollectRequest::Identify { .. } => "identify",
        CollectRequest::WebVital { .. } => "web_vital",
        CollectRequest::ScrollDepth { .. } => "scroll_depth",
        CollectRequest::SearchQuery { .. } => "search_query",
        CollectRequest::Outlink { .. } => "outlink",
        CollectRequest::JsError { .. } => "js_error",
        CollectRequest::Log { .. } => "log",
        CollectRequest::ClickEvent { .. } => "click_event",
        CollectRequest::SurveyResponse { .. } => "survey_response",
        CollectRequest::SessionReplay { .. } => "session_replay",
    }
}
