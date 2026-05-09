use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Goal {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub goal_type: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalStats {
    pub conversions: i64,
    pub unique_visitors: i64,
    pub total_revenue: f64,
    pub conversion_rate: f64,
}

pub async fn create_goal(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    goal_type: &str,
    config: serde_json::Value,
) -> AppResult<Goal> {
    // Validate goal type
    match goal_type {
        "pageview" | "event" | "duration" | "pages_per_session" => {}
        other => {
            return Err(AppError::BadRequest(format!(
                "Invalid goal type: {other}. Must be one of: pageview, event, duration, pages_per_session"
            )));
        }
    }

    let goal: Goal = sqlx::query_as(
        "INSERT INTO goals (project_id, name, goal_type, config) VALUES ($1, $2, $3, $4) \
         RETURNING id, project_id, name, goal_type, config, created_at, updated_at",
    )
    .bind(project_id)
    .bind(name)
    .bind(goal_type)
    .bind(&config)
    .fetch_one(db)
    .await?;

    Ok(goal)
}

pub async fn list_goals(db: &PgPool, project_id: Uuid) -> AppResult<Vec<Goal>> {
    let goals: Vec<Goal> = sqlx::query_as(
        "SELECT id, project_id, name, goal_type, config, created_at, updated_at \
         FROM goals WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(goals)
}

pub async fn get_goal(db: &PgPool, project_id: Uuid, goal_id: Uuid) -> AppResult<Option<Goal>> {
    let goal: Option<Goal> = sqlx::query_as(
        "SELECT id, project_id, name, goal_type, config, created_at, updated_at \
         FROM goals WHERE id = $1 AND project_id = $2",
    )
    .bind(goal_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    Ok(goal)
}

pub async fn update_goal(
    db: &PgPool,
    project_id: Uuid,
    goal_id: Uuid,
    name: &str,
    goal_type: &str,
    config: serde_json::Value,
) -> AppResult<Goal> {
    match goal_type {
        "pageview" | "event" | "duration" | "pages_per_session" => {}
        other => {
            return Err(AppError::BadRequest(format!("Invalid goal type: {other}")));
        }
    }

    let goal: Option<Goal> = sqlx::query_as(
        "UPDATE goals SET name = $3, goal_type = $4, config = $5, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, goal_type, config, created_at, updated_at",
    )
    .bind(goal_id)
    .bind(project_id)
    .bind(name)
    .bind(goal_type)
    .bind(&config)
    .fetch_optional(db)
    .await?;

    goal.ok_or_else(|| AppError::NotFound("Goal not found".to_string()))
}

pub async fn delete_goal(db: &PgPool, project_id: Uuid, goal_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM goals WHERE id = $1 AND project_id = $2")
        .bind(goal_id)
        .bind(project_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Goal not found".to_string()));
    }
    Ok(())
}

pub async fn record_conversion(
    db: &PgPool,
    project_id: Uuid,
    goal_id: Uuid,
    visitor_id: &str,
    session_id: Uuid,
    revenue_amount: Option<f64>,
    revenue_currency: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO goal_conversions (project_id, goal_id, visitor_id, session_id, revenue_amount, revenue_currency) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(project_id)
    .bind(goal_id)
    .bind(visitor_id)
    .bind(session_id)
    .bind(revenue_amount)
    .bind(revenue_currency.unwrap_or("USD"))
    .execute(db)
    .await?;

    Ok(())
}

pub async fn get_goal_stats(
    db: &PgPool,
    project_id: Uuid,
    goal_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<GoalStats> {
    // Get conversion counts and revenue
    let stats: (i64, i64, f64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, \
         COUNT(DISTINCT visitor_id)::bigint, \
         COALESCE(SUM(revenue_amount), 0)::float8 \
         FROM goal_conversions \
         WHERE project_id = $1 AND goal_id = $2 \
         AND created_at >= $3 AND created_at <= $4",
    )
    .bind(project_id)
    .bind(goal_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    // Get total unique visitors in the period for conversion rate
    let total_visitors: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT visitor_id)::bigint FROM sessions \
         WHERE project_id = $1 AND first_at >= $2 AND first_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    let conversion_rate = if total_visitors.0 > 0 {
        (stats.0 as f64 / total_visitors.0 as f64) * 100.0
    } else {
        0.0
    };

    Ok(GoalStats {
        conversions: stats.0,
        unique_visitors: stats.1,
        total_revenue: stats.2,
        conversion_rate,
    })
}

pub async fn evaluate_pageview_goals(
    db: &PgPool,
    project_id: Uuid,
    path: &str,
    visitor_id: &str,
    session_id: Uuid,
) -> AppResult<()> {
    // Fetch all active pageview goals for this project
    let goals: Vec<Goal> = sqlx::query_as(
        "SELECT id, project_id, name, goal_type, config, created_at, updated_at \
         FROM goals WHERE project_id = $1 AND goal_type = 'pageview'",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    for goal in &goals {
        let url_pattern = goal
            .config
            .get("url_pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if matches_pattern(path, url_pattern) {
            // Check if already converted in this session to avoid duplicates
            let existing: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM goal_conversions \
                 WHERE goal_id = $1 AND session_id = $2",
            )
            .bind(goal.id)
            .bind(session_id)
            .fetch_one(db)
            .await?;

            if existing.0 == 0 {
                record_conversion(db, project_id, goal.id, visitor_id, session_id, None, None)
                    .await?;
            }
        }
    }

    Ok(())
}

pub async fn evaluate_event_goals(
    db: &PgPool,
    project_id: Uuid,
    event_name: &str,
    event_data: Option<&serde_json::Value>,
    visitor_id: &str,
    session_id: Uuid,
    revenue_amount: Option<f64>,
) -> AppResult<()> {
    // Fetch all active event goals for this project
    let goals: Vec<Goal> = sqlx::query_as(
        "SELECT id, project_id, name, goal_type, config, created_at, updated_at \
         FROM goals WHERE project_id = $1 AND goal_type = 'event'",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    for goal in &goals {
        let target_event = goal
            .config
            .get("event_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if event_name != target_event {
            continue;
        }

        // Check event_data_match if configured
        let data_match = goal.config.get("event_data_match");
        if let Some(match_obj) = data_match {
            if let Some(match_map) = match_obj.as_object() {
                if !match_map.is_empty() {
                    let matches = if let Some(actual_data) = event_data {
                        match_map.iter().all(|(k, v)| actual_data.get(k) == Some(v))
                    } else {
                        false
                    };
                    if !matches {
                        continue;
                    }
                }
            }
        }

        // Check for duplicate conversion in this session
        let existing: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM goal_conversions \
             WHERE goal_id = $1 AND session_id = $2",
        )
        .bind(goal.id)
        .bind(session_id)
        .fetch_one(db)
        .await?;

        if existing.0 == 0 {
            record_conversion(
                db,
                project_id,
                goal.id,
                visitor_id,
                session_id,
                revenue_amount,
                None,
            )
            .await?;
        }
    }

    Ok(())
}

/// Simple pattern matching for URL patterns.
/// Supports SQL LIKE-style patterns with % as wildcard.
fn matches_pattern(path: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    // Exact match
    if !pattern.contains('%') {
        return path == pattern;
    }
    // Convert SQL LIKE pattern to a simple match
    let parts: Vec<&str> = pattern.split('%').collect();
    if parts.len() == 2 {
        let prefix = parts[0];
        let suffix = parts[1];
        if prefix.is_empty() && suffix.is_empty() {
            return true; // "%" matches everything
        }
        if prefix.is_empty() {
            return path.ends_with(suffix);
        }
        if suffix.is_empty() {
            return path.starts_with(prefix);
        }
        return path.starts_with(prefix) && path.ends_with(suffix);
    }
    // For more complex patterns, do a simple contains check on the non-empty parts
    parts
        .iter()
        .filter(|p| !p.is_empty())
        .all(|p| path.contains(p))
}
