use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    pub baseline_variant: Option<String>,
    pub winner: Option<String>,
    pub variants: Vec<VariantResult>,
}

#[derive(Debug, Serialize)]
pub struct VariantResult {
    pub name: String,
    pub assignments: i64,
    pub conversions: i64,
    pub conversion_rate: f64,
    pub lift_percent: Option<f64>,
    pub p_value: Option<f64>,
    pub confidence: Option<f64>,
    pub significant: bool,
    pub is_baseline: bool,
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
        "running" => ("COALESCE(started_at, $4::timestamptz)", "NULL::timestamptz"),
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
    let result = sqlx::query("DELETE FROM experiments WHERE id = $1 AND project_id = $2")
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

    // Stable weighted assignment so first-touch races resolve consistently.
    let mut hash_input = Vec::new();
    hash_input.extend_from_slice(project_id.as_bytes());
    hash_input.extend_from_slice(experiment_id.as_bytes());
    hash_input.extend_from_slice(visitor_id.as_bytes());
    let digest = Sha256::digest(&hash_input);
    let mut roll_bytes = [0u8; 8];
    roll_bytes.copy_from_slice(&digest[..8]);
    let roll = (u64::from_be_bytes(roll_bytes) as f64 / u64::MAX as f64) * total_weight;
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
                baseline_variant: None,
                winner: None,
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

    let assignment_map: HashMap<String, i64> = assignments.into_iter().collect();
    let configured_variants = configured_variant_names(&experiment.variants);
    let mut variant_names = if configured_variants.is_empty() {
        let mut names: Vec<String> = assignment_map.keys().cloned().collect();
        names.sort();
        names
    } else {
        configured_variants
    };
    for name in assignment_map.keys() {
        if !variant_names.iter().any(|variant| variant == name) {
            variant_names.push(name.clone());
        }
    }
    let baseline_variant = variant_names.first().cloned();

    let conversion_map: HashMap<String, i64> = if let Some(goal_id) = experiment.goal_id {
        sqlx::query_as::<_, (String, i64)>(
            "SELECT ea.variant, COUNT(DISTINCT gc.visitor_id)::bigint \
             FROM experiment_assignments ea \
             INNER JOIN goal_conversions gc ON gc.visitor_id = ea.visitor_id \
               AND gc.project_id = ea.project_id \
             WHERE ea.experiment_id = $1 AND ea.project_id = $2 AND gc.goal_id = $3 \
               AND ea.created_at >= $4 AND ea.created_at <= $5 \
               AND gc.created_at >= $4 AND gc.created_at <= $5 \
             GROUP BY ea.variant",
        )
        .bind(experiment_id)
        .bind(project_id)
        .bind(goal_id)
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await?
        .into_iter()
        .collect()
    } else {
        HashMap::new()
    };

    let baseline = baseline_variant.as_ref().map(|name| {
        (
            *assignment_map.get(name).unwrap_or(&0),
            *conversion_map.get(name).unwrap_or(&0),
        )
    });

    let mut variant_results: Vec<VariantResult> = Vec::new();

    for variant_name in variant_names {
        let assignment_count = *assignment_map.get(&variant_name).unwrap_or(&0);
        let conversions = *conversion_map.get(&variant_name).unwrap_or(&0);
        let conversion_rate = if assignment_count > 0 {
            (conversions as f64 / assignment_count as f64) * 100.0
        } else {
            0.0
        };
        let is_baseline = baseline_variant.as_ref() == Some(&variant_name);
        let comparison = baseline.and_then(|(baseline_assignments, baseline_conversions)| {
            if is_baseline {
                None
            } else {
                compare_proportions(
                    conversions,
                    assignment_count,
                    baseline_conversions,
                    baseline_assignments,
                )
            }
        });

        variant_results.push(VariantResult {
            name: variant_name,
            assignments: assignment_count,
            conversions,
            conversion_rate,
            lift_percent: comparison.as_ref().map(|stats| stats.lift_percent),
            p_value: comparison.as_ref().map(|stats| stats.p_value),
            confidence: comparison.as_ref().map(|stats| stats.confidence),
            significant: comparison.as_ref().is_some_and(|stats| stats.significant),
            is_baseline,
        });
    }

    let winner = variant_results
        .iter()
        .filter(|variant| {
            variant.significant && variant.lift_percent.is_some_and(|lift| lift > 0.0)
        })
        .max_by(|a, b| {
            a.conversion_rate
                .partial_cmp(&b.conversion_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|variant| variant.name.clone());

    Ok(ExperimentResults {
        experiment_id,
        baseline_variant,
        winner,
        variants: variant_results,
    })
}

#[derive(Debug, Clone, Copy)]
struct ProportionComparison {
    lift_percent: f64,
    p_value: f64,
    confidence: f64,
    significant: bool,
}

fn configured_variant_names(variants: &serde_json::Value) -> Vec<String> {
    variants
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|variant| {
                    variant
                        .get("name")
                        .and_then(|name| name.as_str())
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(ToString::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn compare_proportions(
    conversions: i64,
    assignments: i64,
    baseline_conversions: i64,
    baseline_assignments: i64,
) -> Option<ProportionComparison> {
    if assignments <= 0 || baseline_assignments <= 0 {
        return None;
    }

    let rate = conversions as f64 / assignments as f64;
    let baseline_rate = baseline_conversions as f64 / baseline_assignments as f64;
    let lift_percent = if baseline_rate > 0.0 {
        ((rate - baseline_rate) / baseline_rate) * 100.0
    } else if rate > 0.0 {
        100.0
    } else {
        0.0
    };

    let pooled =
        (conversions + baseline_conversions) as f64 / (assignments + baseline_assignments) as f64;
    let standard_error =
        (pooled * (1.0 - pooled) * (1.0 / assignments as f64 + 1.0 / baseline_assignments as f64))
            .sqrt();
    if standard_error <= f64::EPSILON {
        return Some(ProportionComparison {
            lift_percent,
            p_value: 1.0,
            confidence: 0.0,
            significant: false,
        });
    }

    let z = (rate - baseline_rate) / standard_error;
    let p_value = (2.0 * (1.0 - normal_cdf(z.abs()))).clamp(0.0, 1.0);
    let confidence = (1.0 - p_value) * 100.0;

    Some(ProportionComparison {
        lift_percent,
        p_value,
        confidence,
        significant: p_value < 0.05,
    })
}

fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    sign * y
}

#[cfg(test)]
mod tests {
    use super::{compare_proportions, configured_variant_names};
    use serde_json::json;

    #[test]
    fn extracts_configured_variant_order() {
        let variants = configured_variant_names(&json!([
            { "name": "control", "weight": 50 },
            { "name": "variant", "weight": 50 }
        ]));
        assert_eq!(variants, vec!["control", "variant"]);
    }

    #[test]
    fn calculates_significant_positive_lift() {
        let stats = compare_proportions(140, 1000, 100, 1000).expect("comparison");
        assert!(stats.lift_percent > 39.0);
        assert!(stats.p_value < 0.05);
        assert!(stats.confidence > 95.0);
        assert!(stats.significant);
    }

    #[test]
    fn handles_empty_or_flat_comparisons() {
        assert!(compare_proportions(1, 0, 1, 10).is_none());
        let stats = compare_proportions(0, 100, 0, 100).expect("comparison");
        assert_eq!(stats.p_value, 1.0);
        assert!(!stats.significant);
    }
}
