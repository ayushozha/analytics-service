use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ErrorGroup {
    pub message: String,
    pub count: i64,
    pub affected_visitors: i64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub last_path: Option<String>,
    pub last_browser: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorInstance {
    pub id: i64,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub message: String,
    pub stack: Option<String>,
    pub filename: Option<String>,
    pub lineno: Option<i32>,
    pub colno: Option<i32>,
    pub path: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ErrorTimeseriesPoint {
    pub date: NaiveDate,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ErrorStats {
    pub total_errors: i64,
    pub unique_errors: i64,
    pub affected_visitors: i64,
}

/// Get error groups: errors grouped by message with count and first/last seen.
pub async fn get_error_groups(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ErrorGroup>, sqlx::Error> {
    let rows: Vec<(String, i64, i64, DateTime<Utc>, DateTime<Utc>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT message, COUNT(*)::bigint, COUNT(DISTINCT visitor_id)::bigint, \
             MIN(created_at), MAX(created_at), \
             (array_agg(path ORDER BY created_at DESC))[1], \
             (array_agg(browser ORDER BY created_at DESC))[1] \
             FROM js_errors WHERE project_id = $1 \
             AND created_at >= $2 AND created_at <= $3 \
             GROUP BY message ORDER BY 2 DESC LIMIT $4 OFFSET $5",
        )
        .bind(project_id)
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;

    let results = rows
        .into_iter()
        .map(|r| ErrorGroup {
            message: r.0,
            count: r.1,
            affected_visitors: r.2,
            first_seen: r.3,
            last_seen: r.4,
            last_path: r.5,
            last_browser: r.6,
        })
        .collect();

    Ok(results)
}

/// Get individual error instances for a specific error message.
pub async fn get_error_detail(
    db: &PgPool,
    project_id: Uuid,
    message: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<ErrorInstance>, sqlx::Error> {
    let rows: Vec<(i64, String, Uuid, String, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<String>, Option<String>, Option<String>, DateTime<Utc>)> =
        sqlx::query_as(
            "SELECT id, visitor_id, session_id, message, stack, filename, lineno, colno, \
             path, browser, os, created_at \
             FROM js_errors WHERE project_id = $1 AND message = $2 \
             AND created_at >= $3 AND created_at <= $4 \
             ORDER BY created_at DESC LIMIT $5",
        )
        .bind(project_id)
        .bind(message)
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(db)
        .await?;

    let results = rows
        .into_iter()
        .map(|r| ErrorInstance {
            id: r.0,
            visitor_id: r.1,
            session_id: r.2,
            message: r.3,
            stack: r.4,
            filename: r.5,
            lineno: r.6,
            colno: r.7,
            path: r.8,
            browser: r.9,
            os: r.10,
            created_at: r.11,
        })
        .collect();

    Ok(results)
}

/// Get daily error count timeseries.
pub async fn get_error_timeseries(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<ErrorTimeseriesPoint>, sqlx::Error> {
    let rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        "SELECT created_at::date as day, COUNT(*)::bigint \
         FROM js_errors WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         GROUP BY day ORDER BY day",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|(date, count)| ErrorTimeseriesPoint { date, count })
        .collect();

    Ok(results)
}

/// Get aggregate error statistics.
pub async fn get_error_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<ErrorStats, sqlx::Error> {
    let row: (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, COUNT(DISTINCT message)::bigint, \
         COUNT(DISTINCT visitor_id)::bigint \
         FROM js_errors WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    Ok(ErrorStats {
        total_errors: row.0,
        unique_errors: row.1,
        affected_visitors: row.2,
    })
}
