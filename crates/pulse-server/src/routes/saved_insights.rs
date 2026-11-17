use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::routes::insights::authorize;
use crate::services;
use crate::services::annotations::{AnnotationInput, AnnotationListQuery, AnnotationPatch};
use crate::services::insights::saved::{self, InsightInput, InsightListQuery, InsightPatch};
use crate::state::SharedState;

/// Saved insights and annotations are dashboard objects.
const MODULE: &str = "dashboards";

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

// GET /api/v1/saved-insights
pub async fn list(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(f): Query<InsightListQuery>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        json!({ "data": saved::list(&state.db, auth.project_id, &f).await? }),
    ))
}

// POST /api/v1/saved-insights
pub async fn create(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(input): Json<InsightInput>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        saved::create(&state.db, auth.project_id, &input).await?,
    ))
}

// GET /api/v1/saved-insights/{id}
pub async fn get(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(saved::get(&state.db, auth.project_id, id).await?))
}

// PATCH /api/v1/saved-insights/{id}
pub async fn update(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(patch): Json<InsightPatch>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        saved::update(&state.db, auth.project_id, id, &patch).await?,
    ))
}

// DELETE /api/v1/saved-insights/{id}
pub async fn delete(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    saved::delete(&state.db, auth.project_id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

// POST /api/v1/saved-insights/{id}/duplicate
pub async fn duplicate(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        saved::duplicate(&state.db, auth.project_id, id).await?,
    ))
}

// GET /api/v1/saved-insights/{id}/versions
pub async fn versions(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        json!({ "data": saved::versions(&state.db, auth.project_id, id).await? }),
    ))
}

// POST /api/v1/saved-insights/{id}/versions/{version}/restore
pub async fn restore(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((id, version)): Path<(Uuid, i32)>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        saved::restore(&state.db, auth.project_id, id, version).await?,
    ))
}

#[derive(Debug, Deserialize, Default)]
pub struct RunParams {
    /// `true` skips the result cache.
    #[serde(default)]
    pub refresh: bool,
}

// GET /api/v1/saved-insights/{id}/run
pub async fn run(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Query(params): Query<RunParams>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    let insight = saved::get(&state.db, auth.project_id, id).await?;
    let parsed = saved::parse_saved(&insight)?;
    authorize(&state, &auth, parsed.reads_person_properties()).await?;
    Ok(Json(
        saved::run_saved(&state.db, auth.project_id, insight, &parsed, params.refresh).await?,
    ))
}

// GET /api/v1/annotations
pub async fn list_annotations(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(f): Query<AnnotationListQuery>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(json!({
        "data": services::annotations::list(&state.db, auth.project_id, &f).await?
    })))
}

// POST /api/v1/annotations
pub async fn create_annotation(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(input): Json<AnnotationInput>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        services::annotations::create(&state.db, auth.project_id, &input).await?,
    ))
}

// PATCH /api/v1/annotations/{id}
pub async fn update_annotation(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(patch): Json<AnnotationPatch>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        services::annotations::update(&state.db, auth.project_id, id, &patch).await?,
    ))
}

// DELETE /api/v1/annotations/{id}
pub async fn delete_annotation(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    services::annotations::delete(&state.db, auth.project_id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
