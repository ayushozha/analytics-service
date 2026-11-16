use axum::response::IntoResponse;
use axum::{Extension, Json};

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::services::insights::trends::{TrendsActorsQuery, TrendsQuery};
use crate::state::SharedState;

/// Insights read raw events, which belong to the always-on `core` module.
const INSIGHTS_MODULE: &str = "core";

async fn authorize(state: &SharedState, auth: &AuthenticatedProject) -> AppResult<()> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        state,
        auth.project_id,
        INSIGHTS_MODULE,
        &auth.allowed_modules,
    )
    .await
}

// POST /api/v1/insights/trends
pub async fn trends(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<TrendsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth).await?;
    let result = services::insights::trends::run_trends(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}

// POST /api/v1/insights/trends/actors
pub async fn trends_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<TrendsActorsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth).await?;
    let result =
        services::insights::trends::run_trends_actors(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}
