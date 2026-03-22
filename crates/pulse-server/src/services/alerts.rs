use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::state::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRule {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub module: String,
    pub metric: String,
    pub operator: String,
    pub threshold: f64,
    pub window_minutes: i32,
    pub cooldown_minutes: i32,
    pub notify_channels: serde_json::Value,
    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const ALERT_COLUMNS: &str = "id, project_id, name, module, metric, operator, threshold, \
    window_minutes, cooldown_minutes, notify_channels, is_active, last_triggered_at, \
    created_at, updated_at";

const VALID_METRICS: &[&str] = &[
    "pageviews",
    "visitors",
    "bounce_rate",
    "error_count",
    "avg_duration",
];

const VALID_OPERATORS: &[&str] = &["gt", "lt", "gte", "lte", "eq"];

/// Create a new alert rule.
pub async fn create_alert(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    module: &str,
    metric: &str,
    operator: &str,
    threshold: f64,
    window_minutes: i32,
    cooldown_minutes: i32,
    notify_channels: serde_json::Value,
) -> Result<AlertRule, sqlx::Error> {
    let alert: AlertRule = sqlx::query_as(&format!(
        "INSERT INTO alert_rules (project_id, name, module, metric, operator, threshold, \
         window_minutes, cooldown_minutes, notify_channels) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING {ALERT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name)
    .bind(module)
    .bind(metric)
    .bind(operator)
    .bind(threshold)
    .bind(window_minutes)
    .bind(cooldown_minutes)
    .bind(&notify_channels)
    .fetch_one(db)
    .await?;

    Ok(alert)
}

/// List all alert rules for a project.
pub async fn list_alerts(
    db: &PgPool,
    project_id: Uuid,
) -> Result<Vec<AlertRule>, sqlx::Error> {
    let alerts: Vec<AlertRule> = sqlx::query_as(&format!(
        "SELECT {ALERT_COLUMNS} FROM alert_rules WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(alerts)
}

/// Update an alert rule.
pub async fn update_alert(
    db: &PgPool,
    project_id: Uuid,
    alert_id: Uuid,
    name: &str,
    module: &str,
    metric: &str,
    operator: &str,
    threshold: f64,
    window_minutes: i32,
    cooldown_minutes: i32,
    notify_channels: serde_json::Value,
) -> Result<AlertRule, sqlx::Error> {
    let alert: AlertRule = sqlx::query_as(&format!(
        "UPDATE alert_rules SET name = $1, module = $2, metric = $3, operator = $4, \
         threshold = $5, window_minutes = $6, cooldown_minutes = $7, notify_channels = $8, \
         updated_at = NOW() \
         WHERE id = $9 AND project_id = $10 \
         RETURNING {ALERT_COLUMNS}"
    ))
    .bind(name)
    .bind(module)
    .bind(metric)
    .bind(operator)
    .bind(threshold)
    .bind(window_minutes)
    .bind(cooldown_minutes)
    .bind(&notify_channels)
    .bind(alert_id)
    .bind(project_id)
    .fetch_one(db)
    .await?;

    Ok(alert)
}

/// Delete an alert rule.
pub async fn delete_alert(
    db: &PgPool,
    project_id: Uuid,
    alert_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM alert_rules WHERE id = $1 AND project_id = $2")
        .bind(alert_id)
        .bind(project_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Toggle an alert rule's active status (flip current value).
pub async fn toggle_alert(
    db: &PgPool,
    project_id: Uuid,
    alert_id: Uuid,
) -> Result<AlertRule, sqlx::Error> {
    let alert: AlertRule = sqlx::query_as(&format!(
        "UPDATE alert_rules SET is_active = NOT is_active, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING {ALERT_COLUMNS}"
    ))
    .bind(alert_id)
    .bind(project_id)
    .fetch_one(db)
    .await?;

    Ok(alert)
}

/// Evaluate all active alerts for a project.
/// For each alert, query the metric value over the window, compare against threshold,
/// and mark as triggered if condition met and not in cooldown.
pub async fn evaluate_alerts(state: &SharedState, project_id: Uuid) {
    let alerts = match list_alerts(&state.db, project_id).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Failed to list alerts for evaluation: {e}");
            return;
        }
    };

    let now = Utc::now();

    for alert in alerts {
        if !alert.is_active {
            continue;
        }

        // Check cooldown
        if let Some(last_triggered) = alert.last_triggered_at {
            let cooldown = chrono::Duration::minutes(alert.cooldown_minutes as i64);
            if now - last_triggered < cooldown {
                continue;
            }
        }

        let window_start = now - chrono::Duration::minutes(alert.window_minutes as i64);

        // Fetch metric value
        let metric_value = match fetch_metric_value(
            &state.db,
            project_id,
            &alert.metric,
            window_start,
            now,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch metric '{}' for alert {}: {e}",
                    alert.metric,
                    alert.id
                );
                continue;
            }
        };

        // Compare against threshold
        let triggered = match alert.operator.as_str() {
            "gt" => metric_value > alert.threshold,
            "lt" => metric_value < alert.threshold,
            "gte" => metric_value >= alert.threshold,
            "lte" => metric_value <= alert.threshold,
            "eq" => (metric_value - alert.threshold).abs() < f64::EPSILON,
            _ => false,
        };

        if triggered {
            tracing::info!(
                "Alert '{}' triggered: {} {} {} (value: {})",
                alert.name,
                alert.metric,
                alert.operator,
                alert.threshold,
                metric_value
            );

            // Update last_triggered_at
            let _ = sqlx::query(
                "UPDATE alert_rules SET last_triggered_at = NOW(), updated_at = NOW() WHERE id = $1",
            )
            .bind(alert.id)
            .execute(&state.db)
            .await;

            tracing::info!(
                "Alert notification for '{}': channels={}, metric={}, value={}, threshold={}",
                alert.name,
                alert.notify_channels,
                alert.metric,
                metric_value,
                alert.threshold
            );
        }
    }
}

/// Fetch the current value of a metric over a time window.
async fn fetch_metric_value(
    db: &PgPool,
    project_id: Uuid,
    metric: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<f64, sqlx::Error> {
    match metric {
        "pageviews" => {
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM pageviews WHERE project_id = $1 \
                 AND created_at >= $2 AND created_at <= $3",
            )
            .bind(project_id)
            .bind(start)
            .bind(end)
            .fetch_one(db)
            .await?;
            Ok(row.0 as f64)
        }
        "visitors" => {
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(DISTINCT visitor_id) FROM pageviews WHERE project_id = $1 \
                 AND created_at >= $2 AND created_at <= $3",
            )
            .bind(project_id)
            .bind(start)
            .bind(end)
            .fetch_one(db)
            .await?;
            Ok(row.0 as f64)
        }
        "bounce_rate" => {
            let total: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM sessions WHERE project_id = $1 \
                 AND first_at >= $2 AND first_at <= $3",
            )
            .bind(project_id)
            .bind(start)
            .bind(end)
            .fetch_one(db)
            .await?;

            if total.0 == 0 {
                return Ok(0.0);
            }

            let bounces: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM sessions WHERE project_id = $1 \
                 AND first_at >= $2 AND first_at <= $3 AND is_bounce = true",
            )
            .bind(project_id)
            .bind(start)
            .bind(end)
            .fetch_one(db)
            .await?;

            Ok((bounces.0 as f64 / total.0 as f64) * 100.0)
        }
        "error_count" => {
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM js_errors WHERE project_id = $1 \
                 AND created_at >= $2 AND created_at <= $3",
            )
            .bind(project_id)
            .bind(start)
            .bind(end)
            .fetch_one(db)
            .await?;
            Ok(row.0 as f64)
        }
        "avg_duration" => {
            let row: (Option<f64>,) = sqlx::query_as(
                "SELECT AVG(duration_ms)::double precision FROM sessions \
                 WHERE project_id = $1 AND first_at >= $2 AND first_at <= $3",
            )
            .bind(project_id)
            .bind(start)
            .bind(end)
            .fetch_one(db)
            .await?;
            Ok(row.0.unwrap_or(0.0) / 1000.0)
        }
        _ => Ok(0.0),
    }
}
