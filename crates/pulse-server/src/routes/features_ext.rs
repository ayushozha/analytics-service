use axum::extract::{Path, Query};
use axum::http::{header, HeaderMap};
use axum::response::IntoResponse;
use axum::Extension;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::services;
use crate::state::SharedState;

#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

impl DateRangeQuery {
    fn resolve(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let end = self.end_at.unwrap_or_else(Utc::now);
        let start = self.start_at.unwrap_or_else(|| end - Duration::days(30));
        (start, end)
    }
}

// ============================================================================
// AI ANALYTICS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AiQueryInput {
    pub question: String,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn ask_ai_query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<AiQueryInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let end = input.end_at.unwrap_or_else(Utc::now);
    let start = input.start_at.unwrap_or_else(|| end - Duration::days(30));
    let response = services::ai::answer_query(
        &state.db,
        auth.project_id,
        &input.question,
        start,
        end,
        input.limit.unwrap_or(10),
    )
    .await?;
    Ok(axum::Json(response))
}

pub async fn get_ai_insights(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let (start, end) = params.resolve();
    let insights = services::ai::generate_insights(&state.db, auth.project_id, start, end).await?;
    Ok(axum::Json(serde_json::json!({ "data": insights })))
}

#[derive(Debug, Deserialize)]
pub struct AiHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_ai_query_history(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<AiHistoryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let runs = services::ai::list_query_runs(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": runs })))
}

pub async fn list_llm_traces(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<AiHistoryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let traces = services::ai::list_llm_traces(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": traces })))
}

pub async fn record_llm_trace(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::ai::LlmTraceInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let trace = services::ai::record_llm_trace(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(trace)))
}

pub async fn get_llm_trace(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(trace_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let trace = services::ai::get_llm_trace(&state.db, auth.project_id, trace_id).await?;
    Ok(axum::Json(trace))
}

pub async fn list_llm_generations(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<AiHistoryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let generations = services::ai::list_llm_generations(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": generations })))
}

pub async fn record_llm_generation(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::ai::LlmGenerationInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let generation = services::ai::record_llm_generation(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(generation)))
}

pub async fn list_llm_evaluations(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<AiHistoryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let evaluations = services::ai::list_llm_evaluations(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": evaluations })))
}

pub async fn record_llm_evaluation(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::ai::LlmEvaluationInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let evaluation = services::ai::record_llm_evaluation(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(evaluation)))
}

pub async fn get_llm_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ai_queries",
        &auth.allowed_modules,
    )
    .await?;

    let (start, end) = params.resolve();
    let stats = services::ai::get_llm_stats(&state.db, auth.project_id, start, end).await?;
    Ok(axum::Json(stats))
}

// ============================================================================
// BI LAYER
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BiRunQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BiHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_bi_metrics(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let metrics = services::bi::list_metrics(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": metrics })))
}

pub async fn get_bi_metric(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(metric_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let metric = services::bi::get_metric(&state.db, auth.project_id, metric_id).await?;
    Ok(axum::Json(metric))
}

pub async fn create_bi_metric(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::SemanticMetricInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let metric = services::bi::create_metric(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(metric)))
}

pub async fn update_bi_metric(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(metric_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::bi::SemanticMetricInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let metric = services::bi::update_metric(&state.db, auth.project_id, metric_id, input).await?;
    Ok(axum::Json(metric))
}

pub async fn delete_bi_metric(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(metric_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    services::bi::delete_metric(&state.db, auth.project_id, metric_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_bi_row_policies(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let policies = services::bi::list_row_policies(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": policies })))
}

pub async fn create_bi_row_policy(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::BiRowPolicyInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let policy = services::bi::create_row_policy(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(policy)))
}

pub async fn update_bi_row_policy(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(policy_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::bi::BiRowPolicyInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let policy =
        services::bi::update_row_policy(&state.db, auth.project_id, policy_id, input).await?;
    Ok(axum::Json(policy))
}

pub async fn delete_bi_row_policy(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(policy_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    services::bi::delete_row_policy(&state.db, auth.project_id, policy_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_bi_database_connections(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let connections = services::bi::list_database_connections(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": connections })))
}

pub async fn get_bi_database_connection(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(connection_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let connection =
        services::bi::get_database_connection(&state.db, auth.project_id, connection_id).await?;
    Ok(axum::Json(connection))
}

pub async fn create_bi_database_connection(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::BiDatabaseConnectionInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let connection =
        services::bi::create_database_connection(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(connection)))
}

pub async fn update_bi_database_connection(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(connection_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::bi::BiDatabaseConnectionInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let connection =
        services::bi::update_database_connection(&state.db, auth.project_id, connection_id, input)
            .await?;
    Ok(axum::Json(connection))
}

pub async fn delete_bi_database_connection(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(connection_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    services::bi::delete_database_connection(&state.db, auth.project_id, connection_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn test_bi_database_connection(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(connection_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let response =
        services::bi::test_database_connection(&state.db, auth.project_id, connection_id).await?;
    Ok(axum::Json(response))
}

pub async fn run_bi_external_sql(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(connection_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::bi::ExternalSqlRunRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let response =
        services::bi::run_external_sql(&state.db, auth.project_id, connection_id, input).await?;
    Ok(axum::Json(response))
}

pub async fn list_bi_embeds(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let embeds = services::bi::list_embeds(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": embeds })))
}

pub async fn get_bi_embed(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(embed_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let embed = services::bi::get_embed(&state.db, auth.project_id, embed_id).await?;
    Ok(axum::Json(embed))
}

pub async fn create_bi_embed(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::BiEmbedInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let embed = services::bi::create_embed(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(embed)))
}

pub async fn update_bi_embed(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(embed_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::bi::BiEmbedInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let embed = services::bi::update_embed(&state.db, auth.project_id, embed_id, input).await?;
    Ok(axum::Json(embed))
}

pub async fn delete_bi_embed(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(embed_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    services::bi::delete_embed(&state.db, auth.project_id, embed_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn rotate_bi_embed_token(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(embed_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let embed = services::bi::rotate_embed_token(&state.db, auth.project_id, embed_id).await?;
    Ok(axum::Json(embed))
}

pub async fn resolve_bi_embed(
    Extension(state): Extension<SharedState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    let origin = embed_request_origin(&headers);
    let embed = services::bi::resolve_embed(&state.db, &token, origin.as_deref()).await?;
    Ok(axum::Json(embed))
}

fn embed_request_origin(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get(header::REFERER)
                .and_then(|value| value.to_str().ok())
                .and_then(|referer| url::Url::parse(referer).ok())
                .map(|url| {
                    let port = url
                        .port()
                        .map(|port| format!(":{port}"))
                        .unwrap_or_default();
                    format!(
                        "{}://{}{}",
                        url.scheme(),
                        url.host_str().unwrap_or_default(),
                        port
                    )
                })
        })
}

pub async fn run_bi_sql(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::SqlRunRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let response = services::bi::run_ad_hoc_sql(&state.db, auth.project_id, input).await?;
    Ok(axum::Json(response))
}

pub async fn list_bi_saved_queries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let queries = services::bi::list_saved_queries(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": queries })))
}

pub async fn get_bi_saved_query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(query_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let query = services::bi::get_saved_query(&state.db, auth.project_id, query_id).await?;
    Ok(axum::Json(query))
}

pub async fn create_bi_saved_query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::SavedSqlInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let query = services::bi::create_saved_query(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(query)))
}

pub async fn update_bi_saved_query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(query_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::bi::SavedSqlInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let query =
        services::bi::update_saved_query(&state.db, auth.project_id, query_id, input).await?;
    Ok(axum::Json(query))
}

pub async fn delete_bi_saved_query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(query_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    services::bi::delete_saved_query(&state.db, auth.project_id, query_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn run_bi_saved_query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(query_id): Path<Uuid>,
    Query(params): Query<BiRunQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let response =
        services::bi::run_saved_query(&state.db, auth.project_id, query_id, params.limit).await?;
    Ok(axum::Json(response))
}

pub async fn run_bi_visual_query(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::VisualQueryRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let response = services::bi::run_visual_query(&state.db, auth.project_id, input).await?;
    Ok(axum::Json(response))
}

pub async fn run_bi_drill_through(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::DrillThroughRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let response = services::bi::run_drill_through(&state.db, auth.project_id, input).await?;
    Ok(axum::Json(response))
}

pub async fn list_bi_query_runs(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<BiHistoryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let runs = services::bi::list_query_runs(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": runs })))
}

pub async fn list_csv_uploads(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let uploads = services::bi::list_csv_uploads(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": uploads })))
}

pub async fn create_csv_upload(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::bi::CsvUploadInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let upload = services::bi::create_csv_upload(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(upload)))
}

pub async fn get_csv_upload_rows(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(upload_id): Path<Uuid>,
    Query(params): Query<BiHistoryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    let rows = services::bi::get_csv_upload_rows(
        &state.db,
        auth.project_id,
        upload_id,
        params.limit.unwrap_or(100),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": rows })))
}

pub async fn delete_csv_upload(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(upload_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "bi", &auth.allowed_modules)
        .await?;
    services::bi::delete_csv_upload(&state.db, auth.project_id, upload_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================================================
// PRODUCT ANALYTICS WORKSPACE
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DashboardInput {
    pub name: String,
    pub description: Option<String>,
    pub layout: Option<serde_json::Value>,
    pub widgets: Option<serde_json::Value>,
    pub is_default: Option<bool>,
}

pub async fn list_custom_dashboards(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let dashboards =
        services::product_analytics::list_dashboards(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": dashboards })))
}

pub async fn get_custom_dashboard(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(dashboard_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let dashboard =
        services::product_analytics::get_dashboard(&state.db, auth.project_id, dashboard_id)
            .await?;
    Ok(axum::Json(dashboard))
}

pub async fn create_custom_dashboard(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<DashboardInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let dashboard = services::product_analytics::create_dashboard(
        &state.db,
        auth.project_id,
        &input.name,
        input.description.as_deref(),
        input.layout.unwrap_or_else(|| serde_json::json!({})),
        input.widgets.unwrap_or_else(|| serde_json::json!([])),
        input.is_default.unwrap_or(false),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(dashboard)))
}

pub async fn update_custom_dashboard(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(dashboard_id): Path<Uuid>,
    axum::Json(input): axum::Json<DashboardInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let dashboard = services::product_analytics::update_dashboard(
        &state.db,
        auth.project_id,
        dashboard_id,
        &input.name,
        input.description.as_deref(),
        input.layout.unwrap_or_else(|| serde_json::json!({})),
        input.widgets.unwrap_or_else(|| serde_json::json!([])),
        input.is_default.unwrap_or(false),
    )
    .await?;
    Ok(axum::Json(dashboard))
}

pub async fn delete_custom_dashboard(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(dashboard_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    services::product_analytics::delete_dashboard(&state.db, auth.project_id, dashboard_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SavedReportInput {
    pub name: String,
    pub description: Option<String>,
    pub report_type: String,
    pub params: Option<serde_json::Value>,
    pub visualization: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn list_saved_reports(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let reports = services::product_analytics::list_reports(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": reports })))
}

pub async fn get_saved_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let report =
        services::product_analytics::get_report(&state.db, auth.project_id, report_id).await?;
    Ok(axum::Json(report))
}

pub async fn create_saved_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<SavedReportInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let report = services::product_analytics::create_report(
        &state.db,
        auth.project_id,
        &input.name,
        input.description.as_deref(),
        &input.report_type,
        input.params.unwrap_or_else(|| serde_json::json!({})),
        input.visualization.as_deref().unwrap_or("table"),
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(report)))
}

pub async fn update_saved_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_id): Path<Uuid>,
    axum::Json(input): axum::Json<SavedReportInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let report = services::product_analytics::update_report(
        &state.db,
        auth.project_id,
        report_id,
        &input.name,
        input.description.as_deref(),
        &input.report_type,
        input.params.unwrap_or_else(|| serde_json::json!({})),
        input.visualization.as_deref().unwrap_or("table"),
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok(axum::Json(report))
}

pub async fn delete_saved_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    services::product_analytics::delete_report(&state.db, auth.project_id, report_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SavedReportRunQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

pub async fn run_saved_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_id): Path<Uuid>,
    Query(params): Query<SavedReportRunQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let result = services::product_analytics::run_saved_report(
        &state.db,
        auth.project_id,
        report_id,
        params.start_at,
        params.end_at,
    )
    .await?;
    Ok(axum::Json(result))
}

pub async fn run_query_explorer(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::product_analytics::ExplorerRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let result =
        services::product_analytics::run_explorer(&state.db, auth.project_id, input).await?;
    Ok(axum::Json(result))
}

#[derive(Debug, Deserialize)]
pub struct ExplorerHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_query_explorer_runs(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<ExplorerHistoryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let runs = services::product_analytics::list_explorer_runs(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": runs })))
}

// ============================================================================
// PRODUCT INSIGHTS
// ============================================================================

pub async fn get_product_stickiness(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let (start_at, end_at) = params.resolve();
    let report =
        services::product_insights::get_stickiness(&state.db, auth.project_id, start_at, end_at)
            .await?;
    Ok(axum::Json(report))
}

pub async fn get_product_lifecycle(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let (start_at, end_at) = params.resolve();
    let report =
        services::product_insights::get_lifecycle(&state.db, auth.project_id, start_at, end_at)
            .await?;
    Ok(axum::Json(report))
}

pub async fn get_product_activation(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::product_insights::ActivationRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let report =
        services::product_insights::get_activation(&state.db, auth.project_id, input).await?;
    Ok(axum::Json(report))
}

pub async fn get_product_impact(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::product_insights::ImpactAnalysisRequest>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "dashboards",
        &auth.allowed_modules,
    )
    .await?;

    let report =
        services::product_insights::get_impact_analysis(&state.db, auth.project_id, input).await?;
    Ok(axum::Json(report))
}

// ============================================================================
// MARKETING ANALYTICS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct MarketingAttributionQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MarketingImportsQuery {
    pub provider: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MarketingImportSummaryQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub provider: Option<String>,
}

pub async fn get_marketing_channels(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let (start_at, end_at) = params.resolve();
    let data =
        services::marketing::get_channel_groups(&state.db, auth.project_id, start_at, end_at)
            .await?;
    Ok(axum::Json(serde_json::json!({ "data": data })))
}

pub async fn get_marketing_attribution(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<MarketingAttributionQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let date_range = DateRangeQuery {
        start_at: params.start_at,
        end_at: params.end_at,
    };
    let (start_at, end_at) = date_range.resolve();
    let data = services::marketing::get_attribution(
        &state.db,
        auth.project_id,
        start_at,
        end_at,
        params.model.as_deref().unwrap_or("last_touch"),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": data })))
}

pub async fn get_marketing_ecommerce(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let (start_at, end_at) = params.resolve();
    let report =
        services::marketing::get_ecommerce_report(&state.db, auth.project_id, start_at, end_at)
            .await?;
    Ok(axum::Json(report))
}

pub async fn get_marketing_ai_referrers(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let (start_at, end_at) = params.resolve();
    let data =
        services::marketing::get_ai_referrers(&state.db, auth.project_id, start_at, end_at).await?;
    Ok(axum::Json(serde_json::json!({ "data": data })))
}

pub async fn list_marketing_imports(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<MarketingImportsQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let imports = services::marketing::list_imports(
        &state.db,
        auth.project_id,
        params.provider.as_deref(),
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": imports })))
}

pub async fn create_marketing_import(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::marketing::MarketingImportInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let import = services::marketing::create_import(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(import)))
}

pub async fn get_marketing_import_rows(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(import_id): Path<Uuid>,
    Query(params): Query<MarketingImportsQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let rows = services::marketing::get_import_rows(
        &state.db,
        auth.project_id,
        import_id,
        params.limit.unwrap_or(100),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": rows })))
}

pub async fn delete_marketing_import(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(import_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    services::marketing::delete_import(&state.db, auth.project_id, import_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn get_marketing_import_summary(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<MarketingImportSummaryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "utm", &auth.allowed_modules)
        .await?;

    let date_range = DateRangeQuery {
        start_at: params.start_at,
        end_at: params.end_at,
    };
    let (start_at, end_at) = date_range.resolve();
    let summary = services::marketing::get_import_summary(
        &state.db,
        auth.project_id,
        start_at,
        end_at,
        params.provider.as_deref(),
    )
    .await?;
    Ok(axum::Json(summary))
}

pub async fn list_integrations(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<services::integrations::IntegrationFilter>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "integrations",
        &auth.allowed_modules,
    )
    .await?;

    let integrations = services::integrations::list_integrations(params);
    Ok(axum::Json(serde_json::json!({ "data": integrations })))
}

pub async fn get_integration(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(key): Path<String>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "integrations",
        &auth.allowed_modules,
    )
    .await?;

    let integration = services::integrations::get_integration(&key)?;
    Ok(axum::Json(integration))
}

// ============================================================================
// SOURCES / CDP INGESTION
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SourceIngestionQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_event_sources(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "sources",
        &auth.allowed_modules,
    )
    .await?;

    let sources = services::sources::list_sources(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": sources })))
}

pub async fn get_event_source(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(source_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "sources",
        &auth.allowed_modules,
    )
    .await?;

    let source = services::sources::get_source(&state.db, auth.project_id, source_id).await?;
    Ok(axum::Json(source))
}

pub async fn create_event_source(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::sources::SourceInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "sources",
        &auth.allowed_modules,
    )
    .await?;

    let source = services::sources::create_source(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(source)))
}

pub async fn update_event_source(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(source_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::sources::SourceInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "sources",
        &auth.allowed_modules,
    )
    .await?;

    let source =
        services::sources::update_source(&state.db, auth.project_id, source_id, input).await?;
    Ok(axum::Json(source))
}

pub async fn delete_event_source(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(source_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "sources",
        &auth.allowed_modules,
    )
    .await?;

    services::sources::delete_source(&state.db, auth.project_id, source_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_source_ingestions(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(source_id): Path<Uuid>,
    Query(params): Query<SourceIngestionQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "sources",
        &auth.allowed_modules,
    )
    .await?;

    let ingestions = services::sources::list_ingestions(
        &state.db,
        auth.project_id,
        source_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": ingestions })))
}

pub async fn ingest_source_webhook(
    Extension(state): Extension<SharedState>,
    Path(source_id): Path<Uuid>,
    headers: HeaderMap,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> AppResult<impl IntoResponse> {
    let token = source_token_from_headers(&headers).ok_or(AppError::Unauthorized)?;
    let headers_json = source_headers_json(&headers);
    let response =
        services::sources::ingest_source_payload(&state, source_id, &token, payload, headers_json)
            .await?;
    Ok(axum::Json(response))
}

fn source_token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-pulse-source-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}

fn source_headers_json(headers: &HeaderMap) -> serde_json::Value {
    let mut data = serde_json::Map::new();
    for (name, value) in headers {
        let name = name.as_str().to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "authorization" | "cookie" | "set-cookie" | "x-pulse-key" | "x-pulse-source-token"
        ) {
            continue;
        }

        if let Ok(value) = value.to_str() {
            data.insert(
                name,
                serde_json::Value::String(value.chars().take(512).collect()),
            );
        }
    }
    serde_json::Value::Object(data)
}

// ============================================================================
// DESTINATIONS / EVENT ROUTING
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DestinationDeliveryQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_destinations(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    let destinations =
        services::destinations::list_destinations(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": destinations })))
}

pub async fn get_destination(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(destination_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    let destination =
        services::destinations::get_destination(&state.db, auth.project_id, destination_id).await?;
    Ok(axum::Json(destination))
}

pub async fn create_destination(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::destinations::DestinationInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    let destination =
        services::destinations::create_destination(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(destination)))
}

pub async fn update_destination(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(destination_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::destinations::DestinationInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    let destination = services::destinations::update_destination(
        &state.db,
        auth.project_id,
        destination_id,
        input,
    )
    .await?;
    Ok(axum::Json(destination))
}

pub async fn delete_destination(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(destination_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    services::destinations::delete_destination(&state.db, auth.project_id, destination_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_destination_deliveries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DestinationDeliveryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    let deliveries = services::destinations::list_deliveries(
        &state.db,
        auth.project_id,
        params.status.as_deref(),
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": deliveries })))
}

pub async fn retry_destination_delivery(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(delivery_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    services::destinations::retry_delivery(&state.db, auth.project_id, delivery_id).await?;
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}

pub async fn get_destination_health(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "destinations",
        &auth.allowed_modules,
    )
    .await?;

    let health = services::destinations::destination_health(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": health })))
}

// ============================================================================
// CSV EXPORTS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

pub async fn export_csv(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_type): Path<String>,
    Query(params): Query<ExportQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "exports",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));

    let csv = match report_type.as_str() {
        "stats" => {
            services::exports::export_stats_csv(&state.db, auth.project_id, start, end).await
        }
        "pages" => {
            services::exports::export_pages_csv(&state.db, auth.project_id, start, end).await
        }
        "referrers" => {
            services::exports::export_referrers_csv(&state.db, auth.project_id, start, end).await
        }
        "events" => {
            services::exports::export_events_csv(&state.db, auth.project_id, start, end).await
        }
        "devices" => {
            services::exports::export_devices_csv(&state.db, auth.project_id, start, end).await
        }
        "geo" => services::exports::export_geo_csv(&state.db, auth.project_id, start, end).await,
        "campaigns" => {
            services::exports::export_campaigns_csv(&state.db, auth.project_id, start, end).await
        }
        _ => {
            return Err(AppError::BadRequest(format!(
                "Unknown report type: {report_type}"
            )))
        }
    }
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let filename = format!("pulse_{report_type}_{}.csv", start.format("%Y%m%d"));
    let disposition = format!("attachment; filename=\"{filename}\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        csv,
    ))
}

// ============================================================================
// SHARED DASHBOARDS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateSharedDashboard {
    pub name: Option<String>,
    pub modules: Option<Vec<String>>,
    pub password: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn create_shared_dashboard(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateSharedDashboard>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "sharing",
        &auth.allowed_modules,
    )
    .await?;

    let name = input.name.unwrap_or_else(|| "Shared Dashboard".to_string());
    let modules = input.modules.unwrap_or_default();
    let dashboard = services::sharing::create_shared_dashboard(
        &state.db,
        auth.project_id,
        &name,
        &modules,
        input.password.as_deref(),
        input.expires_at,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(dashboard)))
}

pub async fn list_shared_dashboards(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "sharing",
        &auth.allowed_modules,
    )
    .await?;

    let dashboards = services::sharing::list_shared_dashboards(&state.db, auth.project_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": dashboards })))
}

pub async fn delete_shared_dashboard(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "sharing",
        &auth.allowed_modules,
    )
    .await?;

    services::sharing::delete_shared_dashboard(&state.db, auth.project_id, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================================================
// EMAIL REPORTS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EmailReportInput {
    pub name: String,
    pub recipients: Vec<String>,
    pub schedule: String,
    pub modules: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

pub async fn list_email_reports(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "email_reports",
        &auth.allowed_modules,
    )
    .await?;

    let configs = services::email_reports::list_configs(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({
        "data": configs,
        "delivery_configured": state.config.email_report_webhook_url.is_some(),
    })))
}

pub async fn create_email_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<EmailReportInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "email_reports",
        &auth.allowed_modules,
    )
    .await?;

    let modules = input.modules.unwrap_or_default();
    let config = services::email_reports::create_config(
        &state.db,
        auth.project_id,
        &input.name,
        &input.recipients,
        &input.schedule,
        &modules,
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(config)))
}

pub async fn update_email_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_id): Path<Uuid>,
    axum::Json(input): axum::Json<EmailReportInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "email_reports",
        &auth.allowed_modules,
    )
    .await?;

    let modules = input.modules.unwrap_or_default();
    let config = services::email_reports::update_config(
        &state.db,
        auth.project_id,
        report_id,
        &input.name,
        &input.recipients,
        &input.schedule,
        &modules,
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok(axum::Json(config))
}

pub async fn delete_email_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "email_reports",
        &auth.allowed_modules,
    )
    .await?;

    services::email_reports::delete_config(&state.db, auth.project_id, report_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn send_test_email_report(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(report_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "email_reports",
        &auth.allowed_modules,
    )
    .await?;

    services::email_reports::send_test_report(&state, auth.project_id, report_id).await?;
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}

// ============================================================================
// IDENTITY
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct IdentityGraphQuery {
    pub visitor_id: Option<String>,
    pub user_id: Option<String>,
    pub account_id: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_user_profiles(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let profiles = services::identity::list_profiles(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;

    Ok(axum::Json(serde_json::json!({ "data": profiles })))
}

pub async fn get_user_profile(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let profile =
        services::identity::get_profile_by_visitor(&state.db, auth.project_id, &visitor_id).await?;
    Ok(axum::Json(profile))
}

pub async fn list_user_aliases(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(user_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let aliases =
        services::identity::list_aliases_for_user(&state.db, auth.project_id, &user_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": aliases })))
}

pub async fn get_identity_graph(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<IdentityGraphQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let graph = services::identity::get_identity_graph(
        &state.db,
        auth.project_id,
        params.visitor_id.as_deref(),
        params.user_id.as_deref(),
        params.account_id.as_deref(),
        params.limit.unwrap_or(100),
    )
    .await?;
    Ok(axum::Json(graph))
}

pub async fn list_account_profiles(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let accounts = services::identity::list_accounts(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": accounts })))
}

pub async fn get_account_profile(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(account_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let account = services::identity::get_account(&state.db, auth.project_id, &account_id).await?;
    Ok(axum::Json(account))
}

pub async fn list_account_members(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(account_id): Path<String>,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let members = services::identity::list_account_members(
        &state.db,
        auth.project_id,
        &account_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": members })))
}

pub async fn get_account_analytics(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(account_id): Path<String>,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let (start_at, end_at) = params.resolve();
    let analytics = services::identity::get_account_analytics(
        &state.db,
        auth.project_id,
        &account_id,
        start_at,
        end_at,
    )
    .await?;
    Ok(axum::Json(analytics))
}

pub async fn list_scim_users(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let users = services::identity::list_scim_users(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": users })))
}

pub async fn get_scim_user(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let user = services::identity::get_scim_user(&state.db, auth.project_id, user_id).await?;
    Ok(axum::Json(user))
}

pub async fn create_scim_user(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::identity::ScimUserInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let user = services::identity::create_scim_user(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(user)))
}

pub async fn update_scim_user(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(user_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::identity::ScimUserInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let user =
        services::identity::update_scim_user(&state.db, auth.project_id, user_id, input).await?;
    Ok(axum::Json(user))
}

pub async fn delete_scim_user(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    services::identity::delete_scim_user(&state.db, auth.project_id, user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_scim_groups(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let groups = services::identity::list_scim_groups(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": groups })))
}

pub async fn get_scim_group(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(group_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let group = services::identity::get_scim_group(&state.db, auth.project_id, group_id).await?;
    Ok(axum::Json(group))
}

pub async fn create_scim_group(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::identity::ScimGroupInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let group = services::identity::create_scim_group(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(group)))
}

pub async fn update_scim_group(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(group_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::identity::ScimGroupInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    let group =
        services::identity::update_scim_group(&state.db, auth.project_id, group_id, input).await?;
    Ok(axum::Json(group))
}

pub async fn delete_scim_group(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(group_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "identity",
        &auth.allowed_modules,
    )
    .await?;

    services::identity::delete_scim_group(&state.db, auth.project_id, group_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================================================
// SEGMENTS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SegmentInput {
    pub name: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SegmentEvalQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SegmentCompareQuery {
    pub segment_ids: String,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct SegmentBreakdownQuery {
    pub property: String,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn list_segments(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    let segments = services::segments::list_segments(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": segments })))
}

pub async fn create_segment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<SegmentInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    let segment = services::segments::create_segment(
        &state.db,
        auth.project_id,
        &input.name,
        input.description.as_deref(),
        input.definition,
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(segment)))
}

pub async fn get_segment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(segment_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    let segment = services::segments::get_segment(&state.db, auth.project_id, segment_id).await?;
    Ok(axum::Json(segment))
}

pub async fn update_segment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(segment_id): Path<Uuid>,
    axum::Json(input): axum::Json<SegmentInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    let segment = services::segments::update_segment(
        &state.db,
        auth.project_id,
        segment_id,
        &input.name,
        input.description.as_deref(),
        input.definition,
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok(axum::Json(segment))
}

pub async fn delete_segment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(segment_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    services::segments::delete_segment(&state.db, auth.project_id, segment_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn evaluate_segment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(segment_id): Path<Uuid>,
    Query(params): Query<SegmentEvalQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let result = services::segments::evaluate_segment(
        &state.db,
        auth.project_id,
        segment_id,
        start,
        end,
        params.limit.unwrap_or(100),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(result))
}

pub async fn compare_segments(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<SegmentCompareQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    let segment_ids: Result<Vec<Uuid>, _> = params
        .segment_ids
        .split(',')
        .map(|id| Uuid::parse_str(id.trim()))
        .collect();
    let segment_ids =
        segment_ids.map_err(|_| AppError::BadRequest("Invalid segment_ids".to_string()))?;
    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let rows =
        services::segments::compare_segments(&state.db, auth.project_id, &segment_ids, start, end)
            .await?;
    Ok(axum::Json(serde_json::json!({ "data": rows })))
}

pub async fn breakdown_segment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(segment_id): Path<Uuid>,
    Query(params): Query<SegmentBreakdownQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "segments",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let rows = services::segments::breakdown_segment(
        &state.db,
        auth.project_id,
        segment_id,
        &params.property,
        start,
        end,
        params.limit.unwrap_or(20),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": rows })))
}

// ============================================================================
// GOVERNANCE
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TrackingPlanInput {
    pub name: String,
    pub description: Option<String>,
    pub enforcement_mode: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct EventSchemaInput {
    pub tracking_plan_id: Option<Uuid>,
    pub event_name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub required_properties: Option<Vec<String>>,
    pub property_schema: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct EventSchemaListQuery {
    pub tracking_plan_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct EventSchemaStatusInput {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct DataDictionaryInput {
    pub entry_type: String,
    pub name: String,
    pub data_type: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub is_pii: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DataDictionaryQuery {
    pub entry_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ViolationQuery {
    pub event_name: Option<String>,
    pub violation_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_tracking_plans(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let plans = services::governance::list_tracking_plans(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": plans })))
}

pub async fn create_tracking_plan(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<TrackingPlanInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let plan = services::governance::create_tracking_plan(
        &state.db,
        auth.project_id,
        &input.name,
        input.description.as_deref(),
        input.enforcement_mode.as_deref().unwrap_or("observe"),
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(plan)))
}

pub async fn get_tracking_plan(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(plan_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let plan = services::governance::get_tracking_plan(&state.db, auth.project_id, plan_id).await?;
    Ok(axum::Json(plan))
}

pub async fn update_tracking_plan(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(plan_id): Path<Uuid>,
    axum::Json(input): axum::Json<TrackingPlanInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let plan = services::governance::update_tracking_plan(
        &state.db,
        auth.project_id,
        plan_id,
        &input.name,
        input.description.as_deref(),
        input.enforcement_mode.as_deref().unwrap_or("observe"),
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok(axum::Json(plan))
}

pub async fn delete_tracking_plan(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(plan_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    services::governance::delete_tracking_plan(&state.db, auth.project_id, plan_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_event_schemas(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<EventSchemaListQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let schemas = services::governance::list_event_schemas(
        &state.db,
        auth.project_id,
        params.tracking_plan_id,
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": schemas })))
}

pub async fn create_event_schema(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<EventSchemaInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let required_properties = input.required_properties.unwrap_or_default();
    let schema = services::governance::create_event_schema(
        &state.db,
        auth.project_id,
        input.tracking_plan_id,
        &input.event_name,
        input.description.as_deref(),
        input.status.as_deref().unwrap_or("draft"),
        &required_properties,
        input
            .property_schema
            .unwrap_or_else(|| serde_json::json!({})),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(schema)))
}

pub async fn get_event_schema(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(schema_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let schema =
        services::governance::get_event_schema(&state.db, auth.project_id, schema_id).await?;
    Ok(axum::Json(schema))
}

pub async fn update_event_schema(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(schema_id): Path<Uuid>,
    axum::Json(input): axum::Json<EventSchemaInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let required_properties = input.required_properties.unwrap_or_default();
    let schema = services::governance::update_event_schema(
        &state.db,
        auth.project_id,
        schema_id,
        input.tracking_plan_id,
        &input.event_name,
        input.description.as_deref(),
        input.status.as_deref().unwrap_or("draft"),
        &required_properties,
        input
            .property_schema
            .unwrap_or_else(|| serde_json::json!({})),
    )
    .await?;
    Ok(axum::Json(schema))
}

pub async fn update_event_schema_status(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(schema_id): Path<Uuid>,
    axum::Json(input): axum::Json<EventSchemaStatusInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let schema = services::governance::update_event_schema_status(
        &state.db,
        auth.project_id,
        schema_id,
        &input.status,
    )
    .await?;
    Ok(axum::Json(schema))
}

pub async fn delete_event_schema(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(schema_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    services::governance::delete_event_schema(&state.db, auth.project_id, schema_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_data_dictionary_entries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DataDictionaryQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let entries = services::governance::list_dictionary_entries(
        &state.db,
        auth.project_id,
        params.entry_type.as_deref(),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": entries })))
}

pub async fn create_data_dictionary_entry(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<DataDictionaryInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let entry = services::governance::create_dictionary_entry(
        &state.db,
        auth.project_id,
        &input.entry_type,
        &input.name,
        input.data_type.as_deref(),
        input.description.as_deref(),
        input.owner.as_deref(),
        input.is_pii.unwrap_or(false),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(entry)))
}

pub async fn update_data_dictionary_entry(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(entry_id): Path<Uuid>,
    axum::Json(input): axum::Json<DataDictionaryInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let entry = services::governance::update_dictionary_entry(
        &state.db,
        auth.project_id,
        entry_id,
        &input.entry_type,
        &input.name,
        input.data_type.as_deref(),
        input.description.as_deref(),
        input.owner.as_deref(),
        input.is_pii.unwrap_or(false),
    )
    .await?;
    Ok(axum::Json(entry))
}

pub async fn delete_data_dictionary_entry(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(entry_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    services::governance::delete_dictionary_entry(&state.db, auth.project_id, entry_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_quality_violations(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<ViolationQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let violations = services::governance::list_quality_violations(
        &state.db,
        auth.project_id,
        params.event_name.as_deref(),
        params.violation_type.as_deref(),
        params.limit.unwrap_or(100),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": violations })))
}

pub async fn get_governance_health(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "governance",
        &auth.allowed_modules,
    )
    .await?;

    let health = services::governance::governance_health(&state.db, auth.project_id).await?;
    Ok(axum::Json(health))
}

// ============================================================================
// FEATURE FLAGS / REMOTE CONFIG
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FeatureFlagInput {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub flag_type: Option<String>,
    pub default_value: Option<serde_json::Value>,
    pub variants: Option<serde_json::Value>,
    pub rollout_percentage: Option<f64>,
    pub targeting_rules: Option<serde_json::Value>,
    pub remote_config: Option<serde_json::Value>,
    pub experiment_id: Option<Uuid>,
    pub guardrail_metrics: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct FeatureFlagEvaluationInput {
    pub visitor_id: String,
    pub user_id: Option<String>,
    pub traits: Option<serde_json::Value>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct FeatureFlagEvalQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteConfigInput {
    pub key: String,
    pub description: Option<String>,
    pub value: Option<serde_json::Value>,
    pub targeting_rules: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

fn default_targeting_rules() -> serde_json::Value {
    serde_json::json!({ "match": "all", "conditions": [] })
}

fn feature_flag_service_input(
    input: &FeatureFlagInput,
) -> services::feature_flags::FeatureFlagInput<'_> {
    services::feature_flags::FeatureFlagInput {
        key: &input.key,
        name: &input.name,
        description: input.description.as_deref(),
        enabled: input.enabled.unwrap_or(false),
        flag_type: input.flag_type.as_deref().unwrap_or("boolean"),
        default_value: input
            .default_value
            .clone()
            .unwrap_or(serde_json::Value::Bool(false)),
        variants: input
            .variants
            .clone()
            .unwrap_or_else(|| serde_json::json!([])),
        rollout_percentage: input.rollout_percentage.unwrap_or(100.0),
        targeting_rules: input
            .targeting_rules
            .clone()
            .unwrap_or_else(default_targeting_rules),
        remote_config: input
            .remote_config
            .clone()
            .unwrap_or_else(|| serde_json::json!({})),
        experiment_id: input.experiment_id,
        guardrail_metrics: input
            .guardrail_metrics
            .clone()
            .unwrap_or_else(|| serde_json::json!([])),
    }
}

fn evaluation_context(
    input: FeatureFlagEvaluationInput,
) -> services::feature_flags::EvaluationContext {
    services::feature_flags::EvaluationContext {
        visitor_id: input.visitor_id,
        user_id: input.user_id,
        traits: input.traits.unwrap_or_else(|| serde_json::json!({})),
        context: input.context.unwrap_or_else(|| serde_json::json!({})),
    }
}

pub async fn list_feature_flags(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let flags = services::feature_flags::list_feature_flags(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": flags })))
}

pub async fn create_feature_flag(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<FeatureFlagInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let flag = services::feature_flags::create_feature_flag(
        &state.db,
        auth.project_id,
        feature_flag_service_input(&input),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(flag)))
}

pub async fn get_feature_flag(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(flag_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let flag =
        services::feature_flags::get_feature_flag(&state.db, auth.project_id, flag_id).await?;
    Ok(axum::Json(flag))
}

pub async fn update_feature_flag(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(flag_id): Path<Uuid>,
    axum::Json(input): axum::Json<FeatureFlagInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let flag = services::feature_flags::update_feature_flag(
        &state.db,
        auth.project_id,
        flag_id,
        feature_flag_service_input(&input),
    )
    .await?;
    Ok(axum::Json(flag))
}

pub async fn delete_feature_flag(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(flag_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    services::feature_flags::delete_feature_flag(&state.db, auth.project_id, flag_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn evaluate_feature_flag(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(key): Path<String>,
    axum::Json(input): axum::Json<FeatureFlagEvaluationInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let result = services::feature_flags::evaluate_feature_flag(
        &state.db,
        auth.project_id,
        &key,
        &evaluation_context(input),
    )
    .await?;
    Ok(axum::Json(result))
}

pub async fn list_feature_flag_evaluations(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(flag_id): Path<Uuid>,
    Query(params): Query<FeatureFlagEvalQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let evaluations = services::feature_flags::list_feature_flag_evaluations(
        &state.db,
        auth.project_id,
        flag_id,
        params.limit.unwrap_or(100),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": evaluations })))
}

pub async fn list_remote_configs(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let entries = services::feature_flags::list_remote_configs(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": entries })))
}

pub async fn create_remote_config(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<RemoteConfigInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let entry = services::feature_flags::create_remote_config(
        &state.db,
        auth.project_id,
        &input.key,
        input.description.as_deref(),
        input.value.unwrap_or_else(|| serde_json::json!({})),
        input
            .targeting_rules
            .unwrap_or_else(default_targeting_rules),
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(entry)))
}

pub async fn get_remote_config(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(entry_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let entry =
        services::feature_flags::get_remote_config(&state.db, auth.project_id, entry_id).await?;
    Ok(axum::Json(entry))
}

pub async fn update_remote_config(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(entry_id): Path<Uuid>,
    axum::Json(input): axum::Json<RemoteConfigInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let entry = services::feature_flags::update_remote_config(
        &state.db,
        auth.project_id,
        entry_id,
        &input.key,
        input.description.as_deref(),
        input.value.unwrap_or_else(|| serde_json::json!({})),
        input
            .targeting_rules
            .unwrap_or_else(default_targeting_rules),
        input.is_active.unwrap_or(true),
    )
    .await?;
    Ok(axum::Json(entry))
}

pub async fn delete_remote_config(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(entry_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    services::feature_flags::delete_remote_config(&state.db, auth.project_id, entry_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn evaluate_remote_config(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(key): Path<String>,
    axum::Json(input): axum::Json<FeatureFlagEvaluationInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "feature_flags",
        &auth.allowed_modules,
    )
    .await?;

    let result = services::feature_flags::evaluate_remote_config(
        &state.db,
        auth.project_id,
        &key,
        &evaluation_context(input),
    )
    .await?;
    Ok(axum::Json(result))
}

// ============================================================================
// PRIVACY / AUDIT
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PrivacySettingsInput {
    pub anonymize_ip: Option<bool>,
    pub respect_dnt: Option<bool>,
    pub bot_filtering: Option<bool>,
    pub consent_required: Option<bool>,
    pub allowed_consent_modes: Option<Vec<String>>,
    pub blocked_user_agents: Option<Vec<String>>,
}

pub async fn get_privacy_settings(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("admin")?;

    let settings = services::privacy::get_privacy_settings(&state.db, auth.project_id).await?;
    Ok(axum::Json(settings))
}

pub async fn update_privacy_settings(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<PrivacySettingsInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("admin")?;

    let current = services::privacy::get_privacy_settings(&state.db, auth.project_id).await?;
    let allowed_consent_modes = input
        .allowed_consent_modes
        .unwrap_or(current.allowed_consent_modes);
    let blocked_user_agents = input
        .blocked_user_agents
        .unwrap_or(current.blocked_user_agents);
    let settings = services::privacy::upsert_privacy_settings(
        &state.db,
        auth.project_id,
        input.anonymize_ip.unwrap_or(current.anonymize_ip),
        input.respect_dnt.unwrap_or(current.respect_dnt),
        input.bot_filtering.unwrap_or(current.bot_filtering),
        input.consent_required.unwrap_or(current.consent_required),
        &allowed_consent_modes,
        &blocked_user_agents,
    )
    .await?;

    services::audit::record_audit_log(
        &state.db,
        auth.project_id,
        "api_key",
        "privacy.settings.update",
        "privacy_settings",
        None,
        serde_json::json!({
            "anonymize_ip": settings.anonymize_ip,
            "respect_dnt": settings.respect_dnt,
            "bot_filtering": settings.bot_filtering,
            "consent_required": settings.consent_required,
            "allowed_consent_modes": settings.allowed_consent_modes,
        }),
    )
    .await?;

    Ok(axum::Json(settings))
}

pub async fn export_visitor_data(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("admin")?;

    let export =
        services::privacy::export_visitor_data(&state.db, auth.project_id, &visitor_id).await?;
    services::audit::record_audit_log(
        &state.db,
        auth.project_id,
        "api_key",
        "privacy.export",
        "visitor",
        Some(&visitor_id),
        serde_json::json!({ "visitor_id": visitor_id }),
    )
    .await?;

    Ok(axum::Json(export))
}

pub async fn delete_visitor_data(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(visitor_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("admin")?;

    let result =
        services::privacy::delete_visitor_data(&state.db, auth.project_id, &visitor_id).await?;
    services::audit::record_audit_log(
        &state.db,
        auth.project_id,
        "api_key",
        "privacy.delete",
        "visitor",
        Some(&visitor_id),
        serde_json::json!({ "visitor_id": visitor_id, "deleted": result.deleted.clone() }),
    )
    .await?;

    Ok(axum::Json(result))
}

pub async fn list_audit_logs(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("admin")?;

    let logs = services::audit::list_audit_logs(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": logs })))
}

// ============================================================================
// ALERTS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateAlert {
    pub name: String,
    pub module: String,
    pub metric: String,
    pub operator: String,
    pub threshold: f64,
    pub window_minutes: Option<i32>,
    pub cooldown_minutes: Option<i32>,
    pub notify_channels: Option<serde_json::Value>,
}

pub async fn list_alerts(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "alerts",
        &auth.allowed_modules,
    )
    .await?;

    let alerts = services::alerts::list_alerts(&state.db, auth.project_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": alerts })))
}

pub async fn create_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateAlert>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "alerts",
        &auth.allowed_modules,
    )
    .await?;

    let window_minutes = input.window_minutes.unwrap_or(60);
    let cooldown_minutes = input.cooldown_minutes.unwrap_or(360);
    let notify_channels = input.notify_channels.unwrap_or(serde_json::json!([]));

    services::alerts::validate_alert_definition(
        &input.module,
        &input.metric,
        &input.operator,
        input.threshold,
        window_minutes,
        cooldown_minutes,
        &notify_channels,
    )
    .map_err(AppError::BadRequest)?;

    let alert = services::alerts::create_alert(
        &state.db,
        auth.project_id,
        &input.name,
        &input.module,
        &input.metric,
        &input.operator,
        input.threshold,
        window_minutes,
        cooldown_minutes,
        notify_channels,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(alert)))
}

pub async fn update_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(alert_id): Path<Uuid>,
    axum::Json(input): axum::Json<CreateAlert>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "alerts",
        &auth.allowed_modules,
    )
    .await?;

    let window_minutes = input.window_minutes.unwrap_or(60);
    let cooldown_minutes = input.cooldown_minutes.unwrap_or(360);
    let notify_channels = input.notify_channels.unwrap_or(serde_json::json!([]));

    services::alerts::validate_alert_definition(
        &input.module,
        &input.metric,
        &input.operator,
        input.threshold,
        window_minutes,
        cooldown_minutes,
        &notify_channels,
    )
    .map_err(AppError::BadRequest)?;

    let alert = services::alerts::update_alert(
        &state.db,
        auth.project_id,
        alert_id,
        &input.name,
        &input.module,
        &input.metric,
        &input.operator,
        input.threshold,
        window_minutes,
        cooldown_minutes,
        notify_channels,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(axum::Json(alert))
}

pub async fn delete_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(alert_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "alerts",
        &auth.allowed_modules,
    )
    .await?;

    services::alerts::delete_alert(&state.db, auth.project_id, alert_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn toggle_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(alert_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "alerts",
        &auth.allowed_modules,
    )
    .await?;

    let alert = services::alerts::toggle_alert(&state.db, auth.project_id, alert_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(alert))
}

// ============================================================================
// A/B TESTING / EXPERIMENTS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateExperiment {
    pub name: String,
    pub description: Option<String>,
    pub variants: serde_json::Value,
    pub goal_id: Option<Uuid>,
}

pub async fn list_experiments(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ab_testing",
        &auth.allowed_modules,
    )
    .await?;

    let experiments = services::experiments::list_experiments(&state.db, auth.project_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": experiments })))
}

pub async fn create_experiment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateExperiment>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "ab_testing",
        &auth.allowed_modules,
    )
    .await?;

    let experiment = services::experiments::create_experiment(
        &state.db,
        auth.project_id,
        &input.name,
        input.description.as_deref(),
        &input.variants,
        input.goal_id,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(experiment)))
}

pub async fn get_experiment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ab_testing",
        &auth.allowed_modules,
    )
    .await?;

    let experiment =
        services::experiments::get_experiment(&state.db, auth.project_id, experiment_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Experiment not found".to_string()))?;
    Ok(axum::Json(experiment))
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatus {
    pub status: String,
}

pub async fn update_experiment_status(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
    axum::Json(input): axum::Json<UpdateStatus>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "ab_testing",
        &auth.allowed_modules,
    )
    .await?;

    let experiment = services::experiments::update_experiment_status(
        &state.db,
        auth.project_id,
        experiment_id,
        &input.status,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(experiment))
}

pub async fn delete_experiment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "ab_testing",
        &auth.allowed_modules,
    )
    .await?;

    services::experiments::delete_experiment(&state.db, auth.project_id, experiment_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn get_experiment_results(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "ab_testing",
        &auth.allowed_modules,
    )
    .await?;

    let (start, end) = params.resolve();
    let results = services::experiments::get_experiment_results(
        &state.db,
        auth.project_id,
        experiment_id,
        start,
        end,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(results))
}

#[derive(Debug, Deserialize)]
pub struct AssignVisitor {
    pub visitor_id: String,
}

pub async fn assign_experiment_visitor(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
    axum::Json(input): axum::Json<AssignVisitor>,
) -> AppResult<axum::Json<serde_json::Value>> {
    auth.require_scope("ingest")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "ab_testing",
        &auth.allowed_modules,
    )
    .await?;

    let variant = services::experiments::assign_visitor(
        &state.db,
        auth.project_id,
        experiment_id,
        &input.visitor_id,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "variant": variant })))
}

// ============================================================================
// SURVEYS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateSurvey {
    pub name: String,
    pub questions: serde_json::Value,
    pub trigger_config: Option<serde_json::Value>,
    pub appearance: Option<serde_json::Value>,
    pub response_limit: Option<i32>,
}

pub async fn list_surveys(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let surveys = services::surveys::list_surveys(&state.db, auth.project_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": surveys })))
}

pub async fn create_survey(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateSurvey>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let survey = services::surveys::create_survey(
        &state.db,
        auth.project_id,
        &input.name,
        &input.questions,
        &input.trigger_config.unwrap_or(serde_json::json!({})),
        &input.appearance.unwrap_or(serde_json::json!({})),
        input.response_limit,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(survey)))
}

pub async fn get_survey(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let survey = services::surveys::get_survey(&state.db, auth.project_id, survey_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Survey not found".to_string()))?;
    Ok(axum::Json(survey))
}

pub async fn update_survey(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
    axum::Json(input): axum::Json<CreateSurvey>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let survey = services::surveys::update_survey(
        &state.db,
        auth.project_id,
        survey_id,
        &input.name,
        &input.questions,
        &input.trigger_config.unwrap_or(serde_json::json!({})),
        &input.appearance.unwrap_or(serde_json::json!({})),
        input.response_limit,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(survey))
}

pub async fn delete_survey(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    services::surveys::delete_survey(&state.db, auth.project_id, survey_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn update_survey_status(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
    axum::Json(input): axum::Json<UpdateStatus>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let survey = services::surveys::update_survey_status(
        &state.db,
        auth.project_id,
        survey_id,
        &input.status,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(survey))
}

#[derive(Debug, Deserialize)]
pub struct PaginatedParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_survey_responses(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let responses = services::surveys::get_survey_responses(
        &state.db,
        auth.project_id,
        survey_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": responses })))
}

pub async fn get_survey_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let stats = services::surveys::get_survey_stats(&state.db, auth.project_id, survey_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(stats))
}

#[derive(Debug, Deserialize)]
pub struct SurveyAnalysisQuery {
    pub question_id: Option<String>,
}

pub async fn get_survey_nps(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
    Query(params): Query<SurveyAnalysisQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let report = services::surveys::get_nps_report(
        &state.db,
        auth.project_id,
        survey_id,
        params.question_id.as_deref(),
    )
    .await?;
    Ok(axum::Json(report))
}

pub async fn get_survey_sentiment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
    Query(params): Query<SurveyAnalysisQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let report = services::surveys::get_sentiment_report(
        &state.db,
        auth.project_id,
        survey_id,
        params.question_id.as_deref(),
    )
    .await?;
    Ok(axum::Json(report))
}

pub async fn get_active_surveys(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let surveys = services::surveys::get_active_surveys(&state.db, auth.project_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": surveys })))
}

pub async fn list_guides(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let guides = services::surveys::list_guides(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": guides })))
}

pub async fn get_active_guides(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let guides = services::surveys::get_active_guides(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": guides })))
}

pub async fn create_guide(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::surveys::GuideInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let guide = services::surveys::create_guide(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(guide)))
}

pub async fn get_guide(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(guide_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let guide = services::surveys::get_guide(&state.db, auth.project_id, guide_id).await?;
    Ok(axum::Json(guide))
}

pub async fn update_guide(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(guide_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::surveys::GuideInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let guide =
        services::surveys::update_guide(&state.db, auth.project_id, guide_id, input).await?;
    Ok(axum::Json(guide))
}

pub async fn delete_guide(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(guide_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    services::surveys::delete_guide(&state.db, auth.project_id, guide_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn update_guide_status(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(guide_id): Path<Uuid>,
    axum::Json(input): axum::Json<UpdateStatus>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let guide =
        services::surveys::update_guide_status(&state.db, auth.project_id, guide_id, &input.status)
            .await?;
    Ok(axum::Json(guide))
}

pub async fn list_guide_events(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(guide_id): Path<Uuid>,
    Query(params): Query<PaginatedParams>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let events = services::surveys::list_guide_events(
        &state.db,
        auth.project_id,
        guide_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": events })))
}

pub async fn record_guide_event(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(guide_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::surveys::GuideEventInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let event =
        services::surveys::record_guide_event(&state.db, auth.project_id, guide_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(event)))
}

pub async fn get_guide_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(guide_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "surveys",
        &auth.allowed_modules,
    )
    .await?;

    let stats = services::surveys::get_guide_stats(&state.db, auth.project_id, guide_id).await?;
    Ok(axum::Json(stats))
}

// ============================================================================
// SESSION REPLAY
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SessionReplayQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_session_recordings(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<SessionReplayQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "session_replay",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let recordings = services::session_replay::list_recordings(
        &state.db,
        auth.project_id,
        start,
        end,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;

    Ok(axum::Json(serde_json::json!({ "data": recordings })))
}

pub async fn get_session_recording(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(recording_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "session_replay",
        &auth.allowed_modules,
    )
    .await?;

    let recording =
        services::session_replay::get_recording(&state.db, auth.project_id, recording_id).await?;
    Ok(axum::Json(recording))
}

// ============================================================================
// WEB VITALS
// ============================================================================

pub async fn get_vitals_summary(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "webvitals",
        &auth.allowed_modules,
    )
    .await?;

    let (start, end) = params.resolve();
    let summary =
        services::webvitals::get_vitals_summary(&state.db, auth.project_id, start, end).await?;
    Ok(axum::Json(summary))
}

#[derive(Debug, Deserialize)]
pub struct VitalsPageQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn get_vitals_by_page(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<VitalsPageQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "webvitals",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let pages = services::webvitals::get_vitals_by_page(
        &state.db,
        auth.project_id,
        start,
        end,
        params.limit.unwrap_or(20),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": pages })))
}

#[derive(Debug, Deserialize)]
pub struct VitalsTimeseriesQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub metric: Option<String>,
}

pub async fn get_vitals_timeseries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<VitalsTimeseriesQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "webvitals",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let metric = params.metric.as_deref().unwrap_or("LCP");
    let ts =
        services::webvitals::get_vitals_timeseries(&state.db, auth.project_id, start, end, metric)
            .await?;
    Ok(axum::Json(serde_json::json!({ "data": ts })))
}

// ============================================================================
// ERROR TRACKING
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ErrorQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_error_groups(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<ErrorQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let groups = services::error_tracking::get_error_groups(
        &state.db,
        auth.project_id,
        start,
        end,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": groups })))
}

#[derive(Debug, Deserialize)]
pub struct ErrorDetailQuery {
    pub message: Option<String>,
    pub fingerprint: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn get_error_detail(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<ErrorDetailQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    if params.message.is_none() && params.fingerprint.is_none() {
        return Err(AppError::BadRequest(
            "Either message or fingerprint is required".to_string(),
        ));
    }
    let errors = services::error_tracking::get_error_detail(
        &state.db,
        auth.project_id,
        params.message.as_deref(),
        params.fingerprint.as_deref(),
        start,
        end,
        params.limit.unwrap_or(20),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": errors })))
}

pub async fn get_error_timeseries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let (start, end) = params.resolve();
    let ts = services::error_tracking::get_error_timeseries(&state.db, auth.project_id, start, end)
        .await?;
    Ok(axum::Json(serde_json::json!({ "data": ts })))
}

pub async fn get_error_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let (start, end) = params.resolve();
    let stats =
        services::error_tracking::get_error_stats(&state.db, auth.project_id, start, end).await?;
    Ok(axum::Json(stats))
}

#[derive(Debug, Deserialize)]
pub struct ReleaseInput {
    pub version: String,
    pub environment: Option<String>,
    pub commit_sha: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListReleasesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_releases(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<ListReleasesQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let releases = services::error_tracking::list_releases(
        &state.db,
        auth.project_id,
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": releases })))
}

pub async fn create_release(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<ReleaseInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let release = services::error_tracking::create_release(
        &state.db,
        auth.project_id,
        &input.version,
        input.environment.as_deref().unwrap_or("production"),
        input.commit_sha.as_deref(),
        input.deployed_at,
        input.metadata.unwrap_or_else(|| serde_json::json!({})),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(release)))
}

pub async fn delete_release(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(release_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    services::error_tracking::delete_release(&state.db, auth.project_id, release_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SourceMapInput {
    pub release_version: String,
    pub environment: Option<String>,
    pub minified_url: String,
    pub source_map_url: Option<String>,
    pub artifacts: Option<serde_json::Value>,
    pub uploaded_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourceMapQuery {
    pub release_version: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_source_maps(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<SourceMapQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let source_maps = services::error_tracking::list_source_maps(
        &state.db,
        auth.project_id,
        params.release_version.as_deref(),
        params.limit.unwrap_or(50),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": source_maps })))
}

pub async fn register_source_map(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<SourceMapInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    let source_map = services::error_tracking::register_source_map(
        &state.db,
        auth.project_id,
        &input.release_version,
        input.environment.as_deref().unwrap_or("production"),
        &input.minified_url,
        input.source_map_url.as_deref(),
        input.artifacts.unwrap_or_else(|| serde_json::json!({})),
        input.uploaded_by.as_deref(),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(source_map)))
}

pub async fn delete_source_map(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(source_map_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "error_tracking",
        &auth.allowed_modules,
    )
    .await?;

    services::error_tracking::delete_source_map(&state.db, auth.project_id, source_map_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub level: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_logs(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<LogsQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "logs", &auth.allowed_modules)
        .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let logs = services::error_tracking::list_logs(
        &state.db,
        auth.project_id,
        start,
        end,
        services::error_tracking::LogFilters {
            level: params.level,
            release: params.release,
            environment: params.environment,
            search: params.search,
            limit: params.limit.unwrap_or(100),
            offset: params.offset.unwrap_or(0),
        },
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": logs })))
}

pub async fn get_log_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "logs", &auth.allowed_modules)
        .await?;

    let (start, end) = params.resolve();
    let stats =
        services::error_tracking::get_log_stats(&state.db, auth.project_id, start, end).await?;
    Ok(axum::Json(stats))
}

// ============================================================================
// HEATMAPS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct HeatmapQuery {
    pub path: String,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

pub async fn get_click_heatmap(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<HeatmapQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let points =
        services::heatmaps::get_click_heatmap(&state.db, auth.project_id, &params.path, start, end)
            .await?;
    Ok(axum::Json(serde_json::json!({ "data": points })))
}

#[derive(Debug, Deserialize)]
pub struct HeatmapStatsQuery {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn get_click_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<HeatmapStatsQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let stats = services::heatmaps::get_click_stats(
        &state.db,
        auth.project_id,
        start,
        end,
        params.limit.unwrap_or(20),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": stats })))
}

pub async fn list_visual_event_labels(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let labels = services::heatmaps::list_visual_event_labels(&state.db, auth.project_id).await?;
    Ok(axum::Json(serde_json::json!({ "data": labels })))
}

pub async fn create_visual_event_label(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<services::heatmaps::VisualEventLabelInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let label =
        services::heatmaps::create_visual_event_label(&state.db, auth.project_id, input).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(label)))
}

pub async fn update_visual_event_label(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(label_id): Path<Uuid>,
    axum::Json(input): axum::Json<services::heatmaps::VisualEventLabelInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let label =
        services::heatmaps::update_visual_event_label(&state.db, auth.project_id, label_id, input)
            .await?;
    Ok(axum::Json(label))
}

pub async fn delete_visual_event_label(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(label_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    services::heatmaps::delete_visual_event_label(&state.db, auth.project_id, label_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_visual_event_label_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<HeatmapStatsQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let stats = services::heatmaps::list_visual_event_label_stats(
        &state.db,
        auth.project_id,
        start,
        end,
        params.limit.unwrap_or(50),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": stats })))
}

pub async fn get_visual_event_label_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(label_id): Path<Uuid>,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let (start, end) = params.resolve();
    let stats = services::heatmaps::get_visual_event_label_stats(
        &state.db,
        auth.project_id,
        label_id,
        start,
        end,
    )
    .await?;
    Ok(axum::Json(stats))
}

#[derive(Debug, Deserialize)]
pub struct FrictionQuery {
    pub path: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn get_friction_signals(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<FrictionQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(
        &state,
        auth.project_id,
        "heatmaps",
        &auth.allowed_modules,
    )
    .await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let signals = services::heatmaps::detect_friction_signals(
        &state.db,
        auth.project_id,
        start,
        end,
        params.path.as_deref(),
        params.limit.unwrap_or(50),
    )
    .await?;
    Ok(axum::Json(serde_json::json!({ "data": signals })))
}
