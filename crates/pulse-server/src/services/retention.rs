use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info};

use crate::services::partition::get_partition_names;
use crate::state::AppState;

/// Start the data retention background task.
/// Runs daily at 01:00 UTC (after rollup at 00:05) to clean old data.
/// Disabled when data_retention_days is 0.
pub fn start_retention_task(state: Arc<AppState>) {
    let retention_days = state.config.data_retention_days;
    if retention_days == 0 {
        info!("Data retention disabled (DATA_RETENTION_DAYS=0)");
        return;
    }

    info!("Data retention enabled: {retention_days} days");

    tokio::spawn(async move {
        loop {
            let now = Utc::now();
            let tomorrow = (now + Duration::days(1)).date_naive();
            let next_run = tomorrow
                .and_hms_opt(1, 0, 0)
                .expect("valid time")
                .and_utc();
            let sleep_duration = (next_run - now)
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(3600));

            info!(
                "Next retention cleanup at {next_run} (sleeping {}s)",
                sleep_duration.as_secs()
            );
            time::sleep(sleep_duration).await;

            let cutoff = (Utc::now() - Duration::days(retention_days as i64)).date_naive();
            if let Err(e) = run_retention(&state.db, cutoff).await {
                error!("Retention cleanup failed: {e}");
            }
        }
    });
}

async fn run_retention(db: &PgPool, cutoff: NaiveDate) -> Result<(), anyhow::Error> {
    info!("Running retention cleanup, cutoff date: {cutoff}");

    let partitioned_tables = [
        "pageviews",
        "events",
        "goal_conversions",
        "web_vitals",
        "scroll_depths",
        "search_queries",
        "outlinks",
        "js_errors",
        "click_events",
        "experiment_assignments",
    ];
    for parent_table in partitioned_tables {
        let partitions = get_partition_names(db, parent_table).await?;

        for (name, _start, end) in &partitions {
            if *end <= cutoff {
                // Entire partition is before cutoff — drop it
                let sql = format!("DROP TABLE IF EXISTS {name}");
                sqlx::query(&sql).execute(db).await?;
                info!("Dropped expired partition {name}");
            }
        }
    }

    // Clean old sessions
    let result = sqlx::query("DELETE FROM sessions WHERE first_at < $1::date")
        .bind(cutoff)
        .execute(db)
        .await?;
    let deleted = result.rows_affected();
    if deleted > 0 {
        info!("Cleaned {deleted} expired sessions");
    }

    info!("Retention cleanup complete");
    Ok(())
}
