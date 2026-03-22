use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::Extension;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::state::SharedState;

// ─── Query parameter structs ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RetentionQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(default = "default_period")]
    pub period: String,
}

fn default_period() -> String {
    "daily".to_string()
}

#[derive(Debug, Deserialize)]
pub struct CohortQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(default = "default_group_by")]
    pub group_by: String,
    #[serde(default = "default_metric")]
    pub metric: String,
}

fn default_group_by() -> String {
    "week".to_string()
}

fn default_metric() -> String {
    "pageviews".to_string()
}

#[derive(Debug, Deserialize)]
pub struct PathsQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub path: String,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_direction() -> String {
    "forward".to_string()
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct CampaignTimeseriesQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub utm_source: String,
}

#[derive(Debug, Deserialize)]
pub struct PaginatedDateQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateFunnelRequest {
    pub name: String,
    pub steps: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFunnelRequest {
    pub name: String,
    pub steps: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateGoalRequest {
    pub name: String,
    pub goal_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGoalRequest {
    pub name: String,
    pub goal_type: String,
    pub config: serde_json::Value,
}

// ─── Funnels ────────────────────────────────────────────────────────────────

// GET /api/v1/funnels
pub async fn list_funnels(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "funnels",
        &auth.allowed_modules,
    )
    .await?;

    let funnels = services::funnels::list_funnels(&state.db, auth.project_id).await?;
    Ok(axum::Json(json!({ "data": funnels })))
}

// POST /api/v1/funnels
pub async fn create_funnel(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateFunnelRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "funnels",
        &auth.allowed_modules,
    )
    .await?;

    let funnel =
        services::funnels::create_funnel(&state.db, auth.project_id, &input.name, input.steps)
            .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(funnel)))
}

// GET /api/v1/funnels/{id}
pub async fn get_funnel(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(funnel_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "funnels",
        &auth.allowed_modules,
    )
    .await?;

    let funnel = services::funnels::get_funnel(&state.db, auth.project_id, funnel_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Funnel not found".to_string()))?;

    Ok(axum::Json(funnel))
}

// PUT /api/v1/funnels/{id}
pub async fn update_funnel(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(funnel_id): Path<Uuid>,
    axum::Json(input): axum::Json<UpdateFunnelRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "funnels",
        &auth.allowed_modules,
    )
    .await?;

    let funnel = services::funnels::update_funnel(
        &state.db,
        auth.project_id,
        funnel_id,
        &input.name,
        input.steps,
    )
    .await?;

    Ok(axum::Json(funnel))
}

// DELETE /api/v1/funnels/{id}
pub async fn delete_funnel(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(funnel_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "funnels",
        &auth.allowed_modules,
    )
    .await?;

    services::funnels::delete_funnel(&state.db, auth.project_id, funnel_id).await?;

    Ok(axum::Json(json!({ "ok": true })))
}

// GET /api/v1/funnels/{id}/analyze
pub async fn analyze_funnel(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(funnel_id): Path<Uuid>,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "funnels",
        &auth.allowed_modules,
    )
    .await?;

    let steps = services::funnels::analyze_funnel(
        &state.db,
        auth.project_id,
        funnel_id,
        params.start_at,
        params.end_at,
    )
    .await?;

    Ok(axum::Json(json!({ "data": steps })))
}

// ─── Goals ──────────────────────────────────────────────────────────────────

// GET /api/v1/goals
pub async fn list_goals(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "goals",
        &auth.allowed_modules,
    )
    .await?;

    let goals = services::goals::list_goals(&state.db, auth.project_id).await?;
    Ok(axum::Json(json!({ "data": goals })))
}

// POST /api/v1/goals
pub async fn create_goal(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateGoalRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "goals",
        &auth.allowed_modules,
    )
    .await?;

    let goal = services::goals::create_goal(
        &state.db,
        auth.project_id,
        &input.name,
        &input.goal_type,
        input.config,
    )
    .await?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(goal)))
}

// GET /api/v1/goals/{id}
pub async fn get_goal(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(goal_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "goals",
        &auth.allowed_modules,
    )
    .await?;

    let goal = services::goals::get_goal(&state.db, auth.project_id, goal_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Goal not found".to_string()))?;

    Ok(axum::Json(goal))
}

// PUT /api/v1/goals/{id}
pub async fn update_goal(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(goal_id): Path<Uuid>,
    axum::Json(input): axum::Json<UpdateGoalRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "goals",
        &auth.allowed_modules,
    )
    .await?;

    let goal = services::goals::update_goal(
        &state.db,
        auth.project_id,
        goal_id,
        &input.name,
        &input.goal_type,
        input.config,
    )
    .await?;

    Ok(axum::Json(goal))
}

// DELETE /api/v1/goals/{id}
pub async fn delete_goal(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(goal_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "goals",
        &auth.allowed_modules,
    )
    .await?;

    services::goals::delete_goal(&state.db, auth.project_id, goal_id).await?;

    Ok(axum::Json(json!({ "ok": true })))
}

// GET /api/v1/goals/{id}/stats
pub async fn get_goal_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(goal_id): Path<Uuid>,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "goals",
        &auth.allowed_modules,
    )
    .await?;

    let stats = services::goals::get_goal_stats(
        &state.db,
        auth.project_id,
        goal_id,
        params.start_at,
        params.end_at,
    )
    .await?;

    Ok(axum::Json(json!({ "data": stats })))
}

// ─── Retention ──────────────────────────────────────────────────────────────

// GET /api/v1/retention
pub async fn get_retention(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<RetentionQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "retention",
        &auth.allowed_modules,
    )
    .await?;

    let data = services::retention_analysis::get_retention(
        &state.db,
        auth.project_id,
        params.start_at,
        params.end_at,
        &params.period,
    )
    .await?;

    Ok(axum::Json(json!({ "data": data })))
}

// ─── Cohorts ────────────────────────────────────────────────────────────────

// GET /api/v1/cohorts
pub async fn get_cohorts(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<CohortQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "cohorts",
        &auth.allowed_modules,
    )
    .await?;

    let data = services::cohorts::get_cohorts(
        &state.db,
        auth.project_id,
        params.start_at,
        params.end_at,
        &params.group_by,
        &params.metric,
    )
    .await?;

    Ok(axum::Json(json!({ "data": data })))
}

// ─── Paths ──────────────────────────────────────────────────────────────────

// GET /api/v1/paths
pub async fn get_paths(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PathsQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "paths",
        &auth.allowed_modules,
    )
    .await?;

    let data = services::paths::get_page_flows(
        &state.db,
        auth.project_id,
        params.start_at,
        params.end_at,
        &params.path,
        &params.direction,
        params.limit,
    )
    .await?;

    Ok(axum::Json(json!({ "data": data })))
}

// ─── Campaigns ──────────────────────────────────────────────────────────────

// GET /api/v1/campaigns
pub async fn get_campaigns(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "utm",
        &auth.allowed_modules,
    )
    .await?;

    let data = services::campaigns::get_campaign_stats(
        &state.db,
        auth.project_id,
        params.start_at,
        params.end_at,
    )
    .await?;

    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/campaigns/sources
pub async fn get_sources(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedDateQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "utm",
        &auth.allowed_modules,
    )
    .await?;

    let data = services::campaigns::get_sources(
        &state.db,
        auth.project_id,
        params.start_at,
        params.end_at,
        params.limit,
        params.offset,
    )
    .await?;

    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/campaigns/mediums
pub async fn get_mediums(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedDateQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "utm",
        &auth.allowed_modules,
    )
    .await?;

    let data = services::campaigns::get_mediums(
        &state.db,
        auth.project_id,
        params.start_at,
        params.end_at,
        params.limit,
        params.offset,
    )
    .await?;

    Ok(axum::Json(json!({ "data": data })))
}

// GET /api/v1/campaigns/timeseries
pub async fn get_campaign_timeseries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<CampaignTimeseriesQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "utm",
        &auth.allowed_modules,
    )
    .await?;

    let data = services::campaigns::get_campaign_timeseries(
        &state.db,
        auth.project_id,
        params.start_at,
        params.end_at,
        &params.utm_source,
    )
    .await?;

    Ok(axum::Json(json!({ "data": data })))
}
