use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Extension;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::services::query as qsvc;
use crate::state::SharedState;

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TimeseriesQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PaginatedQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

// GET /api/v1/stats
pub async fn get_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<StatsQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;

    let duration = params.end_at - params.start_at;
    let prev_start = params.start_at - duration;
    let prev_end = params.start_at;
    let today = Utc::now().date_naive();

    let current = qsvc::fetch_stats(&state.db, auth.project_id, params.start_at, params.end_at, today).await?;
    let previous = qsvc::fetch_stats(&state.db, auth.project_id, prev_start, prev_end, today).await?;
    let events_current = qsvc::fetch_events_count(&state.db, auth.project_id, params.start_at, params.end_at, today).await?;
    let events_prev = qsvc::fetch_events_count(&state.db, auth.project_id, prev_start, prev_end, today).await?;

    let mut result = json!({
        "pageviews": { "value": current.0, "prev": previous.0 },
        "visitors": { "value": current.1, "prev": previous.1 },
        "sessions": { "value": current.2, "prev": previous.2 },
        "bounce_rate": {
            "value": if current.2 > 0 { (current.3 as f64) / (current.2 as f64) * 100.0 } else { 0.0 },
            "prev": if previous.2 > 0 { (previous.3 as f64) / (previous.2 as f64) * 100.0 } else { 0.0 },
        },
        "avg_duration": {
            "value": if current.2 > 0 { (current.4 as f64) / (current.2 as f64) / 1000.0 } else { 0.0 },
            "prev": if previous.2 > 0 { (previous.4 as f64) / (previous.2 as f64) / 1000.0 } else { 0.0 },
        },
        "events": { "value": events_current, "prev": events_prev },
    });

    if let Some(ref umami) = state.umami {
        if let Some(website_id) = qsvc::get_umami_website_id(&state, auth.project_id).await {
            let start_ms = params.start_at.timestamp_millis();
            let end_ms = params.end_at.timestamp_millis();
            if let Ok(umami_stats) = umami.get_stats(&state, &website_id, start_ms, end_ms).await {
                let map = result.as_object_mut().unwrap();
                map.insert("umami".to_string(), json!({
                    "pageviews": umami_stats.pageviews,
                    "visitors": umami_stats.visitors,
                    "sessions": umami_stats.visits,
                    "bounces": umami_stats.bounces,
                    "totaltime": umami_stats.totaltime,
                }));
            }
        }
    }

    Ok(axum::Json(result))
}

// GET /api/v1/stats/timeseries
pub async fn get_timeseries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<TimeseriesQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    let today = Utc::now().date_naive();
    let data = qsvc::fetch_timeseries(&state.db, auth.project_id, params.start_at, params.end_at, today).await?;
    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/pages
pub async fn get_pages(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    let today = Utc::now().date_naive();
    let data = qsvc::fetch_pages(&state.db, auth.project_id, params.start_at, params.end_at, today, params.limit, params.offset).await?;

    if let Some(ref umami) = state.umami {
        if let Some(website_id) = qsvc::get_umami_website_id(&state, auth.project_id).await {
            let start_ms = params.start_at.timestamp_millis();
            let end_ms = params.end_at.timestamp_millis();
            if let Ok(umami_pages) = umami.get_pageviews(&state, &website_id, start_ms, end_ms, params.limit).await {
                let result = qsvc::merge_page_data(data, umami_pages);
                return Ok(axum::Json(json!({ "data": result })));
            }
        }
    }

    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/referrers
pub async fn get_referrers(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    let today = Utc::now().date_naive();
    let mut data = qsvc::fetch_referrers(&state.db, auth.project_id, params.start_at, params.end_at, today, params.limit, params.offset).await?;

    if let Some(ref umami) = state.umami {
        if let Some(website_id) = qsvc::get_umami_website_id(&state, auth.project_id).await {
            let start_ms = params.start_at.timestamp_millis();
            let end_ms = params.end_at.timestamp_millis();
            if let Ok(umami_refs) = umami.get_referrers(&state, &website_id, start_ms, end_ms, params.limit).await {
                data = qsvc::merge_kv_data(
                    data,
                    "referrer_domain",
                    "visits",
                    &umami_refs.iter().map(|r| (r.x.clone(), r.y)).collect::<Vec<_>>(),
                );
            }
        }
    }

    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/events
pub async fn get_events(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    let today = Utc::now().date_naive();
    let data = qsvc::fetch_events(&state.db, auth.project_id, params.start_at, params.end_at, today, params.limit, params.offset).await?;
    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/devices
pub async fn get_devices(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    let today = Utc::now().date_naive();
    let data = qsvc::fetch_devices(&state.db, auth.project_id, params.start_at, params.end_at, today, params.limit, params.offset).await?;
    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/geo
pub async fn get_geo(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    let today = Utc::now().date_naive();
    let mut data = qsvc::fetch_geo(&state.db, auth.project_id, params.start_at, params.end_at, today, params.limit, params.offset).await?;

    if let Some(ref umami) = state.umami {
        if let Some(website_id) = qsvc::get_umami_website_id(&state, auth.project_id).await {
            let start_ms = params.start_at.timestamp_millis();
            let end_ms = params.end_at.timestamp_millis();
            if let Ok(umami_countries) = umami.get_countries(&state, &website_id, start_ms, end_ms, params.limit).await {
                data = qsvc::merge_kv_data(
                    data,
                    "country",
                    "visitors",
                    &umami_countries.iter().map(|c| (c.x.clone(), c.y)).collect::<Vec<_>>(),
                );
            }
        }
    }

    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/realtime
pub async fn get_realtime(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;

    let active_visitors = qsvc::fetch_realtime(&state, auth.project_id).await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let mut result = json!({ "active_visitors": active_visitors });

    if let Some(ref umami) = state.umami {
        if let Some(website_id) = qsvc::get_umami_website_id(&state, auth.project_id).await {
            if let Ok(umami_active) = umami.get_active_visitors(&state, &website_id).await {
                let map = result.as_object_mut().unwrap();
                map.insert("umami_active_visitors".to_string(), json!(umami_active.x));
                let total = active_visitors + umami_active.x;
                map.insert("total_active_visitors".to_string(), json!(total));
            }
        }
    }

    Ok(axum::Json(result))
}
