use axum::response::IntoResponse;
use axum::{Extension, Json};

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::services::insights::funnels::{FunnelActorsQuery, FunnelsQuery};
use crate::services::insights::trends::{TrendsActorsQuery, TrendsQuery};
use crate::state::SharedState;

/// Insights read raw events, which belong to the always-on `core` module.
const INSIGHTS_MODULE: &str = "core";
/// Person properties and profile data are governed by the `identity` module.
const IDENTITY_MODULE: &str = "identity";

async fn authorize(
    state: &SharedState,
    auth: &AuthenticatedProject,
    reads_person_data: bool,
) -> AppResult<()> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        state,
        auth.project_id,
        INSIGHTS_MODULE,
        &auth.allowed_modules,
    )
    .await?;
    if reads_person_data {
        services::modules::require_module_read(
            state,
            auth.project_id,
            IDENTITY_MODULE,
            &auth.allowed_modules,
        )
        .await?;
    }
    Ok(())
}

// POST /api/v1/insights/trends
pub async fn trends(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(query): Json<TrendsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, query.reads_person_properties()).await?;
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
    authorize(&state, &auth, true).await?;
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
    authorize(&state, &auth, query.reads_person_properties()).await?;
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
    authorize(&state, &auth, true).await?;
    let result =
        services::insights::funnels::run_funnel_actors(&state.db, auth.project_id, &query).await?;
    Ok(Json(result))
}
