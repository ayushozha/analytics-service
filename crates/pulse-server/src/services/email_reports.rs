use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tokio::time;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::query;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailReportConfig {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub recipients: Vec<String>,
    pub schedule: String,
    pub modules: Vec<String>,
    pub is_active: Option<bool>,
    pub last_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const EMAIL_REPORT_COLUMNS: &str = "id, project_id, name, recipients, schedule, modules, \
    is_active, last_sent_at, created_at, updated_at";

pub fn start_email_report_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            time::sleep(std::time::Duration::from_secs(3600)).await;
            if let Err(e) = send_due_reports(&state).await {
                error!("Email report scheduler failed: {e}");
            }
        }
    });
}

pub async fn create_config(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    recipients: &[String],
    schedule: &str,
    modules: &[String],
    is_active: bool,
) -> AppResult<EmailReportConfig> {
    validate_name(name)?;
    validate_schedule(schedule)?;
    validate_recipients(recipients)?;

    let config: EmailReportConfig = sqlx::query_as(&format!(
        "INSERT INTO email_report_configs \
         (project_id, name, recipients, schedule, modules, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {EMAIL_REPORT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name)
    .bind(recipients)
    .bind(schedule)
    .bind(modules)
    .bind(is_active)
    .fetch_one(db)
    .await?;

    Ok(config)
}

pub async fn list_configs(db: &PgPool, project_id: Uuid) -> AppResult<Vec<EmailReportConfig>> {
    let configs: Vec<EmailReportConfig> = sqlx::query_as(&format!(
        "SELECT {EMAIL_REPORT_COLUMNS} FROM email_report_configs \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(configs)
}

pub async fn update_config(
    db: &PgPool,
    project_id: Uuid,
    config_id: Uuid,
    name: &str,
    recipients: &[String],
    schedule: &str,
    modules: &[String],
    is_active: bool,
) -> AppResult<EmailReportConfig> {
    validate_name(name)?;
    validate_schedule(schedule)?;
    validate_recipients(recipients)?;

    let config: Option<EmailReportConfig> = sqlx::query_as(&format!(
        "UPDATE email_report_configs SET \
             name = $1, recipients = $2, schedule = $3, modules = $4, \
             is_active = $5, updated_at = NOW() \
         WHERE id = $6 AND project_id = $7 \
         RETURNING {EMAIL_REPORT_COLUMNS}"
    ))
    .bind(name)
    .bind(recipients)
    .bind(schedule)
    .bind(modules)
    .bind(is_active)
    .bind(config_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    config.ok_or_else(|| AppError::NotFound("Email report config not found".to_string()))
}

pub async fn delete_config(db: &PgPool, project_id: Uuid, config_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM email_report_configs WHERE id = $1 AND project_id = $2")
        .bind(config_id)
        .bind(project_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Email report config not found".to_string(),
        ));
    }

    Ok(())
}

pub async fn send_test_report(
    state: &AppState,
    project_id: Uuid,
    config_id: Uuid,
) -> AppResult<()> {
    let config: Option<EmailReportConfig> = sqlx::query_as(&format!(
        "SELECT {EMAIL_REPORT_COLUMNS} FROM email_report_configs \
         WHERE id = $1 AND project_id = $2"
    ))
    .bind(config_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await?;

    let config =
        config.ok_or_else(|| AppError::NotFound("Email report config not found".to_string()))?;
    send_report(state, &config, false).await
}

async fn send_due_reports(state: &AppState) -> AppResult<()> {
    let configs: Vec<EmailReportConfig> = sqlx::query_as(&format!(
        "SELECT {EMAIL_REPORT_COLUMNS} FROM email_report_configs \
         WHERE COALESCE(is_active, true) = true"
    ))
    .fetch_all(&state.db)
    .await?;

    let now = Utc::now();
    for config in configs {
        if !is_due(&config, now) {
            continue;
        }
        if let Err(e) = send_report(state, &config, true).await {
            warn!("Failed to send email report {}: {e}", config.id);
        }
    }

    Ok(())
}

async fn send_report(
    state: &AppState,
    config: &EmailReportConfig,
    mark_sent: bool,
) -> AppResult<()> {
    let Some(delivery_url) = &state.config.email_report_webhook_url else {
        warn!(
            "EMAIL_REPORT_WEBHOOK_URL is not configured; email report {} not sent",
            config.id
        );
        return Err(AppError::Internal(
            "EMAIL_REPORT_WEBHOOK_URL is not configured".to_string(),
        ));
    };

    let now = Utc::now();
    let (start, end) = report_window(&config.schedule, now);
    let today = now.date_naive();
    let stats = query::fetch_stats(&state.db, config.project_id, start, end, today).await?;
    let events = query::fetch_events_count(&state.db, config.project_id, start, end, today).await?;
    let top_pages =
        query::fetch_pages(&state.db, config.project_id, start, end, today, 5, 0).await?;
    let top_referrers =
        query::fetch_referrers(&state.db, config.project_id, start, end, today, 5, 0).await?;

    let bounce_rate = if stats.2 > 0 {
        (stats.3 as f64 / stats.2 as f64) * 100.0
    } else {
        0.0
    };
    let avg_duration_ms = if stats.2 > 0 { stats.4 / stats.2 } else { 0 };

    let report = serde_json::json!({
        "config_id": config.id,
        "project_id": config.project_id,
        "name": config.name,
        "schedule": config.schedule,
        "period": {
            "start_at": start.to_rfc3339(),
            "end_at": end.to_rfc3339(),
        },
        "metrics": {
            "pageviews": stats.0,
            "visitors": stats.1,
            "sessions": stats.2,
            "bounce_rate": bounce_rate,
            "avg_duration_ms": avg_duration_ms,
            "events": events,
        },
        "top_pages": top_pages,
        "top_referrers": top_referrers,
        "modules": config.modules,
    });

    let subject = format!("Pulse {} report: {}", config.schedule, config.name);
    let text = format!(
        "{subject}\n\nPageviews: {}\nVisitors: {}\nSessions: {}\nEvents: {}\nBounce rate: {:.1}%\nAverage duration: {}ms\n",
        stats.0, stats.1, stats.2, events, bounce_rate, avg_duration_ms
    );
    let html = format!(
        "<h1>{}</h1><ul><li>Pageviews: {}</li><li>Visitors: {}</li><li>Sessions: {}</li><li>Events: {}</li><li>Bounce rate: {:.1}%</li><li>Average duration: {}ms</li></ul>",
        subject, stats.0, stats.1, stats.2, events, bounce_rate, avg_duration_ms
    );

    let payload = serde_json::json!({
        "to": config.recipients,
        "subject": subject,
        "text": text,
        "html": html,
        "report": report,
    });

    let response = reqwest::Client::new()
        .post(delivery_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !response.status().is_success() {
        return Err(AppError::Internal(format!(
            "Email report delivery failed with status {}",
            response.status()
        )));
    }

    if mark_sent {
        sqlx::query(
            "UPDATE email_report_configs SET last_sent_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(config.id)
        .execute(&state.db)
        .await?;
    }

    info!("Sent email report {}", config.id);
    Ok(())
}

fn validate_name(name: &str) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Email report name cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_schedule(schedule: &str) -> AppResult<()> {
    match schedule {
        "daily" | "weekly" | "monthly" => Ok(()),
        other => Err(AppError::BadRequest(format!(
            "Invalid email report schedule: {other}"
        ))),
    }
}

fn validate_recipients(recipients: &[String]) -> AppResult<()> {
    if recipients.is_empty() {
        return Err(AppError::BadRequest(
            "Email report requires at least one recipient".to_string(),
        ));
    }
    for recipient in recipients {
        let trimmed = recipient.trim();
        let Some((local, domain)) = trimmed.split_once('@') else {
            return Err(AppError::BadRequest(format!(
                "Invalid email report recipient: {recipient}"
            )));
        };
        if local.is_empty()
            || domain.is_empty()
            || domain.starts_with('.')
            || domain.ends_with('.')
            || !domain.contains('.')
            || trimmed.chars().any(char::is_whitespace)
        {
            return Err(AppError::BadRequest(format!(
                "Invalid email report recipient: {recipient}"
            )));
        }
    }
    Ok(())
}

fn is_due(config: &EmailReportConfig, now: DateTime<Utc>) -> bool {
    let Some(last_sent_at) = config.last_sent_at else {
        return true;
    };
    let cadence = match config.schedule.as_str() {
        "daily" => Duration::days(1),
        "weekly" => Duration::weeks(1),
        "monthly" => Duration::days(30),
        _ => Duration::weeks(1),
    };
    now - last_sent_at >= cadence
}

fn report_window(schedule: &str, end: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let duration = match schedule {
        "daily" => Duration::days(1),
        "weekly" => Duration::weeks(1),
        "monthly" => Duration::days(30),
        _ => Duration::weeks(1),
    };
    (end - duration, end)
}

#[cfg(test)]
mod tests {
    use super::{validate_name, validate_recipients, validate_schedule};

    #[test]
    fn validates_email_report_inputs() {
        let recipients = vec!["team@example.com".to_string(), "ops@example.co".to_string()];

        assert!(validate_name("Weekly Growth").is_ok());
        assert!(validate_schedule("weekly").is_ok());
        assert!(validate_recipients(&recipients).is_ok());
    }

    #[test]
    fn rejects_invalid_email_report_inputs() {
        assert!(validate_name("   ").is_err());
        assert!(validate_schedule("hourly").is_err());
        assert!(validate_recipients(&[]).is_err());
        assert!(validate_recipients(&["not-an-email".to_string()]).is_err());
        assert!(validate_recipients(&["team@example".to_string()]).is_err());
        assert!(validate_recipients(&["bad address@example.com".to_string()]).is_err());
    }
}
