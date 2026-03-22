use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info};

use crate::state::AppState;

/// Start the daily rollup background task.
/// Runs at startup for yesterday (in case it was missed), then schedules
/// daily at 00:05 UTC.
pub fn start_rollup_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        // Run immediately for yesterday
        let yesterday = (Utc::now() - Duration::days(1)).date_naive();
        info!("Running initial rollup for {yesterday}");
        if let Err(e) = compute_all_rollups(&state.db, yesterday).await {
            error!("Initial rollup failed: {e}");
        }

        // Schedule daily
        loop {
            let now = Utc::now();
            // Next run at 00:05 UTC tomorrow
            let tomorrow = (now + Duration::days(1)).date_naive();
            let next_run = tomorrow
                .and_hms_opt(0, 5, 0)
                .expect("valid time")
                .and_utc();
            let sleep_duration = (next_run - now).to_std().unwrap_or(std::time::Duration::from_secs(3600));

            info!("Next rollup scheduled at {next_run} (sleeping {}s)", sleep_duration.as_secs());
            time::sleep(sleep_duration).await;

            let target_date = (Utc::now() - Duration::days(1)).date_naive();
            info!("Running daily rollup for {target_date}");
            if let Err(e) = compute_all_rollups(&state.db, target_date).await {
                error!("Daily rollup failed for {target_date}: {e}");
            }
        }
    });
}

async fn compute_all_rollups(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    compute_daily_stats(db, date).await?;
    compute_daily_pages(db, date).await?;
    compute_daily_referrers(db, date).await?;
    compute_daily_events(db, date).await?;
    compute_daily_geo(db, date).await?;
    compute_daily_devices(db, date).await?;
    compute_daily_campaigns(db, date).await?;
    info!("Rollup complete for {date}");
    Ok(())
}

async fn compute_daily_stats(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"INSERT INTO daily_stats (project_id, date, pageviews, visitors, sessions, bounces, total_duration_ms)
        SELECT
            p.project_id,
            $1::date,
            COUNT(p.id),
            COUNT(DISTINCT p.visitor_id),
            COUNT(DISTINCT p.session_id),
            (SELECT COUNT(*) FROM sessions s
             WHERE s.project_id = p.project_id
             AND s.first_at::date = $1::date
             AND s.is_bounce = true),
            COALESCE((SELECT SUM(s2.duration_ms) FROM sessions s2
             WHERE s2.project_id = p.project_id
             AND s2.first_at::date = $1::date), 0)
        FROM pageviews p
        WHERE p.created_at >= $1::date AND p.created_at < ($1::date + interval '1 day')
        GROUP BY p.project_id
        ON CONFLICT (project_id, date) DO UPDATE SET
            pageviews = EXCLUDED.pageviews,
            visitors = EXCLUDED.visitors,
            sessions = EXCLUDED.sessions,
            bounces = EXCLUDED.bounces,
            total_duration_ms = EXCLUDED.total_duration_ms"#,
    )
    .bind(date)
    .execute(db)
    .await?;
    Ok(())
}

async fn compute_daily_pages(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    // Delete existing rows for this date first (composite PK includes path)
    sqlx::query("DELETE FROM daily_pages WHERE date = $1")
        .bind(date)
        .execute(db)
        .await?;

    sqlx::query(
        r#"INSERT INTO daily_pages (project_id, date, path, views, unique_views, avg_duration_ms)
        SELECT
            project_id,
            $1::date,
            path,
            COUNT(*),
            COUNT(DISTINCT visitor_id),
            COALESCE(AVG(duration_ms), 0)::int
        FROM pageviews
        WHERE created_at >= $1::date AND created_at < ($1::date + interval '1 day')
        GROUP BY project_id, path"#,
    )
    .bind(date)
    .execute(db)
    .await?;
    Ok(())
}

async fn compute_daily_referrers(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    sqlx::query("DELETE FROM daily_referrers WHERE date = $1")
        .bind(date)
        .execute(db)
        .await?;

    sqlx::query(
        r#"INSERT INTO daily_referrers (project_id, date, referrer_domain, visits)
        SELECT
            project_id,
            $1::date,
            COALESCE(referrer_domain, 'Direct'),
            COUNT(DISTINCT session_id)
        FROM pageviews
        WHERE created_at >= $1::date AND created_at < ($1::date + interval '1 day')
        GROUP BY project_id, referrer_domain"#,
    )
    .bind(date)
    .execute(db)
    .await?;
    Ok(())
}

async fn compute_daily_events(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    sqlx::query("DELETE FROM daily_events WHERE date = $1")
        .bind(date)
        .execute(db)
        .await?;

    sqlx::query(
        r#"INSERT INTO daily_events (project_id, date, event_name, count)
        SELECT
            project_id,
            $1::date,
            event_name,
            COUNT(*)
        FROM events
        WHERE created_at >= $1::date AND created_at < ($1::date + interval '1 day')
        GROUP BY project_id, event_name"#,
    )
    .bind(date)
    .execute(db)
    .await?;
    Ok(())
}

async fn compute_daily_geo(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    sqlx::query("DELETE FROM daily_geo WHERE date = $1")
        .bind(date)
        .execute(db)
        .await?;

    sqlx::query(
        r#"INSERT INTO daily_geo (project_id, date, country, visitors)
        SELECT
            project_id,
            $1::date,
            COALESCE(country, 'XX'),
            COUNT(DISTINCT visitor_id)
        FROM sessions
        WHERE first_at >= $1::date AND first_at < ($1::date + interval '1 day')
        GROUP BY project_id, country"#,
    )
    .bind(date)
    .execute(db)
    .await?;
    Ok(())
}

async fn compute_daily_devices(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    sqlx::query("DELETE FROM daily_devices WHERE date = $1")
        .bind(date)
        .execute(db)
        .await?;

    sqlx::query(
        r#"INSERT INTO daily_devices (project_id, date, browser, os, device, visitors)
        SELECT
            project_id,
            $1::date,
            COALESCE(browser, 'Unknown'),
            COALESCE(os, 'Unknown'),
            COALESCE(device, 'desktop'),
            COUNT(DISTINCT visitor_id)
        FROM sessions
        WHERE first_at >= $1::date AND first_at < ($1::date + interval '1 day')
        GROUP BY project_id, browser, os, device"#,
    )
    .bind(date)
    .execute(db)
    .await?;
    Ok(())
}

async fn compute_daily_campaigns(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    sqlx::query("DELETE FROM daily_campaigns WHERE date = $1")
        .bind(date)
        .execute(db)
        .await?;

    sqlx::query(
        r#"INSERT INTO daily_campaigns (project_id, date, utm_source, utm_medium, utm_campaign, visitors, sessions, pageviews, bounces)
        SELECT
            p.project_id,
            $1::date,
            COALESCE(p.utm_source, ''),
            COALESCE(p.utm_medium, ''),
            COALESCE(p.utm_campaign, ''),
            COUNT(DISTINCT p.visitor_id),
            COUNT(DISTINCT p.session_id),
            COUNT(p.id),
            (SELECT COUNT(*) FROM sessions s
             WHERE s.project_id = p.project_id
             AND s.first_at::date = $1::date
             AND s.is_bounce = true
             AND s.id IN (
                SELECT DISTINCT pv.session_id FROM pageviews pv
                WHERE pv.project_id = p.project_id
                AND pv.created_at >= $1::date AND pv.created_at < ($1::date + interval '1 day')
                AND pv.utm_source = p.utm_source
             ))
        FROM pageviews p
        WHERE p.created_at >= $1::date AND p.created_at < ($1::date + interval '1 day')
          AND p.utm_source IS NOT NULL
        GROUP BY p.project_id, p.utm_source, p.utm_medium, p.utm_campaign"#,
    )
    .bind(date)
    .execute(db)
    .await?;
    Ok(())
}

/// Manually trigger rollup for a specific date (used by admin endpoint).
pub async fn trigger_rollup(db: &PgPool, date: NaiveDate) -> Result<(), anyhow::Error> {
    compute_all_rollups(db, date).await
}
