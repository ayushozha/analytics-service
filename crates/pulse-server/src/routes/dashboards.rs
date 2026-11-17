use axum::extract::{Path, Query};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::services::dashboards::{
    self, DashboardInput, DashboardListQuery, DashboardPatch, LayoutItem, TileInput, TilePatch,
};
use crate::services::insights::saved::{self, ParsedQuery};
use crate::state::SharedState;

const MODULE: &str = "dashboards";
const IDENTITY_MODULE: &str = "identity";
/// Wrong share passwords allowed per token before a cool-down.
const SHARE_MAX_FAILURES: i64 = 10;
const SHARE_LOCKOUT_SECS: i64 = 15 * 60;

async fn can_read(state: &SharedState, auth: &AuthenticatedProject) -> AppResult<()> {
    auth.require_scope("query")?;
    services::modules::require_module_read(state, auth.project_id, MODULE, &auth.allowed_modules)
        .await
}

async fn can_write(state: &SharedState, auth: &AuthenticatedProject) -> AppResult<()> {
    auth.require_scope("query")?;
    services::modules::require_module_write(state, auth.project_id, MODULE, &auth.allowed_modules)
        .await
}

// GET /api/v2/dashboards
pub async fn list(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(f): Query<DashboardListQuery>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        json!({ "data": dashboards::list(&state.db, auth.project_id, &f).await? }),
    ))
}

// POST /api/v2/dashboards
pub async fn create(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(input): Json<DashboardInput>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        dashboards::create(&state.db, auth.project_id, &input).await?,
    ))
}

// GET /api/v2/dashboard-templates
pub async fn templates(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    let list: Vec<_> = dashboards::templates()
        .into_iter()
        .map(|t| {
            json!({
                "id": t.id, "name": t.name, "description": t.description,
                "tiles": t.tiles.iter().map(|i| json!({"name": i.name, "kind": i.kind})).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(Json(json!({ "data": list })))
}

// GET /api/v2/dashboards/{id}
pub async fn get(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    let d = dashboards::get(&state.db, auth.project_id, id).await?;
    let tiles = dashboards::tiles(&state.db, id).await?;
    Ok(Json(json!({ "dashboard": d, "tiles": tiles })))
}

// PATCH /api/v2/dashboards/{id}
pub async fn update(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(p): Json<DashboardPatch>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        dashboards::update(&state.db, auth.project_id, id, &p).await?,
    ))
}

// DELETE /api/v2/dashboards/{id}
pub async fn delete(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    dashboards::delete(&state.db, auth.project_id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize, Default)]
pub struct DuplicateParams {
    #[serde(default)]
    pub duplicate_insights: bool,
}

// POST /api/v2/dashboards/{id}/duplicate
pub async fn duplicate(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Query(p): Query<DuplicateParams>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        dashboards::duplicate(&state.db, auth.project_id, id, p.duplicate_insights).await?,
    ))
}

// POST /api/v2/dashboards/{id}/tiles
pub async fn add_tile(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(t): Json<TileInput>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        dashboards::add_tile(&state.db, auth.project_id, id, &t).await?,
    ))
}

// PATCH /api/v2/dashboards/{id}/tiles/{tile_id}
pub async fn update_tile(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((id, tile_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<TilePatch>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        dashboards::update_tile(&state.db, auth.project_id, id, tile_id, &p).await?,
    ))
}

// DELETE /api/v2/dashboards/{id}/tiles/{tile_id}
pub async fn delete_tile(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((id, tile_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    dashboards::delete_tile(&state.db, auth.project_id, id, tile_id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct MoveTile {
    pub to_dashboard_id: Uuid,
}

// POST /api/v2/dashboards/{id}/tiles/{tile_id}/move
pub async fn move_tile(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((id, tile_id)): Path<(Uuid, Uuid)>,
    Json(m): Json<MoveTile>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        dashboards::move_tile(&state.db, auth.project_id, id, tile_id, m.to_dashboard_id).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct Layouts {
    pub tiles: Vec<LayoutItem>,
}

// PUT /api/v2/dashboards/{id}/layouts
pub async fn update_layouts(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(l): Json<Layouts>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(json!({
        "data": dashboards::update_layouts(&state.db, auth.project_id, id, &l.tiles).await?
    })))
}

#[derive(Debug, Deserialize, Default)]
pub struct RunParams {
    #[serde(default)]
    pub refresh: bool,
}

// GET /api/v2/dashboards/{id}/run
pub async fn run(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Query(p): Query<RunParams>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    services::modules::require_module_read(&state, auth.project_id, "core", &auth.allowed_modules)
        .await?;
    // Tiles that read person properties need identity access; other tiles
    // still render if the key lacks it.
    let identity_ok = services::modules::require_module_read(
        &state,
        auth.project_id,
        IDENTITY_MODULE,
        &auth.allowed_modules,
    )
    .await
    .is_ok();
    let gate = move |q: &ParsedQuery| {
        (!identity_ok && q.reads_person_properties())
            .then(|| "this tile needs access to the identity module".to_string())
    };
    let d = dashboards::get(&state.db, auth.project_id, id).await?;
    let tiles = dashboards::run(&state.db, auth.project_id, &d, p.refresh, &gate).await?;
    Ok(Json(json!({ "dashboard": d, "tiles": tiles })))
}

/// Public view of a share link: a dashboard, a saved insight, or (legacy
/// links) last-30-day overview stats. Password-protected links take the
/// password in the `x-share-password` header.
// GET /api/shared/{token}
pub async fn shared(
    Extension(state): Extension<SharedState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    let link = services::sharing::resolve_shared_token(&state.db, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("Share link not found or expired".into()))?;

    if link.password_hash.is_some() {
        let fail_key = state.redis_key(&format!("share:fail:{}", link.id));
        let mut redis = state.redis.clone();
        let failures: i64 = redis.get(&fail_key).await.unwrap_or(0);
        if failures >= SHARE_MAX_FAILURES {
            return Err(AppError::RateLimited);
        }
        let given = headers
            .get("x-share-password")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !services::sharing::verify_shared_password(&link, given) {
            let n: i64 = redis.incr(&fail_key, 1).await.unwrap_or(1);
            if n == 1 {
                let _: Result<(), _> = redis.expire(&fail_key, SHARE_LOCKOUT_SECS).await;
            }
            return Err(AppError::Unauthorized);
        }
    }

    // The public view never exposes more than the link's creator could read.
    let allow_person = link.allow_person_data;
    let gate = move |q: &ParsedQuery| {
        (!allow_person && q.reads_person_properties())
            .then(|| "this tile uses person data, which this link may not show".to_string())
    };
    if let Some(dashboard_id) = link.dashboard_id {
        let d = dashboards::get(&state.db, link.project_id, dashboard_id).await?;
        let tiles = dashboards::run(&state.db, link.project_id, &d, false, &gate).await?;
        return Ok(Json(
            json!({ "type": "dashboard", "name": link.name, "dashboard": d, "tiles": tiles }),
        ));
    }
    if let Some(insight_id) = link.insight_id {
        let insight = saved::get(&state.db, link.project_id, insight_id).await?;
        let parsed = saved::parse_saved(&insight)?;
        if let Some(msg) = gate(&parsed) {
            return Err(AppError::Forbidden(msg));
        }
        let r = saved::run_saved(&state.db, link.project_id, insight, &parsed, false).await?;
        return Ok(Json(
            json!({ "type": "insight", "name": link.name, "insight": r }),
        ));
    }
    let end = Utc::now();
    let start = end - Duration::days(30);
    let today = end.date_naive();
    let stats = services::query::fetch_stats(&state.db, link.project_id, start, end, today).await?;
    let pages =
        services::query::fetch_pages(&state.db, link.project_id, start, end, today, 10, 0).await?;
    Ok(Json(json!({
        "type": "overview",
        "name": link.name,
        "modules": link.modules,
        "range": { "start_at": start, "end_at": end },
        "stats": { "pageviews": stats.0, "visitors": stats.1, "sessions": stats.2, "bounces": stats.3 },
        "top_pages": pages,
    })))
}
