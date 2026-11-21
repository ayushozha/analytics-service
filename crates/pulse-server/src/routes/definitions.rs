//! Event / property definitions and the events explorer.

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedProject;
use crate::routes::insights::authorize;
use crate::services::definitions::{
    self, DefinitionKind, DefinitionPatch, DefinitionsQuery, EventsQuery, PropertyDefinitionsQuery,
};
use crate::services::insights::filters::DataNeeds;
use crate::state::SharedState;

/// Person data (traits, person property definitions) is governed by `identity`.
async fn can_read_persons(state: &SharedState, auth: &AuthenticatedProject) -> bool {
    crate::services::modules::require_module_read(
        state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await
    .is_ok()
}

// GET /api/v2/event-definitions
pub async fn event_definitions(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(q): Query<DefinitionsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, DataNeeds::default()).await?;
    let data = definitions::list_event_definitions(&state.db, auth.project_id, &q).await?;
    Ok(Json(json!({ "data": data })))
}

// GET /api/v2/property-definitions
pub async fn property_definitions(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(q): Query<PropertyDefinitionsQuery>,
) -> AppResult<impl IntoResponse> {
    let person = q.kind == Some(DefinitionKind::PersonProperty);
    authorize(
        &state,
        &auth,
        DataNeeds {
            person,
            ..Default::default()
        },
    )
    .await?;
    let data = definitions::list_property_definitions(&state.db, auth.project_id, &q).await?;
    Ok(Json(json!({ "data": data })))
}

// PATCH /api/v2/definitions
pub async fn update_definition(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(patch): Json<DefinitionPatch>,
) -> AppResult<impl IntoResponse> {
    let person = patch.kind == DefinitionKind::PersonProperty;
    authorize(
        &state,
        &auth,
        DataNeeds {
            person,
            ..Default::default()
        },
    )
    .await?;
    let meta = definitions::upsert_definition(&state.db, auth.project_id, &patch).await?;
    Ok(Json(meta))
}

// POST /api/v2/events/query
pub async fn query_events(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(q): Json<EventsQuery>,
) -> AppResult<impl IntoResponse> {
    authorize(&state, &auth, q.joins().into()).await?;
    let include_persons = can_read_persons(&state, &auth).await;
    Ok(Json(
        definitions::query_events(&state.db, auth.project_id, &q, include_persons).await?,
    ))
}
