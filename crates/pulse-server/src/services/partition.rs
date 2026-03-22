use std::sync::Arc;

use chrono::{Datelike, Months, NaiveDate, Utc};
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info};

use crate::state::AppState;

/// Start the partition management background task.
/// Ensures partitions exist for the current month through 3 months ahead.
/// Runs at startup, then on the 1st of each month at 01:00 UTC.
pub fn start_partition_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        if let Err(e) = ensure_partitions(&state.db).await {
            error!("Initial partition check failed: {e}");
        }

        loop {
            let now = Utc::now();
            let next_month = first_of_next_month(now.date_naive());
            let next_run = next_month
                .and_hms_opt(1, 0, 0)
                .expect("valid time")
                .and_utc();
            let sleep_duration = (next_run - now)
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(86400));

            info!(
                "Next partition check at {next_run} (sleeping {}s)",
                sleep_duration.as_secs()
            );
            time::sleep(sleep_duration).await;

            if let Err(e) = ensure_partitions(&state.db).await {
                error!("Partition management failed: {e}");
            }
        }
    });
}

async fn ensure_partitions(db: &PgPool) -> Result<(), anyhow::Error> {
    let now = Utc::now().date_naive();
    let target = now + Months::new(3);

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
        let existing = get_existing_partition_months(db, parent_table).await?;
        let mut month = first_of_month(now);

        while month <= first_of_month(target) {
            if !existing.contains(&month) {
                create_partition(db, parent_table, month).await?;
            }
            month = month + Months::new(1);
        }
    }

    info!("Partition check complete — partitions exist through {target}");
    Ok(())
}

async fn get_existing_partition_months(
    db: &PgPool,
    parent_table: &str,
) -> Result<Vec<NaiveDate>, anyhow::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT c.relname::text
        FROM pg_inherits i
        JOIN pg_class c ON c.oid = i.inhrelid
        JOIN pg_class p ON p.oid = i.inhparent
        WHERE p.relname = $1
        ORDER BY c.relname"#,
    )
    .bind(parent_table)
    .fetch_all(db)
    .await?;

    let mut months = Vec::new();
    let prefix = format!("{}_", parent_table);
    for (name,) in rows {
        if let Some(suffix) = name.strip_prefix(&prefix) {
            // suffix is like "2026_01"
            let parts: Vec<&str> = suffix.split('_').collect();
            if parts.len() == 2 {
                if let (Ok(year), Ok(month)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>()) {
                    if let Some(date) = NaiveDate::from_ymd_opt(year, month, 1) {
                        months.push(date);
                    }
                }
            }
        }
    }

    Ok(months)
}

async fn create_partition(
    db: &PgPool,
    parent_table: &str,
    month_start: NaiveDate,
) -> Result<(), anyhow::Error> {
    let partition_name = format!(
        "{}_{:04}_{:02}",
        parent_table,
        month_start.year(),
        month_start.month()
    );
    let next_month = month_start + Months::new(1);

    let sql = format!(
        "CREATE TABLE IF NOT EXISTS {partition_name} PARTITION OF {parent_table} \
         FOR VALUES FROM ('{month_start}') TO ('{next_month}')"
    );

    sqlx::query(&sql).execute(db).await?;
    info!("Created partition {partition_name}");
    Ok(())
}

/// Get all existing partition table names for a parent table.
pub async fn get_partition_names(
    db: &PgPool,
    parent_table: &str,
) -> Result<Vec<(String, NaiveDate, NaiveDate)>, anyhow::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT c.relname::text
        FROM pg_inherits i
        JOIN pg_class c ON c.oid = i.inhrelid
        JOIN pg_class p ON p.oid = i.inhparent
        WHERE p.relname = $1
        ORDER BY c.relname"#,
    )
    .bind(parent_table)
    .fetch_all(db)
    .await?;

    let prefix = format!("{}_", parent_table);
    let mut result = Vec::new();
    for (name,) in rows {
        if let Some(suffix) = name.strip_prefix(&prefix) {
            let parts: Vec<&str> = suffix.split('_').collect();
            if parts.len() == 2 {
                if let (Ok(year), Ok(month)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>()) {
                    if let Some(start) = NaiveDate::from_ymd_opt(year, month, 1) {
                        let end = start + Months::new(1);
                        result.push((name.clone(), start, end));
                    }
                }
            }
        }
    }

    Ok(result)
}

fn first_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap()
}

fn first_of_next_month(date: NaiveDate) -> NaiveDate {
    first_of_month(date) + Months::new(1)
}
