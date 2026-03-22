use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ClickPoint {
    pub x: f64,
    pub y: f64,
    pub count: i64,
    pub element_selector: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PageClickStats {
    pub path: String,
    pub total_clicks: i64,
    pub unique_visitors: i64,
}

/// Get click heatmap for a specific page path.
/// Aggregates nearby clicks by rounding coordinates to integer grid points.
pub async fn get_click_heatmap(
    db: &PgPool,
    project_id: Uuid,
    path: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<ClickPoint>, sqlx::Error> {
    let rows: Vec<(f64, f64, i64, Option<String>)> = sqlx::query_as(
        "SELECT ROUND(x)::double precision, ROUND(y)::double precision, \
         COUNT(*)::bigint, \
         (array_agg(element_selector) FILTER (WHERE element_selector IS NOT NULL))[1] \
         FROM click_events WHERE project_id = $1 AND path = $2 \
         AND created_at >= $3 AND created_at <= $4 \
         GROUP BY ROUND(x), ROUND(y) ORDER BY 3 DESC",
    )
    .bind(project_id)
    .bind(path)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| ClickPoint {
            x: r.0,
            y: r.1,
            count: r.2,
            element_selector: r.3,
        })
        .collect();

    Ok(results)
}

/// Get pages with the most clicks.
pub async fn get_click_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<PageClickStats>, sqlx::Error> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT path, COUNT(*)::bigint, COUNT(DISTINCT visitor_id)::bigint \
         FROM click_events WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         GROUP BY path ORDER BY 2 DESC LIMIT $4",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| PageClickStats {
            path: r.0,
            total_clicks: r.1,
            unique_visitors: r.2,
        })
        .collect();

    Ok(results)
}
