use axum::extract::{Path, Query};
use axum::http::header;
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
    services::modules::require_module_read(&state, auth.project_id, "exports", &auth.allowed_modules).await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));

    let csv = match report_type.as_str() {
        "stats" => services::exports::export_stats_csv(&state.db, auth.project_id, start, end).await,
        "pages" => services::exports::export_pages_csv(&state.db, auth.project_id, start, end).await,
        "referrers" => services::exports::export_referrers_csv(&state.db, auth.project_id, start, end).await,
        "events" => services::exports::export_events_csv(&state.db, auth.project_id, start, end).await,
        "devices" => services::exports::export_devices_csv(&state.db, auth.project_id, start, end).await,
        "geo" => services::exports::export_geo_csv(&state.db, auth.project_id, start, end).await,
        "campaigns" => services::exports::export_campaigns_csv(&state.db, auth.project_id, start, end).await,
        _ => return Err(AppError::BadRequest(format!("Unknown report type: {report_type}"))),
    }.map_err(|e| AppError::Internal(e.to_string()))?;

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
    services::modules::require_module_write(&state, auth.project_id, "sharing", &auth.allowed_modules).await?;

    let name = input.name.unwrap_or_else(|| "Shared Dashboard".to_string());
    let modules = input.modules.unwrap_or_default();
    let dashboard = services::sharing::create_shared_dashboard(
        &state.db, auth.project_id, &name, &modules, input.password.as_deref(), input.expires_at,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(dashboard)))
}

pub async fn list_shared_dashboards(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "sharing", &auth.allowed_modules).await?;

    let dashboards = services::sharing::list_shared_dashboards(&state.db, auth.project_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": dashboards })))
}

pub async fn delete_shared_dashboard(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "sharing", &auth.allowed_modules).await?;

    services::sharing::delete_shared_dashboard(&state.db, auth.project_id, id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
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
    services::modules::require_module_read(&state, auth.project_id, "alerts", &auth.allowed_modules).await?;

    let alerts = services::alerts::list_alerts(&state.db, auth.project_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": alerts })))
}

pub async fn create_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateAlert>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "alerts", &auth.allowed_modules).await?;

    let alert = services::alerts::create_alert(
        &state.db, auth.project_id, &input.name, &input.module, &input.metric,
        &input.operator, input.threshold, input.window_minutes.unwrap_or(60),
        input.cooldown_minutes.unwrap_or(360), input.notify_channels.unwrap_or(serde_json::json!([])),
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(alert)))
}

pub async fn update_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(alert_id): Path<Uuid>,
    axum::Json(input): axum::Json<CreateAlert>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "alerts", &auth.allowed_modules).await?;

    let alert = services::alerts::update_alert(
        &state.db, auth.project_id, alert_id, &input.name, &input.module, &input.metric,
        &input.operator, input.threshold, input.window_minutes.unwrap_or(60),
        input.cooldown_minutes.unwrap_or(360), input.notify_channels.unwrap_or(serde_json::json!([])),
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(axum::Json(alert))
}

pub async fn delete_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(alert_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "alerts", &auth.allowed_modules).await?;

    services::alerts::delete_alert(&state.db, auth.project_id, alert_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn toggle_alert(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(alert_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "alerts", &auth.allowed_modules).await?;

    let alert = services::alerts::toggle_alert(&state.db, auth.project_id, alert_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
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
    services::modules::require_module_read(&state, auth.project_id, "ab_testing", &auth.allowed_modules).await?;

    let experiments = services::experiments::list_experiments(&state.db, auth.project_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": experiments })))
}

pub async fn create_experiment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateExperiment>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "ab_testing", &auth.allowed_modules).await?;

    let experiment = services::experiments::create_experiment(
        &state.db, auth.project_id, &input.name, input.description.as_deref(),
        &input.variants, input.goal_id,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(experiment)))
}

pub async fn get_experiment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "ab_testing", &auth.allowed_modules).await?;

    let experiment = services::experiments::get_experiment(&state.db, auth.project_id, experiment_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?
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
    services::modules::require_module_write(&state, auth.project_id, "ab_testing", &auth.allowed_modules).await?;

    let experiment = services::experiments::update_experiment_status(
        &state.db, auth.project_id, experiment_id, &input.status,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(experiment))
}

pub async fn delete_experiment(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "ab_testing", &auth.allowed_modules).await?;

    services::experiments::delete_experiment(&state.db, auth.project_id, experiment_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn get_experiment_results(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(experiment_id): Path<Uuid>,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "ab_testing", &auth.allowed_modules).await?;

    let (start, end) = params.resolve();
    let results = services::experiments::get_experiment_results(
        &state.db, auth.project_id, experiment_id, start, end,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;
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
    services::modules::require_module_write(&state, auth.project_id, "ab_testing", &auth.allowed_modules).await?;

    let variant = services::experiments::assign_visitor(
        &state.db, auth.project_id, experiment_id, &input.visitor_id,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;
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
    services::modules::require_module_read(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let surveys = services::surveys::list_surveys(&state.db, auth.project_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": surveys })))
}

pub async fn create_survey(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    axum::Json(input): axum::Json<CreateSurvey>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let survey = services::surveys::create_survey(
        &state.db, auth.project_id, &input.name, &input.questions,
        &input.trigger_config.unwrap_or(serde_json::json!({})),
        &input.appearance.unwrap_or(serde_json::json!({})),
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(survey)))
}

pub async fn get_survey(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let survey = services::surveys::get_survey(&state.db, auth.project_id, survey_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?
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
    services::modules::require_module_write(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let survey = services::surveys::update_survey(
        &state.db, auth.project_id, survey_id, &input.name, &input.questions,
        &input.trigger_config.unwrap_or(serde_json::json!({})),
        &input.appearance.unwrap_or(serde_json::json!({})),
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(survey))
}

pub async fn delete_survey(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    services::surveys::delete_survey(&state.db, auth.project_id, survey_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn update_survey_status(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
    axum::Json(input): axum::Json<UpdateStatus>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_write(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let survey = services::surveys::update_survey_status(
        &state.db, auth.project_id, survey_id, &input.status,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;
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
    services::modules::require_module_read(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let responses = services::surveys::get_survey_responses(
        &state.db, auth.project_id, survey_id, params.limit.unwrap_or(50), params.offset.unwrap_or(0),
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": responses })))
}

pub async fn get_survey_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Path(survey_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let stats = services::surveys::get_survey_stats(&state.db, auth.project_id, survey_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(stats))
}

pub async fn get_active_surveys(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "surveys", &auth.allowed_modules).await?;

    let surveys = services::surveys::get_active_surveys(&state.db, auth.project_id)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(serde_json::json!({ "data": surveys })))
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
    services::modules::require_module_read(&state, auth.project_id, "webvitals", &auth.allowed_modules).await?;

    let (start, end) = params.resolve();
    let summary = services::webvitals::get_vitals_summary(&state.db, auth.project_id, start, end)
        .await?;
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
    services::modules::require_module_read(&state, auth.project_id, "webvitals", &auth.allowed_modules).await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let pages = services::webvitals::get_vitals_by_page(
        &state.db, auth.project_id, start, end, params.limit.unwrap_or(20),
    ).await?;
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
    services::modules::require_module_read(&state, auth.project_id, "webvitals", &auth.allowed_modules).await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let metric = params.metric.as_deref().unwrap_or("LCP");
    let ts = services::webvitals::get_vitals_timeseries(
        &state.db, auth.project_id, start, end, metric,
    ).await?;
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
    services::modules::require_module_read(&state, auth.project_id, "error_tracking", &auth.allowed_modules).await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let groups = services::error_tracking::get_error_groups(
        &state.db, auth.project_id, start, end, params.limit.unwrap_or(50), params.offset.unwrap_or(0),
    ).await?;
    Ok(axum::Json(serde_json::json!({ "data": groups })))
}

#[derive(Debug, Deserialize)]
pub struct ErrorDetailQuery {
    pub message: String,
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
    services::modules::require_module_read(&state, auth.project_id, "error_tracking", &auth.allowed_modules).await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let errors = services::error_tracking::get_error_detail(
        &state.db, auth.project_id, &params.message, start, end, params.limit.unwrap_or(20),
    ).await?;
    Ok(axum::Json(serde_json::json!({ "data": errors })))
}

pub async fn get_error_timeseries(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "error_tracking", &auth.allowed_modules).await?;

    let (start, end) = params.resolve();
    let ts = services::error_tracking::get_error_timeseries(&state.db, auth.project_id, start, end).await?;
    Ok(axum::Json(serde_json::json!({ "data": ts })))
}

pub async fn get_error_stats(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
    Query(params): Query<DateRangeQuery>,
) -> AppResult<impl IntoResponse> {
    auth.require_scope("query")?;
    services::modules::require_module_read(&state, auth.project_id, "error_tracking", &auth.allowed_modules).await?;

    let (start, end) = params.resolve();
    let stats = services::error_tracking::get_error_stats(&state.db, auth.project_id, start, end).await?;
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
    services::modules::require_module_read(&state, auth.project_id, "heatmaps", &auth.allowed_modules).await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let points = services::heatmaps::get_click_heatmap(
        &state.db, auth.project_id, &params.path, start, end,
    ).await?;
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
    services::modules::require_module_read(&state, auth.project_id, "heatmaps", &auth.allowed_modules).await?;

    let end = params.end_at.unwrap_or_else(Utc::now);
    let start = params.start_at.unwrap_or_else(|| end - Duration::days(30));
    let stats = services::heatmaps::get_click_stats(
        &state.db, auth.project_id, start, end, params.limit.unwrap_or(20),
    ).await?;
    Ok(axum::Json(serde_json::json!({ "data": stats })))
}
