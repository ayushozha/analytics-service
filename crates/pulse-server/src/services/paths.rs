use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageFlow {
    pub from_path: String,
    pub to_path: String,
    pub transitions: i64,
    pub percentage: f64,
}

pub async fn get_page_flows(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    path: &str,
    direction: &str,
    limit: i64,
) -> AppResult<Vec<PageFlow>> {
    match direction {
        "forward" | "backward" => {}
        other => {
            return Err(AppError::BadRequest(format!(
                "Invalid direction: {other}. Must be 'forward' or 'backward'"
            )));
        }
    }

    let flows = if direction == "forward" {
        // Find the next page users visit after the target path
        // Uses a window function to get the next row in the same session
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            r#"WITH ordered_pv AS (
                SELECT
                    session_id,
                    path,
                    LEAD(path) OVER (PARTITION BY session_id ORDER BY created_at) AS next_path
                FROM pageviews
                WHERE project_id = $1
                AND created_at >= $2
                AND created_at <= $3
            )
            SELECT
                path AS from_path,
                next_path AS to_path,
                COUNT(*)::bigint AS transitions
            FROM ordered_pv
            WHERE path = $4 AND next_path IS NOT NULL
            GROUP BY path, next_path
            ORDER BY transitions DESC
            LIMIT $5"#,
        )
        .bind(project_id)
        .bind(start)
        .bind(end)
        .bind(path)
        .bind(limit)
        .fetch_all(db)
        .await?;

        rows
    } else {
        // Find the previous page users visited before the target path
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            r#"WITH ordered_pv AS (
                SELECT
                    session_id,
                    path,
                    LAG(path) OVER (PARTITION BY session_id ORDER BY created_at) AS prev_path
                FROM pageviews
                WHERE project_id = $1
                AND created_at >= $2
                AND created_at <= $3
            )
            SELECT
                prev_path AS from_path,
                path AS to_path,
                COUNT(*)::bigint AS transitions
            FROM ordered_pv
            WHERE path = $4 AND prev_path IS NOT NULL
            GROUP BY prev_path, path
            ORDER BY transitions DESC
            LIMIT $5"#,
        )
        .bind(project_id)
        .bind(start)
        .bind(end)
        .bind(path)
        .bind(limit)
        .fetch_all(db)
        .await?;

        rows
    };

    // Calculate total transitions for percentage
    let total: i64 = flows.iter().map(|f| f.2).sum();

    let page_flows = flows
        .into_iter()
        .map(|(from_path, to_path, transitions)| {
            let percentage = if total > 0 {
                (transitions as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            PageFlow {
                from_path,
                to_path,
                transitions,
                percentage,
            }
        })
        .collect();

    Ok(page_flows)
}
