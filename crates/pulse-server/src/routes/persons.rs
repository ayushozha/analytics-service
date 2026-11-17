use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::services::persons::{self, EventsQuery, PersonsQuery, PropertiesPatch};
use crate::state::SharedState;

/// People data is governed by the `identity` module.
const MODULE: &str = "identity";

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

// POST /api/v2/persons/query
pub async fn query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(q): Json<PersonsQuery>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    if q.uses_cohorts() {
        services::modules::require_module_read(
            &state,
            auth.project_id,
            "cohorts",
            &auth.allowed_modules,
        )
        .await?;
    }
    Ok(Json(persons::list(&state.db, auth.project_id, &q).await?))
}

// GET /api/v2/persons/{visitor_id}
pub async fn get(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    // Cohort memberships are cohorts-module data: include them only if readable.
    let include_cohorts = services::modules::require_module_read(
        &state,
        auth.project_id,
        "cohorts",
        &auth.allowed_modules,
    )
    .await
    .is_ok();
    Ok(Json(
        persons::get(&state.db, auth.project_id, &visitor_id, include_cohorts).await?,
    ))
}

// GET /api/v2/persons/{visitor_id}/events
pub async fn events(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
    Query(q): Query<EventsQuery>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        persons::events(&state.db, auth.project_id, &visitor_id, &q).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct Page {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

// GET /api/v2/persons/{visitor_id}/sessions
pub async fn sessions(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
    Query(p): Query<Page>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        persons::sessions(&state.db, auth.project_id, &visitor_id, p.limit, p.offset).await?,
    ))
}

// PATCH /api/v2/persons/{visitor_id}/properties
pub async fn update_properties(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
    Json(p): Json<PropertiesPatch>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        persons::update_properties(&state.db, auth.project_id, &visitor_id, &p).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct MergeBody {
    pub visitor_ids: Vec<String>,
}

// POST /api/v2/persons/{visitor_id}/merge
pub async fn merge(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
    Json(b): Json<MergeBody>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        persons::merge(&state, auth.project_id, &visitor_id, &b.visitor_ids).await?,
    ))
}

#[derive(Debug, Deserialize, Default)]
pub struct DeleteParams {
    #[serde(default)]
    pub delete_events: bool,
}

// DELETE /api/v2/persons/{visitor_id}
pub async fn delete(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
    Query(p): Query<DeleteParams>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        persons::delete(&state.db, auth.project_id, &visitor_id, p.delete_events).await?,
    ))
}
