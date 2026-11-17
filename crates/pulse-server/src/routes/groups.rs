use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::services::groups::{self, GroupsQuery, Page, TypeNames};
use crate::services::identity::PersonUpdate;
use crate::services::persons::PropertiesPatch;
use crate::state::SharedState;

/// Group data is governed like person data, by the `identity` module.
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

// GET /api/v2/groups/types
pub async fn list_types(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(groups::list_types(&state.db, auth.project_id).await?))
}

// PATCH /api/v2/groups/types/{index}
pub async fn rename_type(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(index): Path<i16>,
    Json(b): Json<TypeNames>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        groups::rename_type(&state.db, auth.project_id, index, &b).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct IdentifyBody {
    pub group_type: String,
    pub group_key: String,
    #[serde(default)]
    pub properties: Value,
}

// POST /api/v2/groups — server-side `$groupidentify`.
pub async fn identify(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(b): Json<IdentifyBody>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    let index = groups::type_index(&state.db, auth.project_id, &b.group_type, true)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "this project already has {} group types",
                groups::MAX_GROUP_TYPES
            ))
        })?;
    let update = PersonUpdate {
        set: b.properties,
        ..Default::default()
    };
    Ok(Json(
        groups::upsert(&state.db, auth.project_id, index, &b.group_key, &update).await?,
    ))
}

// POST /api/v2/groups/{index}/query
pub async fn query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(index): Path<i16>,
    Json(q): Json<GroupsQuery>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        groups::list(&state.db, auth.project_id, index, &q).await?,
    ))
}

// GET /api/v2/groups/{index}/{key}
pub async fn get(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((index, key)): Path<(i16, String)>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        groups::get(&state.db, auth.project_id, index, &key).await?,
    ))
}

// GET /api/v2/groups/{index}/{key}/events
pub async fn events(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((index, key)): Path<(i16, String)>,
    Query(p): Query<Page>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        groups::events(&state.db, auth.project_id, index, &key, &p).await?,
    ))
}

// GET /api/v2/groups/{index}/{key}/persons
pub async fn persons(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((index, key)): Path<(i16, String)>,
    Query(p): Query<Page>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        groups::persons(&state.db, auth.project_id, index, &key, &p).await?,
    ))
}

// PATCH /api/v2/groups/{index}/{key}/properties
pub async fn update_properties(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((index, key)): Path<(i16, String)>,
    Json(p): Json<PropertiesPatch>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    let update = PersonUpdate {
        set: p.set.unwrap_or_else(|| json!({})),
        set_once: p.set_once.unwrap_or_else(|| json!({})),
        unset: p.unset,
    };
    Ok(Json(
        groups::upsert(&state.db, auth.project_id, index, &key, &update).await?,
    ))
}

// DELETE /api/v2/groups/{index}/{key}
pub async fn delete(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path((index, key)): Path<(i16, String)>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    groups::delete(&state.db, auth.project_id, index, &key).await?;
    Ok(Json(json!({ "deleted": true })))
}
