use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::{campaigns, query as qsvc};

const DASHBOARD_COLUMNS: &str = "id, project_id, name, description, layout, widgets, \
    is_default, created_at, updated_at";
const REPORT_COLUMNS: &str = "id, project_id, name, description, report_type, params, \
    visualization, is_active, created_at, updated_at";
const EXPLORER_COLUMNS: &str = "id, project_id, report_type, query, result, row_count, created_at";

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomDashboard {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub layout: serde_json::Value,
    pub widgets: serde_json::Value,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedReport {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub report_type: String,
    pub params: serde_json::Value,
    pub visualization: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QueryExplorerRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub report_type: String,
    pub query: serde_json::Value,
    pub result: serde_json::Value,
    pub row_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerRequest {
    pub report_type: String,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplorerResponse {
    pub run: QueryExplorerRun,
    pub summary: String,
}

pub async fn list_dashboards(db: &PgPool, project_id: Uuid) -> AppResult<Vec<CustomDashboard>> {
    let dashboards = sqlx::query_as(&format!(
        "SELECT {DASHBOARD_COLUMNS} FROM custom_dashboards \
         WHERE project_id = $1 ORDER BY is_default DESC, created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(dashboards)
}

pub async fn get_dashboard(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
) -> AppResult<CustomDashboard> {
    let dashboard = sqlx::query_as(&format!(
        "SELECT {DASHBOARD_COLUMNS} FROM custom_dashboards WHERE id = $1 AND project_id = $2"
    ))
    .bind(dashboard_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Dashboard not found".to_string()))?;
    Ok(dashboard)
}

pub async fn create_dashboard(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    layout: serde_json::Value,
    widgets: serde_json::Value,
    is_default: bool,
) -> AppResult<CustomDashboard> {
    validate_name(name, "dashboard name")?;
    validate_widgets(&widgets)?;

    let dashboard = sqlx::query_as(&format!(
        "INSERT INTO custom_dashboards \
         (project_id, name, description, layout, widgets, is_default) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {DASHBOARD_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name.trim())
    .bind(description.map(str::trim))
    .bind(layout)
    .bind(widgets)
    .bind(is_default)
    .fetch_one(db)
    .await?;
    Ok(dashboard)
}

pub async fn update_dashboard(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
    name: &str,
    description: Option<&str>,
    layout: serde_json::Value,
    widgets: serde_json::Value,
    is_default: bool,
) -> AppResult<CustomDashboard> {
    validate_name(name, "dashboard name")?;
    validate_widgets(&widgets)?;

    let dashboard = sqlx::query_as(&format!(
        "UPDATE custom_dashboards SET \
           name = $3, description = $4, layout = $5, widgets = $6, is_default = $7, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING {DASHBOARD_COLUMNS}"
    ))
    .bind(dashboard_id)
    .bind(project_id)
    .bind(name.trim())
    .bind(description.map(str::trim))
    .bind(layout)
    .bind(widgets)
    .bind(is_default)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Dashboard not found".to_string()))?;
    Ok(dashboard)
}

pub async fn delete_dashboard(db: &PgPool, project_id: Uuid, dashboard_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM custom_dashboards WHERE id = $1 AND project_id = $2")
        .bind(dashboard_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Dashboard not found".to_string()));
    }
    Ok(())
}

pub async fn list_reports(db: &PgPool, project_id: Uuid) -> AppResult<Vec<SavedReport>> {
    let reports = sqlx::query_as(&format!(
        "SELECT {REPORT_COLUMNS} FROM saved_reports \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(reports)
}

pub async fn get_report(db: &PgPool, project_id: Uuid, report_id: Uuid) -> AppResult<SavedReport> {
    let report = sqlx::query_as(&format!(
        "SELECT {REPORT_COLUMNS} FROM saved_reports WHERE id = $1 AND project_id = $2"
    ))
    .bind(report_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Report not found".to_string()))?;
    Ok(report)
}

pub async fn create_report(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    report_type: &str,
    params: serde_json::Value,
    visualization: &str,
    is_active: bool,
) -> AppResult<SavedReport> {
    validate_name(name, "report name")?;
    validate_report_type(report_type)?;

    let report = sqlx::query_as(&format!(
        "INSERT INTO saved_reports \
         (project_id, name, description, report_type, params, visualization, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING {REPORT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name.trim())
    .bind(description.map(str::trim))
    .bind(report_type.trim())
    .bind(params)
    .bind(visualization.trim())
    .bind(is_active)
    .fetch_one(db)
    .await?;
    Ok(report)
}

pub async fn update_report(
    db: &PgPool,
    project_id: Uuid,
    report_id: Uuid,
    name: &str,
    description: Option<&str>,
    report_type: &str,
    params: serde_json::Value,
    visualization: &str,
    is_active: bool,
) -> AppResult<SavedReport> {
    validate_name(name, "report name")?;
    validate_report_type(report_type)?;

    let report = sqlx::query_as(&format!(
        "UPDATE saved_reports SET \
           name = $3, description = $4, report_type = $5, params = $6, \
           visualization = $7, is_active = $8, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING {REPORT_COLUMNS}"
    ))
    .bind(report_id)
    .bind(project_id)
    .bind(name.trim())
    .bind(description.map(str::trim))
    .bind(report_type.trim())
    .bind(params)
    .bind(visualization.trim())
    .bind(is_active)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Report not found".to_string()))?;
    Ok(report)
}

pub async fn delete_report(db: &PgPool, project_id: Uuid, report_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM saved_reports WHERE id = $1 AND project_id = $2")
        .bind(report_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Report not found".to_string()));
    }
    Ok(())
}

pub async fn run_saved_report(
    db: &PgPool,
    project_id: Uuid,
    report_id: Uuid,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
) -> AppResult<ExplorerResponse> {
    let report = get_report(db, project_id, report_id).await?;
    if !report.is_active {
        return Err(AppError::BadRequest("Report is inactive".to_string()));
    }

    let request = ExplorerRequest {
        report_type: report.report_type,
        start_at: start_at.or_else(|| date_from_params(&report.params, "start_at")),
        end_at: end_at.or_else(|| date_from_params(&report.params, "end_at")),
        limit: int_from_params(&report.params, "limit"),
        offset: int_from_params(&report.params, "offset"),
        params: Some(report.params),
    };
    run_explorer(db, project_id, request).await
}

pub async fn run_explorer(
    db: &PgPool,
    project_id: Uuid,
    request: ExplorerRequest,
) -> AppResult<ExplorerResponse> {
    let report_type = validate_report_type(&request.report_type)?;
    let (start, end) = resolve_range(request.start_at, request.end_at)?;
    let today = Utc::now().date_naive();
    let limit = request.limit.unwrap_or(20).clamp(1, 100);
    let offset = request.offset.unwrap_or(0).max(0);

    let (result, row_count, summary) = match report_type {
        "stats" => {
            let stats = qsvc::fetch_stats(db, project_id, start, end, today).await?;
            let events = qsvc::fetch_events_count(db, project_id, start, end, today).await?;
            (
                json!({
                    "pageviews": stats.0,
                    "visitors": stats.1,
                    "sessions": stats.2,
                    "bounce_rate": percent(stats.3, stats.2),
                    "avg_duration": if stats.2 > 0 { (stats.4 as f64) / (stats.2 as f64) / 1000.0 } else { 0.0 },
                    "events": events,
                }),
                1,
                "Summary metrics for the selected date range.".to_string(),
            )
        }
        "timeseries" => {
            let data = qsvc::fetch_timeseries(db, project_id, start, end, today).await?;
            let row_count = data.len() as i32;
            (
                json!({ "data": data }),
                row_count,
                format!("Returned {row_count} daily trend rows."),
            )
        }
        "pages" => {
            let data = qsvc::fetch_pages(db, project_id, start, end, today, limit, offset).await?;
            table_result("page rows", data)
        }
        "referrers" => {
            let data =
                qsvc::fetch_referrers(db, project_id, start, end, today, limit, offset).await?;
            table_result("referrer rows", data)
        }
        "events" => {
            let data = qsvc::fetch_events(db, project_id, start, end, today, limit, offset).await?;
            table_result("event rows", data)
        }
        "devices" => {
            let data =
                qsvc::fetch_devices(db, project_id, start, end, today, limit, offset).await?;
            table_result("device rows", data)
        }
        "geo" => {
            let data = qsvc::fetch_geo(db, project_id, start, end, today, limit, offset).await?;
            table_result("geo rows", data)
        }
        "campaigns" => {
            let data = campaigns::get_campaign_stats(db, project_id, start, end).await?;
            let values = serde_json::to_value(data).unwrap_or_else(|_| json!([]));
            let row_count = values.as_array().map(|rows| rows.len()).unwrap_or(0) as i32;
            (
                json!({ "data": values }),
                row_count,
                format!("Returned {row_count} campaign rows."),
            )
        }
        _ => unreachable!("validated report type"),
    };

    let query = json!({
        "report_type": report_type,
        "start_at": start,
        "end_at": end,
        "limit": limit,
        "offset": offset,
        "params": request.params.unwrap_or_else(|| json!({})),
    });

    let run = sqlx::query_as(&format!(
        "INSERT INTO query_explorer_runs \
         (project_id, report_type, query, result, row_count) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING {EXPLORER_COLUMNS}"
    ))
    .bind(project_id)
    .bind(report_type)
    .bind(query)
    .bind(result)
    .bind(row_count)
    .fetch_one(db)
    .await?;

    Ok(ExplorerResponse { run, summary })
}

pub async fn list_explorer_runs(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<QueryExplorerRun>> {
    let runs = sqlx::query_as(&format!(
        "SELECT {EXPLORER_COLUMNS} FROM query_explorer_runs \
         WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(runs)
}

fn validate_report_type(report_type: &str) -> AppResult<&'static str> {
    match report_type.trim() {
        "stats" => Ok("stats"),
        "timeseries" => Ok("timeseries"),
        "pages" => Ok("pages"),
        "referrers" => Ok("referrers"),
        "events" => Ok("events"),
        "devices" => Ok("devices"),
        "geo" => Ok("geo"),
        "campaigns" => Ok("campaigns"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported report_type: {other}"
        ))),
    }
}

fn validate_name(name: &str, field: &str) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    Ok(())
}

fn validate_widgets(widgets: &serde_json::Value) -> AppResult<()> {
    if !widgets.is_array() {
        return Err(AppError::BadRequest("widgets must be an array".to_string()));
    }
    Ok(())
}

fn resolve_range(
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
) -> AppResult<(DateTime<Utc>, DateTime<Utc>)> {
    let end = end_at.unwrap_or_else(Utc::now);
    let start = start_at.unwrap_or_else(|| end - Duration::days(30));
    if start >= end {
        return Err(AppError::BadRequest(
            "start_at must be before end_at".to_string(),
        ));
    }
    Ok((start, end))
}

fn table_result(label: &str, data: Vec<serde_json::Value>) -> (serde_json::Value, i32, String) {
    let row_count = data.len() as i32;
    (
        json!({ "data": data }),
        row_count,
        format!("Returned {row_count} {label}."),
    )
}

fn percent(numerator: i64, denominator: i64) -> f64 {
    if denominator > 0 {
        (numerator as f64) / (denominator as f64) * 100.0
    } else {
        0.0
    }
}

fn date_from_params(params: &serde_json::Value, key: &str) -> Option<DateTime<Utc>> {
    params
        .get(key)
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<DateTime<Utc>>().ok())
}

fn int_from_params(params: &serde_json::Value, key: &str) -> Option<i64> {
    params.get(key).and_then(|value| value.as_i64())
}

#[cfg(test)]
mod tests {
    use super::{resolve_range, validate_report_type, validate_widgets};
    use chrono::{Duration, Utc};
    use serde_json::json;

    #[test]
    fn validates_supported_report_types() {
        assert!(validate_report_type("pages").is_ok());
        assert!(validate_report_type("campaigns").is_ok());
        assert!(validate_report_type("raw_sql").is_err());
    }

    #[test]
    fn rejects_invalid_dashboard_widgets() {
        assert!(validate_widgets(&json!([])).is_ok());
        assert!(validate_widgets(&json!({ "bad": true })).is_err());
    }

    #[test]
    fn rejects_inverted_date_ranges() {
        let end = Utc::now();
        let start = end + Duration::days(1);
        assert!(resolve_range(Some(start), Some(end)).is_err());
    }
}
