use std::sync::Arc;

use crate::models::webhook::Webhook;
use crate::state::AppState;
use chrono::{Datelike, Duration, NaiveDate, Timelike, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info};

type HmacSha256 = Hmac<Sha256>;

/// Start all webhook-related background tasks.
pub fn start_webhook_tasks(state: Arc<AppState>) {
    // Traffic spike checker — every 10 minutes
    let s = state.clone();
    tokio::spawn(async move {
        loop {
            time::sleep(std::time::Duration::from_secs(600)).await;
            if let Err(e) = check_traffic_spikes(&s).await {
                error!("Traffic spike check failed: {e}");
            }
        }
    });

    // Zero traffic checker — every hour
    let s = state.clone();
    tokio::spawn(async move {
        loop {
            time::sleep(std::time::Duration::from_secs(3600)).await;
            if let Err(e) = check_zero_traffic(&s).await {
                error!("Zero traffic check failed: {e}");
            }
        }
    });

    // Daily summary + baseline update — at 00:30 UTC
    let s = state.clone();
    tokio::spawn(async move {
        loop {
            let now = Utc::now();
            let tomorrow = (now + Duration::days(1)).date_naive();
            let next_run = tomorrow
                .and_hms_opt(0, 30, 0)
                .expect("valid time")
                .and_utc();
            let sleep_duration = (next_run - now)
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(3600));

            time::sleep(sleep_duration).await;

            let yesterday = (Utc::now() - Duration::days(1)).date_naive();
            if let Err(e) = send_daily_summaries(&s, yesterday).await {
                error!("Daily summary dispatch failed: {e}");
            }
            if let Err(e) = update_baselines(&s.db).await {
                error!("Baseline update failed: {e}");
            }
        }
    });
}

async fn check_traffic_spikes(state: &AppState) -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let hour = now.hour() as i16;
    let dow = now.weekday().num_days_from_monday() as i16;
    let ten_min_ago = now - Duration::minutes(10);

    // Get projects that have traffic_spike webhooks
    let webhooks: Vec<Webhook> = sqlx::query_as(
        "SELECT id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at \
         FROM webhooks WHERE is_active = true AND 'traffic_spike' = ANY(events)",
    )
    .fetch_all(&state.db)
    .await?;

    for webhook in &webhooks {
        // Get baseline for this hour/dow
        let baseline: Option<(f64,)> = sqlx::query_as(
            "SELECT avg_pageviews FROM webhook_baselines \
             WHERE project_id = $1 AND hour_of_day = $2 AND day_of_week = $3",
        )
        .bind(webhook.project_id)
        .bind(hour)
        .bind(dow)
        .fetch_optional(&state.db)
        .await?;

        let avg = baseline.map(|(v,)| v).unwrap_or(0.0);
        if avg < 1.0 {
            continue; // No meaningful baseline yet
        }

        // Count pageviews in last 10 minutes, extrapolate to hourly
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pageviews \
             WHERE project_id = $1 AND created_at >= $2",
        )
        .bind(webhook.project_id)
        .bind(ten_min_ago)
        .fetch_one(&state.db)
        .await?;

        let hourly_rate = count.0 as f64 * 6.0; // 10min × 6 = 1 hour
        let multiplier = hourly_rate / avg;

        if multiplier >= 2.0 {
            let payload = serde_json::json!({
                "event": "traffic_spike",
                "project_id": webhook.project_id,
                "timestamp": now.to_rfc3339(),
                "data": {
                    "current_rate": hourly_rate as i64,
                    "baseline_rate": avg as i64,
                    "multiplier": (multiplier * 10.0).round() / 10.0,
                }
            });
            dispatch_webhook(webhook, &payload).await;
        }
    }

    Ok(())
}

async fn check_zero_traffic(state: &AppState) -> Result<(), anyhow::Error> {
    let three_hours_ago = Utc::now() - Duration::hours(3);

    let webhooks: Vec<Webhook> = sqlx::query_as(
        "SELECT id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at \
         FROM webhooks WHERE is_active = true AND 'zero_traffic' = ANY(events)",
    )
    .fetch_all(&state.db)
    .await?;

    for webhook in &webhooks {
        // Don't fire more than once per 6 hours
        if let Some(last) = webhook.last_triggered_at {
            if Utc::now() - last < Duration::hours(6) {
                continue;
            }
        }

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pageviews \
             WHERE project_id = $1 AND created_at >= $2",
        )
        .bind(webhook.project_id)
        .bind(three_hours_ago)
        .fetch_one(&state.db)
        .await?;

        if count.0 == 0 {
            let payload = serde_json::json!({
                "event": "zero_traffic",
                "project_id": webhook.project_id,
                "timestamp": Utc::now().to_rfc3339(),
                "data": {
                    "hours_without_traffic": 3,
                }
            });
            dispatch_webhook(webhook, &payload).await;

            // Update last_triggered_at
            let _ = sqlx::query("UPDATE webhooks SET last_triggered_at = NOW() WHERE id = $1")
                .bind(webhook.id)
                .execute(&state.db)
                .await;
        }
    }

    Ok(())
}

async fn send_daily_summaries(state: &AppState, date: NaiveDate) -> Result<(), anyhow::Error> {
    let webhooks: Vec<Webhook> = sqlx::query_as(
        "SELECT id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at \
         FROM webhooks WHERE is_active = true AND 'daily_summary' = ANY(events)",
    )
    .fetch_all(&state.db)
    .await?;

    for webhook in &webhooks {
        let stats: Option<(i64, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT pageviews, visitors, sessions, bounces, total_duration_ms \
             FROM daily_stats WHERE project_id = $1 AND date = $2",
        )
        .bind(webhook.project_id)
        .bind(date)
        .fetch_optional(&state.db)
        .await?;

        let (pv, vis, sess, bounces, dur) = stats.unwrap_or((0, 0, 0, 0, 0));
        let bounce_rate = if sess > 0 {
            (bounces as f64 / sess as f64 * 100.0).round()
        } else {
            0.0
        };
        let avg_duration = if sess > 0 { dur / sess } else { 0 };

        let payload = serde_json::json!({
            "event": "daily_summary",
            "project_id": webhook.project_id,
            "timestamp": Utc::now().to_rfc3339(),
            "data": {
                "date": date.to_string(),
                "pageviews": pv,
                "visitors": vis,
                "sessions": sess,
                "bounce_rate": bounce_rate,
                "avg_duration_ms": avg_duration,
            }
        });
        dispatch_webhook(webhook, &payload).await;
    }

    Ok(())
}

async fn update_baselines(db: &PgPool) -> Result<(), anyhow::Error> {
    // Rolling 14-day average by hour and day-of-week
    sqlx::query(
        r#"INSERT INTO webhook_baselines (project_id, hour_of_day, day_of_week, avg_pageviews, updated_at)
        SELECT
            project_id,
            EXTRACT(HOUR FROM created_at)::smallint,
            EXTRACT(ISODOW FROM created_at)::smallint - 1,
            COUNT(*)::double precision / 14.0
        FROM pageviews
        WHERE created_at >= NOW() - interval '14 days'
        GROUP BY project_id, EXTRACT(HOUR FROM created_at), EXTRACT(ISODOW FROM created_at)
        ON CONFLICT (project_id, hour_of_day, day_of_week) DO UPDATE SET
            avg_pageviews = EXCLUDED.avg_pageviews,
            updated_at = NOW()"#,
    )
    .execute(db)
    .await?;

    info!("Updated webhook baselines");
    Ok(())
}

async fn dispatch_webhook(webhook: &Webhook, payload: &serde_json::Value) {
    let body = serde_json::to_string(payload).unwrap();
    let mut req = reqwest::Client::new()
        .post(&webhook.url)
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(5));

    if let Some(secret) = &webhook.secret {
        if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
            mac.update(body.as_bytes());
            let signature = hex::encode(mac.finalize().into_bytes());
            req = req.header("X-Pulse-Signature", signature);
        }
    }

    match req.body(body).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                error!("Webhook {} returned status {}", webhook.id, resp.status());
            }
        }
        Err(e) => {
            error!("Webhook {} dispatch failed: {e}", webhook.id);
        }
    }
}
