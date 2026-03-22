use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Experiment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub variants: serde_json::Value,
    pub goal_id: Option<Uuid>,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ExperimentResults {
    pub experiment_id: Uuid,
    pub variants: Vec<VariantResult>,
}

#[derive(Debug, Serialize)]
pub struct VariantResult {
    pub name: String,
    pub assignments: i64,
    pub conversions: i64,
    pub conversion_rate: f64,
}

const EXPERIMENT_COLUMNS: &str = "id, project_id, name, description, variants, goal_id, status, \
    started_at, ended_at, created_at, updated_at";

/// Create a new experiment.
pub async fn create_experiment(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    variants: &serde_json::Value,
    goal_id: Option<Uuid>,
) -> Result<Experiment, sqlx::Error> {
    let experiment: Experiment = sqlx::query_as(&format!(
        "INSERT INTO experiments (project_id, name, description, variants, goal_id) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING {EXPERIMENT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name)
    .bind(description)
    .bind(variants)
    .bind(goal_id)
    .fetch_one(db)
    .await?;

    Ok(experiment)
}

/// List all experiments for a project.
pub async fn list_experiments(
    db: &PgPool,
    project_id: Uuid,
) -> Result<Vec<Experiment>, sqlx::Error> {
    let experiments: Vec<Experiment> = sqlx::query_as(&format!(
        "SELECT {EXPERIMENT_COLUMNS} FROM experiments WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(experiments)
}

/// Get a single experiment by ID.
pub async fn get_experiment(
    db: &PgPool,
    project_id: Uuid,
    experiment_id: Uuid,
) -> Result<Option<Experiment>, sqlx::Error> {
    let experiment: Option<Experiment> = sqlx::query_as(&format!(
        "SELECT {EXPERIMENT_COLUMNS} FROM experiments WHERE id = $1 AND project_id = $2"
    ))
    .bind(experiment_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    Ok(experiment)
}

/// Update experiment status (draft, running, paused, completed).
pub async fn update_experiment_status(
    db: &PgPool,
    project_id: Uuid,
    experiment_id: Uuid,
    status: &str,
) -> Result<Experiment, sqlx::Error> {
    let now = Utc::now();

    let (started_at_expr, ended_at_expr) = match status {
        "running" => (
            "COALESCE(started_at, $4::timestamptz)",
            "NULL::timestamptz",
        ),
        "completed" => ("started_at", "$4::timestamptz"),
        _ => ("started_at", "ended_at"),
    };

    let query = format!(
        "UPDATE experiments SET status = $1, started_at = {started_at_expr}, \
         ended_at = {ended_at_expr}, updated_at = NOW() \
         WHERE id = $2 AND project_id = $3 \
         RETURNING {EXPERIMENT_COLUMNS}"
    );

    let experiment: Experiment = sqlx::query_as(&query)
        .bind(status)
        .bind(experiment_id)
        .bind(project_id)
        .bind(now)
        .fetch_one(db)
        .await?;

    Ok(experiment)
}

/// Delete an experiment.
pub async fn delete_experiment(
    db: &PgPool,
    project_id: Uuid,
    experiment_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM experiments WHERE id = $1 AND project_id = $2",
    )
    .bind(experiment_id)
    .bind(project_id)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Assign a visitor to an experiment variant.
/// If the visitor is already assigned, return the existing variant.
/// Otherwise, randomly assign based on variant weights.
pub async fn assign_visitor(
    db: &PgPool,
    project_id: Uuid,
    experiment_id: Uuid,
    visitor_id: &str,
) -> Result<String, sqlx::Error> {
    // Check if already assigned
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT variant FROM experiment_assignments \
         WHERE experiment_id = $1 AND visitor_id = $2 LIMIT 1",
    )
    .bind(experiment_id)
    .bind(visitor_id)
    .fetch_optional(db)
    .await?;

    if let Some((variant,)) = existing {
        return Ok(variant);
    }

    // Fetch the experiment to get variants
    let experiment: Option<Experiment> = sqlx::query_as(&format!(
        "SELECT {EXPERIMENT_COLUMNS} FROM experiments WHERE id = $1 AND project_id = $2"
    ))
    .bind(experiment_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    let experiment = match experiment {
        Some(e) => e,
        None => return Ok("control".to_string()),
    };

    // Parse variants: expected format [{"name": "control", "weight": 50}, ...]
    let variants = match experiment.variants.as_array() {
        Some(v) if !v.is_empty() => v,
        _ => return Ok("control".to_string()),
    };

    let mut names: Vec<String> = Vec::new();
    let mut weights: Vec<f64> = Vec::new();

    for v in variants {
        let name = v
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        let weight = v.get("weight").and_then(|w| w.as_f64()).unwrap_or(1.0);
        names.push(name);
        weights.push(weight);
    }

    let total_weight: f64 = weights.iter().sum();
    if total_weight <= 0.0 {
        return Ok("control".to_string());
    }

    // Weighted random selection
    let mut rng = rand::thread_rng();
    let roll: f64 = rng.gen::<f64>() * total_weight;
    let mut cumulative = 0.0;
    let mut selected = &names[0];

    for (i, weight) in weights.iter().enumerate() {
        cumulative += weight;
        if roll < cumulative {
            selected = &names[i];
            break;
        }
    }

    let variant = selected.clone();

    // Insert assignment
    sqlx::query(
        "INSERT INTO experiment_assignments (project_id, experiment_id, visitor_id, variant) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(project_id)
    .bind(experiment_id)
    .bind(visitor_id)
    .bind(&variant)
    .execute(db)
    .await?;

    Ok(variant)
}

/// Get experiment results: per-variant assignments, conversions, and conversion rate.
pub async fn get_experiment_results(
    db: &PgPool,
    project_id: Uuid,
    experiment_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<ExperimentResults, sqlx::Error> {
    // Fetch experiment to check goal_id
    let experiment: Option<Experiment> = sqlx::query_as(&format!(
        "SELECT {EXPERIMENT_COLUMNS} FROM experiments WHERE id = $1 AND project_id = $2"
    ))
    .bind(experiment_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    let experiment = match experiment {
        Some(e) => e,
        None => {
            return Ok(ExperimentResults {
                experiment_id,
                variants: vec![],
            })
        }
    };

    // Count assignments per variant
    let assignments: Vec<(String, i64)> = sqlx::query_as(
        "SELECT variant, COUNT(*)::bigint FROM experiment_assignments \
         WHERE experiment_id = $1 AND project_id = $2 \
         AND created_at >= $3 AND created_at <= $4 \
         GROUP BY variant ORDER BY variant",
    )
    .bind(experiment_id)
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    let mut variant_results: Vec<VariantResult> = Vec::new();

    for (variant_name, assignment_count) in &assignments {
        let conversions = if let Some(goal_id) = experiment.goal_id {
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(DISTINCT gc.visitor_id)::bigint \
                 FROM goal_conversions gc \
                 INNER JOIN experiment_assignments ea ON ea.visitor_id = gc.visitor_id \
                 AND ea.experiment_id = $1 AND ea.variant = $2 \
                 WHERE gc.goal_id = $3 AND gc.project_id = $4 \
                 AND gc.created_at >= $5 AND gc.created_at <= $6",
            )
            .bind(experiment_id)
            .bind(variant_name)
            .bind(goal_id)
            .bind(project_id)
            .bind(start)
            .bind(end)
            .fetch_one(db)
            .await?;
            row.0
        } else {
            0
        };

        let conversion_rate = if *assignment_count > 0 {
            (conversions as f64 / *assignment_count as f64) * 100.0
        } else {
            0.0
        };

        variant_results.push(VariantResult {
            name: variant_name.clone(),
            assignments: *assignment_count,
            conversions,
            conversion_rate,
        });
    }

    Ok(ExperimentResults {
        experiment_id,
        variants: variant_results,
    })
}
