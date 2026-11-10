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

/// How steps must be completed for a visitor to count at step N.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FunnelOrder {
    /// Each step must happen after the previous one (default).
    #[default]
    Ordered,
    /// All of steps 1..=N must happen inside the window, in any order.
    Unordered,
}

#[derive(Debug, Clone, Copy)]
pub struct FunnelOptions {
    pub order: FunnelOrder,
    /// Maximum time from a visitor's first step-1 entry to each later step.
    pub conversion_window: chrono::Duration,
}

impl Default for FunnelOptions {
    fn default() -> Self {
        Self {
            order: FunnelOrder::Ordered,
            conversion_window: chrono::Duration::days(14),
        }
    }
}

pub const MAX_FUNNEL_STEPS: usize = 20;
pub const MAX_CONVERSION_WINDOW_DAYS: i64 = 365;

/// SQL source for a step: returns `(visitor_id, created_at)` rows matching it.
/// `$1` = project_id, `$2`/`$3` = analysis range; `param` is this step's bind index.
fn step_source_sql(step: &StepDef, param: usize) -> AppResult<String> {
    match step.step_type.as_str() {
        "url" => Ok(format!(
            "SELECT visitor_id, created_at FROM pageviews \
             WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 AND path LIKE ${param}"
        )),
        "event" => Ok(format!(
            "SELECT visitor_id, created_at FROM events \
             WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 AND event_name = ${param}"
        )),
        other => Err(AppError::BadRequest(format!("Unknown step type: {other}"))),
    }
}

/// Build one query that returns the converted-visitor count for every step.
///
/// Ordered: a visitor enters at their first step-1 occurrence (`t0`); step N
/// requires an occurrence at or after their step N-1 time and no later than
/// `t0 + window`. Counts are therefore monotonically non-increasing.
///
/// Unordered: a visitor counts at step N if every one of steps 1..=N occurs
/// within `window` of their earliest occurrence of any of those steps.
pub(crate) fn build_funnel_sql(steps: &[StepDef], order: FunnelOrder) -> AppResult<String> {
    // Binds: $1 project, $2 start, $3 end, $4 window seconds, $5.. step values.
    let mut ctes = Vec::new();
    let mut selects = Vec::new();
    for (i, step) in steps.iter().enumerate() {
        ctes.push(format!("src{i} AS ({})", step_source_sql(step, i + 5)?));
    }
    match order {
        FunnelOrder::Ordered => {
            ctes.push(
                "s0 AS (SELECT visitor_id, MIN(created_at) AS t0, MIN(created_at) AS t \
                 FROM src0 GROUP BY visitor_id)"
                    .to_string(),
            );
            for i in 1..steps.len() {
                ctes.push(format!(
                    "s{i} AS (SELECT p.visitor_id, p.t0, MIN(x.created_at) AS t \
                     FROM s{prev} p JOIN src{i} x ON x.visitor_id = p.visitor_id \
                     AND x.created_at >= p.t \
                     AND x.created_at <= p.t0 + make_interval(secs => $4::double precision) \
                     GROUP BY p.visitor_id, p.t0)",
                    prev = i - 1
                ));
            }
        }
        FunnelOrder::Unordered => {
            let union = (0..steps.len())
                .map(|i| format!("SELECT visitor_id, created_at, {i} AS step FROM src{i}"))
                .collect::<Vec<_>>()
                .join(" UNION ALL ");
            ctes.push(format!("all_steps AS ({union})"));
            ctes.push(
                "anchor AS (SELECT visitor_id, MIN(created_at) AS t0 FROM all_steps GROUP BY visitor_id)"
                    .to_string(),
            );
            ctes.push(
                "within AS (SELECT a.visitor_id, a.step FROM all_steps a JOIN anchor n \
                 ON n.visitor_id = a.visitor_id \
                 AND a.created_at <= n.t0 + make_interval(secs => $4::double precision) \
                 GROUP BY a.visitor_id, a.step)"
                    .to_string(),
            );
            for i in 0..steps.len() {
                ctes.push(format!(
                    "s{i} AS (SELECT visitor_id FROM within WHERE step <= {i} \
                     GROUP BY visitor_id HAVING COUNT(*) = {n})",
                    n = i + 1
                ));
            }
        }
    }
    for i in 0..steps.len() {
        selects.push(format!("(SELECT COUNT(*) FROM s{i})::bigint"));
    }
    Ok(format!(
        "WITH {} SELECT ARRAY[{}]::bigint[]",
        ctes.join(", "),
        selects.join(", ")
    ))
}

pub async fn analyze_funnel(
    db: &PgPool,
    project_id: Uuid,
    funnel_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    options: FunnelOptions,
) -> AppResult<Vec<FunnelStep>> {
    let funnel = get_funnel(db, project_id, funnel_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Funnel not found".to_string()))?;

    let step_defs: Vec<StepDef> = serde_json::from_value(funnel.steps.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid funnel steps: {e}")))?;

    if step_defs.is_empty() {
        return Ok(Vec::new());
    }
    if step_defs.len() > MAX_FUNNEL_STEPS {
        return Err(AppError::BadRequest(format!(
            "Funnels support at most {MAX_FUNNEL_STEPS} steps"
        )));
    }
    let window_secs = options.conversion_window.num_seconds();
    if window_secs <= 0 || window_secs > MAX_CONVERSION_WINDOW_DAYS * 86_400 {
        return Err(AppError::BadRequest(format!(
            "conversion window must be between 1 second and {MAX_CONVERSION_WINDOW_DAYS} days"
        )));
    }

    let sql = build_funnel_sql(&step_defs, options.order)?;
    let mut query = sqlx::query_as::<_, (Vec<i64>,)>(&sql)
        .bind(project_id)
        .bind(start)
        .bind(end)
        .bind(window_secs as f64);
    for step in &step_defs {
        query = query.bind(&step.value);
    }
    let (counts,) = query.fetch_one(db).await?;

    let mut results: Vec<FunnelStep> = Vec::with_capacity(step_defs.len());
    let mut prev_visitors: Option<i64> = None;
    for (i, step) in step_defs.iter().enumerate() {
        let label = step
            .label
            .clone()
            .unwrap_or_else(|| format!("Step {}: {}", i + 1, step.value));
        let visitors = counts.get(i).copied().unwrap_or(0);
        let dropoff_pct = match prev_visitors {
            Some(prev) if prev > 0 => ((prev - visitors) as f64 / prev as f64) * 100.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn step(t: &str, v: &str) -> StepDef {
        StepDef {
            step_type: t.into(),
            value: v.into(),
            label: None,
        }
    }

    #[test]
    fn ordered_sql_chains_each_step_after_previous_within_window() {
        let sql = build_funnel_sql(
            &[
                step("url", "/pricing"),
                step("event", "signup"),
                step("event", "paid"),
            ],
            FunnelOrder::Ordered,
        )
        .unwrap();
        assert!(sql.contains("s1 AS (SELECT p.visitor_id, p.t0, MIN(x.created_at) AS t FROM s0 p"));
        assert!(sql.contains("s2 AS (SELECT p.visitor_id, p.t0, MIN(x.created_at) AS t FROM s1 p"));
        assert!(sql.contains("x.created_at >= p.t"));
        assert!(sql.contains("p.t0 + make_interval(secs => $4::double precision)"));
        assert!(sql.contains("path LIKE $5"));
        assert!(sql.contains("event_name = $6"));
        assert!(sql.contains("event_name = $7"));
    }

    #[test]
    fn unordered_sql_requires_all_prior_steps() {
        let sql = build_funnel_sql(
            &[step("event", "a"), step("event", "b")],
            FunnelOrder::Unordered,
        )
        .unwrap();
        assert!(sql.contains("HAVING COUNT(*) = 1"));
        assert!(sql.contains("HAVING COUNT(*) = 2"));
    }

    #[test]
    fn rejects_unknown_step_type() {
        assert!(build_funnel_sql(&[step("click", "x")], FunnelOrder::Ordered).is_err());
    }
}
