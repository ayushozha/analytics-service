use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::services::insights::filters::DataNeeds;
use crate::services::insights::funnels::{FunnelActorsQuery, FunnelsQuery};
use crate::services::insights::paths::{PathsActorsQuery, PathsQuery};
use crate::services::insights::retention::{RetentionActorsQuery, RetentionQuery};
use crate::services::insights::trends::{TrendsActorsQuery, TrendsQuery};
use crate::state::SharedState;

/// Insights read raw events, which belong to the always-on `core` module.
const INSIGHTS_MODULE: &str = "core";
/// Person properties and profile data are governed by the `identity` module.
const IDENTITY_MODULE: &str = "identity";

/// Cohort membership is governed by the `cohorts` module.
const COHORTS_MODULE: &str = "cohorts";

/// Insights need `core`; reading person properties needs `identity`;
/// filtering by cohort needs `cohorts`.
pub(crate) async fn authorize(
    state: &SharedState,
    auth: &AuthenticatedProject,
    needs: DataNeeds,
) -> AppResult<()> {
    auth.require_scope("query")?;
    let mut modules = vec![INSIGHTS_MODULE];
    if needs.person {
        modules.push(IDENTITY_MODULE);
    }
    if needs.cohorts {
        modules.push(COHORTS_MODULE);
    }
    for module in modules {
        services::modules::require_module_read(
            state,
            auth.project_id,
            module,
            &auth.allowed_modules,
        )
        .await?;
    }
    Ok(())
}

/// Actor endpoints always return person data.
fn actor_needs(q: DataNeeds) -> DataNeeds {
    DataNeeds { person: true, ..q }
}

// POST /api/v1/insights/trends
pub async fn trends(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<TrendsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, query.data_needs()).await?;
    let result = services::insights::trends::run_trends(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/trends/actors
pub async fn trends_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<TrendsActorsQuery>,
) -> AppResult<impl IntoResponse> {
    // Actors always include user ids and profile traits.
    authorize(&state, &auth, actor_needs(query.query.data_needs())).await?;
    let result =
        services::insights::trends::run_trends_actors(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/funnels
pub async fn funnels(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<FunnelsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, query.data_needs()).await?;
    let result =
        services::insights::funnels::run_funnel(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/funnels/actors
pub async fn funnel_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<FunnelActorsQuery>,
) -> AppResult<impl IntoResponse> {
    // Actors always include user ids and profile traits.
    authorize(&state, &auth, actor_needs(query.query.data_needs())).await?;
    let result =
        services::insights::funnels::run_funnel_actors(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/retention
pub async fn retention(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<RetentionQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, query.data_needs()).await?;
    let result =
        services::insights::retention::run_retention(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/retention/actors
pub async fn retention_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<RetentionActorsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, actor_needs(query.query.data_needs())).await?;
    let result =
        services::insights::retention::run_retention_actors(&state.db, auth.project_id, &query)
            .await?;
    Ok(Json(result))
}

// POST /api/v1/insights/stickiness
pub async fn stickiness(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<TrendsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, query.data_needs()).await?;
    let result =
        services::insights::stickiness::run_stickiness(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/lifecycle
pub async fn lifecycle(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<TrendsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, query.data_needs()).await?;
    let result =
        services::insights::stickiness::run_lifecycle(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct StickinessActorsBody {
    pub query: TrendsQuery,
    #[serde(default)]
    pub series: usize,
    /// Number of active intervals (the clicked bar).
    pub intervals: i64,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Deserialize)]
pub struct LifecycleActorsBody {
    pub query: TrendsQuery,
    /// Bucket start as returned in `days`.
    pub day: String,
    /// new, returning, resurrecting or dormant.
    pub status: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

pub(crate) fn default_limit() -> usize {
    100
}

// POST /api/v1/insights/stickiness/actors
pub async fn stickiness_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(b): Json<StickinessActorsBody>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, actor_needs(b.query.data_needs())).await?;
    let result = services::insights::stickiness::run_stickiness_actors(
        &state.db,
        auth.project_id,
        &b.query,
        b.series,
        b.intervals,
        b.limit,
        b.offset,
    )
    .await?;
    Ok(Json(result))
}

// POST /api/v1/insights/lifecycle/actors
pub async fn lifecycle_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(b): Json<LifecycleActorsBody>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, actor_needs(b.query.data_needs())).await?;
    let result = services::insights::stickiness::run_lifecycle_actors(
        &state.db,
        auth.project_id,
        &b.query,
        &b.day,
        &b.status,
        b.limit,
        b.offset,
    )
    .await?;
    Ok(Json(result))
}

// POST /api/v1/insights/paths
pub async fn paths(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<PathsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, query.data_needs()).await?;
    let result = services::insights::paths::run_paths(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/paths/actors
pub async fn paths_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<PathsActorsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, actor_needs(query.query.data_needs())).await?;
    let result =
        services::insights::paths::run_paths_actors(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}
