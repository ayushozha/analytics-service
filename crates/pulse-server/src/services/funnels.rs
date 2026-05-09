use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Funnel {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub steps: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStep {
    pub label: String,
    pub visitors: i64,
    pub dropoff_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDef {
    #[serde(rename = "type")]
    pub step_type: String,
    pub value: String,
    #[serde(default)]
    pub label: Option<String>,
}

pub async fn create_funnel(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    steps: serde_json::Value,
) -> AppResult<Funnel> {
    let funnel: Funnel = sqlx::query_as(
        "INSERT INTO funnels (project_id, name, steps) VALUES ($1, $2, $3) \
         RETURNING id, project_id, name, steps, created_at, updated_at",
    )
    .bind(project_id)
    .bind(name)
    .bind(&steps)
    .fetch_one(db)
    .await?;

    Ok(funnel)
}

pub async fn list_funnels(db: &PgPool, project_id: Uuid) -> AppResult<Vec<Funnel>> {
    let funnels: Vec<Funnel> = sqlx::query_as(
        "SELECT id, project_id, name, steps, created_at, updated_at \
         FROM funnels WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(funnels)
}

pub async fn get_funnel(
    db: &PgPool,
    project_id: Uuid,
    funnel_id: Uuid,
) -> AppResult<Option<Funnel>> {
    let funnel: Option<Funnel> = sqlx::query_as(
        "SELECT id, project_id, name, steps, created_at, updated_at \
         FROM funnels WHERE id = $1 AND project_id = $2",
    )
    .bind(funnel_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    Ok(funnel)
}

pub async fn update_funnel(
    db: &PgPool,
    project_id: Uuid,
    funnel_id: Uuid,
    name: &str,
    steps: serde_json::Value,
) -> AppResult<Funnel> {
    let funnel: Option<Funnel> = sqlx::query_as(
        "UPDATE funnels SET name = $3, steps = $4, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, steps, created_at, updated_at",
    )
    .bind(funnel_id)
    .bind(project_id)
    .bind(name)
    .bind(&steps)
    .fetch_optional(db)
    .await?;

    funnel.ok_or_else(|| AppError::NotFound("Funnel not found".to_string()))
}

pub async fn delete_funnel(db: &PgPool, project_id: Uuid, funnel_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM funnels WHERE id = $1 AND project_id = $2")
        .bind(funnel_id)
        .bind(project_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Funnel not found".to_string()));
    }
    Ok(())
}

pub async fn analyze_funnel(
    db: &PgPool,
    project_id: Uuid,
    funnel_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<Vec<FunnelStep>> {
    let funnel = get_funnel(db, project_id, funnel_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Funnel not found".to_string()))?;

    let step_defs: Vec<StepDef> = serde_json::from_value(funnel.steps.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid funnel steps: {e}")))?;

    if step_defs.is_empty() {
        return Ok(Vec::new());
    }

    let mut results: Vec<FunnelStep> = Vec::with_capacity(step_defs.len());
    let mut prev_visitors: Option<i64> = None;

    for (i, step) in step_defs.iter().enumerate() {
        let label = step
            .label
            .clone()
            .unwrap_or_else(|| format!("Step {}: {}", i + 1, step.value));

        let visitors: i64 = match step.step_type.as_str() {
            "url" => {
                let row: (i64,) = sqlx::query_as(
                    "SELECT COUNT(DISTINCT visitor_id) FROM pageviews \
                     WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
                     AND path LIKE $4",
                )
                .bind(project_id)
                .bind(start)
                .bind(end)
                .bind(&step.value)
                .fetch_one(db)
                .await?;
                row.0
            }
            "event" => {
                let row: (i64,) = sqlx::query_as(
                    "SELECT COUNT(DISTINCT visitor_id) FROM events \
                     WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
                     AND event_name = $4",
                )
                .bind(project_id)
                .bind(start)
                .bind(end)
                .bind(&step.value)
                .fetch_one(db)
                .await?;
                row.0
            }
            other => {
                return Err(AppError::BadRequest(format!("Unknown step type: {other}")));
            }
        };

        let dropoff_pct = match prev_visitors {
            Some(prev) if prev > 0 => {
                let dropped = prev - visitors;
                (dropped as f64 / prev as f64) * 100.0
            }
            _ => 0.0,
        };

        results.push(FunnelStep {
            label,
            visitors,
            dropoff_pct,
        });

        prev_visitors = Some(visitors);
    }

    Ok(results)
}
