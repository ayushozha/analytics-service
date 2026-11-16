use axum::extract::{Path, Query};
use axum::http::header;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::routes::insights::{authorize, LifecycleActorsBody, StickinessActorsBody};
use crate::services;
use crate::services::insights::filters::DataNeeds;
use crate::services::insights::{funnels, paths, retention, stickiness, trends};
use crate::services::person_cohorts::{self, CohortInput, CohortPatch, MAX_STATIC_MEMBERS};
use crate::state::SharedState;

const MODULE: &str = "cohorts";
const IDENTITY_MODULE: &str = "identity";
/// Page size used when exporting members as CSV.
const CSV_PAGE: i64 = 10_000;
/// Page size used when collecting actors into a static cohort.
const ACTOR_PAGE: usize = 1_000;

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

/// Member lists expose who is in a cohort: identity access required.
async fn can_read_people(state: &SharedState, auth: &AuthenticatedProject) -> AppResult<()> {
    can_read(state, auth).await?;
    services::modules::require_module_read(
        state,
        auth.project_id,
        IDENTITY_MODULE,
        &auth.allowed_modules,
    )
    .await
}

// GET /api/v2/cohorts
pub async fn list(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        json!({ "data": person_cohorts::list(&state.db, auth.project_id).await? }),
    ))
}

// POST /api/v2/cohorts
pub async fn create(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(input): Json<CohortInput>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        person_cohorts::create(&state.db, auth.project_id, &input).await?,
    ))
}

// GET /api/v2/cohorts/{id}
pub async fn get(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        person_cohorts::get(&state.db, auth.project_id, id).await?,
    ))
}

// PATCH /api/v2/cohorts/{id}
pub async fn update(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(p): Json<CohortPatch>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        person_cohorts::update(&state.db, auth.project_id, id, &p).await?,
    ))
}

// DELETE /api/v2/cohorts/{id}
pub async fn delete(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    person_cohorts::delete(&state.db, auth.project_id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

// POST /api/v2/cohorts/{id}/calculate
pub async fn calculate(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        person_cohorts::calculate(&state.db, auth.project_id, id).await?,
    ))
}

// GET /api/v2/cohorts/{id}/history
pub async fn history(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_read(&state, &auth).await?;
    Ok(Json(
        json!({ "data": person_cohorts::history(&state.db, auth.project_id, id).await? }),
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

// GET /api/v2/cohorts/{id}/people
pub async fn people(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Query(p): Query<Page>,
) -> AppResult<impl IntoResponse> {
    can_read_people(&state, &auth).await?;
    Ok(Json(
        person_cohorts::members(&state.db, auth.project_id, id, p.limit, p.offset).await?,
    ))
}

fn csv_field(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

// GET /api/v2/cohorts/{id}/people.csv
pub async fn people_csv(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_read_people(&state, &auth).await?;
    let mut csv = String::from("visitor_id,user_id,traits\n");
    let mut offset = 0;
    loop {
        let page =
            person_cohorts::members(&state.db, auth.project_id, id, CSV_PAGE, offset).await?;
        let rows = page["data"].as_array().cloned().unwrap_or_default();
        for m in &rows {
            csv.push_str(&format!(
                "{},{},{}\n",
                csv_field(m["visitor_id"].as_str().unwrap_or_default()),
                csv_field(m["user_id"].as_str().unwrap_or_default()),
                csv_field(&m["traits"].to_string()),
            ));
        }
        if (rows.len() as i64) < CSV_PAGE {
            break;
        }
        offset += CSV_PAGE;
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"cohort-{id}.csv\""),
            ),
        ],
        csv,
    ))
}

#[derive(Debug, Deserialize)]
pub struct AddPeople {
    /// Visitor ids or identified user ids.
    pub ids: Vec<String>,
}

// POST /api/v2/cohorts/{id}/people
pub async fn add_people(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(b): Json<AddPeople>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    let (added, unknown) =
        person_cohorts::add_members(&state.db, auth.project_id, id, &b.ids).await?;
    Ok(Json(json!({ "added": added, "unknown": unknown })))
}

// POST /api/v2/cohorts/{id}/people/csv   (body: CSV, first column = id)
pub async fn add_people_csv(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    body: String,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    let ids = person_cohorts::parse_csv_ids(&body);
    if ids.is_empty() {
        return Err(AppError::BadRequest("the CSV has no ids".into()));
    }
    let (added, unknown) =
        person_cohorts::add_members(&state.db, auth.project_id, id, &ids).await?;
    Ok(Json(
        json!({ "rows": ids.len(), "added": added, "unknown": unknown }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct RemovePeople {
    pub visitor_ids: Vec<String>,
}

// DELETE /api/v2/cohorts/{id}/people
pub async fn remove_people(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
    Json(b): Json<RemovePeople>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    let removed =
        person_cohorts::remove_members(&state.db, auth.project_id, id, &b.visitor_ids).await?;
    Ok(Json(json!({ "removed": removed })))
}

// POST /api/v2/cohorts/{id}/duplicate-static
pub async fn duplicate_static(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    Ok(Json(
        person_cohorts::duplicate_as_static(&state.db, auth.project_id, id).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct FromActors {
    pub name: String,
    /// trends, funnels, retention, stickiness, lifecycle or paths
    pub kind: String,
    /// The body of that insight's `/actors` endpoint (limit/offset ignored).
    pub actors_query: Value,
}

/// Run one page of an insight's actors query.
async fn actors_page(
    state: &SharedState,
    project_id: Uuid,
    kind: &str,
    q: &Value,
    offset: usize,
) -> AppResult<Value> {
    let mut q = q.clone();
    q["limit"] = json!(ACTOR_PAGE);
    q["offset"] = json!(offset);
    let de = |e: serde_json::Error| AppError::BadRequest(format!("invalid actors_query: {e}"));
    let db = &state.db;
    match kind {
        "trends" => {
            trends::run_trends_actors(db, project_id, &serde_json::from_value(q).map_err(de)?).await
        }
        "funnels" => {
            funnels::run_funnel_actors(db, project_id, &serde_json::from_value(q).map_err(de)?)
                .await
        }
        "retention" => {
            retention::run_retention_actors(db, project_id, &serde_json::from_value(q).map_err(de)?)
                .await
        }
        "paths" => {
            paths::run_paths_actors(db, project_id, &serde_json::from_value(q).map_err(de)?).await
        }
        "stickiness" => {
            let b: StickinessActorsBody = serde_json::from_value(q).map_err(de)?;
            stickiness::run_stickiness_actors(
                db,
                project_id,
                &b.query,
                b.series,
                b.intervals,
                b.limit,
                b.offset,
            )
            .await
        }
        "lifecycle" => {
            let b: LifecycleActorsBody = serde_json::from_value(q).map_err(de)?;
            stickiness::run_lifecycle_actors(
                db, project_id, &b.query, &b.day, &b.status, b.limit, b.offset,
            )
            .await
        }
        other => Err(AppError::BadRequest(format!(
            "unknown insight kind '{other}'"
        ))),
    }
}

// POST /api/v2/cohorts/from-actors
/// Save the visitors behind any insight data point as a static cohort.
pub async fn from_actors(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Json(b): Json<FromActors>,
) -> AppResult<impl IntoResponse> {
    can_write(&state, &auth).await?;
    // Actor queries read person data, exactly like the /actors endpoints.
    authorize(
        &state,
        &auth,
        DataNeeds {
            person: true,
            cohorts: true,
        },
    )
    .await?;
    let mut ids: Vec<String> = Vec::new();
    loop {
        let page =
            actors_page(&state, auth.project_id, &b.kind, &b.actors_query, ids.len()).await?;
        let batch: Vec<String> = page["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| a["visitor_id"].as_str().map(String::from))
            .collect();
        let n = batch.len();
        ids.extend(batch);
        if n < ACTOR_PAGE {
            break;
        }
        if ids.len() >= MAX_STATIC_MEMBERS {
            return Err(AppError::BadRequest(format!(
                "more than {MAX_STATIC_MEMBERS} people; narrow the query"
            )));
        }
    }
    let cohort = person_cohorts::create(
        &state.db,
        auth.project_id,
        &CohortInput {
            name: b.name,
            description: None,
            is_static: true,
            filters: None,
        },
    )
    .await?;
    person_cohorts::add_members(&state.db, auth.project_id, cohort.id, &ids).await?;
    Ok(Json(
        person_cohorts::get(&state.db, auth.project_id, cohort.id).await?,
    ))
}
