use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeatureFlag {
    pub id: Uuid,
    pub project_id: Uuid,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub flag_type: String,
    pub default_value: serde_json::Value,
    pub variants: serde_json::Value,
    pub rollout_percentage: f64,
    pub targeting_rules: serde_json::Value,
    pub remote_config: serde_json::Value,
    pub experiment_id: Option<Uuid>,
    pub guardrail_metrics: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeatureFlagEvaluation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub flag_id: Uuid,
    pub visitor_id: String,
    pub user_id: Option<String>,
    pub matched: bool,
    pub enabled: bool,
    pub variant: Option<String>,
    pub value: serde_json::Value,
    pub reason: String,
    pub context: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RemoteConfigEntry {
    pub id: Uuid,
    pub project_id: Uuid,
    pub key: String,
    pub description: Option<String>,
    pub value: serde_json::Value,
    pub targeting_rules: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationContext {
    pub visitor_id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub traits: serde_json::Value,
    #[serde(default)]
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeatureFlagEvaluationResult {
    pub key: String,
    pub enabled: bool,
    pub matched: bool,
    pub variant: Option<String>,
    pub value: serde_json::Value,
    pub reason: String,
    pub experiment_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteConfigEvaluationResult {
    pub key: String,
    pub matched: bool,
    pub value: serde_json::Value,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TargetingDefinition {
    #[serde(default = "default_match_type", rename = "match")]
    match_type: String,
    #[serde(default)]
    conditions: Vec<TargetingCondition>,
}

#[derive(Debug, Clone, Deserialize)]
struct TargetingCondition {
    field: String,
    op: String,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

const FEATURE_FLAG_COLUMNS: &str = "id, project_id, key, name, description, enabled, flag_type, \
    default_value, variants, rollout_percentage, targeting_rules, remote_config, experiment_id, \
    guardrail_metrics, created_at, updated_at";
const FEATURE_EVALUATION_COLUMNS: &str = "id, project_id, flag_id, visitor_id, user_id, matched, \
    enabled, variant, value, reason, context, created_at";
const REMOTE_CONFIG_COLUMNS: &str = "id, project_id, key, description, value, targeting_rules, \
    is_active, created_at, updated_at";

fn default_match_type() -> String {
    "all".to_string()
}

pub async fn create_feature_flag(
    db: &PgPool,
    project_id: Uuid,
    input: FeatureFlagInput<'_>,
) -> AppResult<FeatureFlag> {
    validate_flag_input(&input)?;
    ensure_experiment_belongs_to_project(db, project_id, input.experiment_id).await?;

    let flag = sqlx::query_as(&format!(
        "INSERT INTO feature_flags \
         (project_id, key, name, description, enabled, flag_type, default_value, variants, \
          rollout_percentage, targeting_rules, remote_config, experiment_id, guardrail_metrics) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
         RETURNING {FEATURE_FLAG_COLUMNS}"
    ))
    .bind(project_id)
    .bind(input.key)
    .bind(input.name)
    .bind(input.description)
    .bind(input.enabled)
    .bind(input.flag_type)
    .bind(input.default_value)
    .bind(input.variants)
    .bind(input.rollout_percentage)
    .bind(input.targeting_rules)
    .bind(input.remote_config)
    .bind(input.experiment_id)
    .bind(input.guardrail_metrics)
    .fetch_one(db)
    .await?;

    Ok(flag)
}

pub async fn list_feature_flags(db: &PgPool, project_id: Uuid) -> AppResult<Vec<FeatureFlag>> {
    let flags = sqlx::query_as(&format!(
        "SELECT {FEATURE_FLAG_COLUMNS} FROM feature_flags \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(flags)
}

pub async fn get_feature_flag(
    db: &PgPool,
    project_id: Uuid,
    flag_id: Uuid,
) -> AppResult<FeatureFlag> {
    let flag = sqlx::query_as(&format!(
        "SELECT {FEATURE_FLAG_COLUMNS} FROM feature_flags WHERE id = $1 AND project_id = $2"
    ))
    .bind(flag_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;
    flag.ok_or_else(|| AppError::NotFound("Feature flag not found".to_string()))
}

pub async fn update_feature_flag(
    db: &PgPool,
    project_id: Uuid,
    flag_id: Uuid,
    input: FeatureFlagInput<'_>,
) -> AppResult<FeatureFlag> {
    validate_flag_input(&input)?;
    ensure_experiment_belongs_to_project(db, project_id, input.experiment_id).await?;

    let flag = sqlx::query_as(&format!(
        "UPDATE feature_flags SET key = $1, name = $2, description = $3, enabled = $4, \
         flag_type = $5, default_value = $6, variants = $7, rollout_percentage = $8, \
         targeting_rules = $9, remote_config = $10, experiment_id = $11, \
         guardrail_metrics = $12, updated_at = NOW() \
         WHERE id = $13 AND project_id = $14 RETURNING {FEATURE_FLAG_COLUMNS}"
    ))
    .bind(input.key)
    .bind(input.name)
    .bind(input.description)
    .bind(input.enabled)
    .bind(input.flag_type)
    .bind(input.default_value)
    .bind(input.variants)
    .bind(input.rollout_percentage)
    .bind(input.targeting_rules)
    .bind(input.remote_config)
    .bind(input.experiment_id)
    .bind(input.guardrail_metrics)
    .bind(flag_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    flag.ok_or_else(|| AppError::NotFound("Feature flag not found".to_string()))
}

pub async fn delete_feature_flag(db: &PgPool, project_id: Uuid, flag_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM feature_flags WHERE id = $1 AND project_id = $2")
        .bind(flag_id)
        .bind(project_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Feature flag not found".to_string()));
    }
    Ok(())
}

pub async fn evaluate_feature_flag(
    db: &PgPool,
    project_id: Uuid,
    key: &str,
    ctx: &EvaluationContext,
) -> AppResult<FeatureFlagEvaluationResult> {
    let flag = sqlx::query_as::<_, FeatureFlag>(&format!(
        "SELECT {FEATURE_FLAG_COLUMNS} FROM feature_flags WHERE project_id = $1 AND key = $2"
    ))
    .bind(project_id)
    .bind(key)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Feature flag not found".to_string()))?;

    let result = evaluate_flag_value(db, project_id, &flag, ctx).await?;
    record_flag_evaluation(db, project_id, &flag, ctx, &result).await?;
    Ok(result)
}

pub async fn list_feature_flag_evaluations(
    db: &PgPool,
    project_id: Uuid,
    flag_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<FeatureFlagEvaluation>> {
    let rows = sqlx::query_as(&format!(
        "SELECT {FEATURE_EVALUATION_COLUMNS} FROM feature_flag_evaluations \
         WHERE project_id = $1 AND flag_id = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
    ))
    .bind(project_id)
    .bind(flag_id)
    .bind(limit.clamp(1, 500))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn create_remote_config(
    db: &PgPool,
    project_id: Uuid,
    key: &str,
    description: Option<&str>,
    value: serde_json::Value,
    targeting_rules: serde_json::Value,
    is_active: bool,
) -> AppResult<RemoteConfigEntry> {
    validate_key(key)?;
    validate_targeting_rules(&targeting_rules)?;

    let entry = sqlx::query_as(&format!(
        "INSERT INTO remote_config_entries (project_id, key, description, value, targeting_rules, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {REMOTE_CONFIG_COLUMNS}"
    ))
    .bind(project_id)
    .bind(key)
    .bind(description)
    .bind(value)
    .bind(targeting_rules)
    .bind(is_active)
    .fetch_one(db)
    .await?;
    Ok(entry)
}

pub async fn list_remote_configs(
    db: &PgPool,
    project_id: Uuid,
) -> AppResult<Vec<RemoteConfigEntry>> {
    let entries = sqlx::query_as(&format!(
        "SELECT {REMOTE_CONFIG_COLUMNS} FROM remote_config_entries \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(entries)
}

pub async fn get_remote_config(
    db: &PgPool,
    project_id: Uuid,
    entry_id: Uuid,
) -> AppResult<RemoteConfigEntry> {
    let entry = sqlx::query_as(&format!(
        "SELECT {REMOTE_CONFIG_COLUMNS} FROM remote_config_entries WHERE id = $1 AND project_id = $2"
    ))
    .bind(entry_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;
    entry.ok_or_else(|| AppError::NotFound("Remote config entry not found".to_string()))
}

pub async fn update_remote_config(
    db: &PgPool,
    project_id: Uuid,
    entry_id: Uuid,
    key: &str,
    description: Option<&str>,
    value: serde_json::Value,
    targeting_rules: serde_json::Value,
    is_active: bool,
) -> AppResult<RemoteConfigEntry> {
    validate_key(key)?;
    validate_targeting_rules(&targeting_rules)?;

    let entry = sqlx::query_as(&format!(
        "UPDATE remote_config_entries SET key = $1, description = $2, value = $3, \
         targeting_rules = $4, is_active = $5, updated_at = NOW() \
         WHERE id = $6 AND project_id = $7 RETURNING {REMOTE_CONFIG_COLUMNS}"
    ))
    .bind(key)
    .bind(description)
    .bind(value)
    .bind(targeting_rules)
    .bind(is_active)
    .bind(entry_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;
    entry.ok_or_else(|| AppError::NotFound("Remote config entry not found".to_string()))
}

pub async fn delete_remote_config(db: &PgPool, project_id: Uuid, entry_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM remote_config_entries WHERE id = $1 AND project_id = $2")
        .bind(entry_id)
        .bind(project_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Remote config entry not found".to_string(),
        ));
    }
    Ok(())
}

pub async fn evaluate_remote_config(
    db: &PgPool,
    project_id: Uuid,
    key: &str,
    ctx: &EvaluationContext,
) -> AppResult<RemoteConfigEvaluationResult> {
    let entry = sqlx::query_as::<_, RemoteConfigEntry>(&format!(
        "SELECT {REMOTE_CONFIG_COLUMNS} FROM remote_config_entries WHERE project_id = $1 AND key = $2"
    ))
    .bind(project_id)
    .bind(key)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Remote config entry not found".to_string()))?;

    if !entry.is_active {
        return Ok(RemoteConfigEvaluationResult {
            key: entry.key,
            matched: false,
            value: serde_json::Value::Null,
            reason: "inactive".to_string(),
        });
    }

    let matched = matches_targeting_rules(&entry.targeting_rules, ctx)?;
    Ok(RemoteConfigEvaluationResult {
        key: entry.key,
        matched,
        value: if matched {
            entry.value
        } else {
            serde_json::Value::Null
        },
        reason: if matched { "match" } else { "no_match" }.to_string(),
    })
}

#[derive(Debug, Clone)]
pub struct FeatureFlagInput<'a> {
    pub key: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub enabled: bool,
    pub flag_type: &'a str,
    pub default_value: serde_json::Value,
    pub variants: serde_json::Value,
    pub rollout_percentage: f64,
    pub targeting_rules: serde_json::Value,
    pub remote_config: serde_json::Value,
    pub experiment_id: Option<Uuid>,
    pub guardrail_metrics: serde_json::Value,
}

async fn evaluate_flag_value(
    db: &PgPool,
    project_id: Uuid,
    flag: &FeatureFlag,
    ctx: &EvaluationContext,
) -> AppResult<FeatureFlagEvaluationResult> {
    if !flag.enabled {
        return Ok(flag_result(
            flag,
            false,
            false,
            None,
            flag.default_value.clone(),
            "disabled",
        ));
    }

    let matched = matches_targeting_rules(&flag.targeting_rules, ctx)?;
    if !matched {
        return Ok(flag_result(
            flag,
            false,
            false,
            None,
            flag.default_value.clone(),
            "no_match",
        ));
    }

    if !matches_rollout(
        project_id,
        &flag.key,
        &ctx.visitor_id,
        flag.rollout_percentage,
    ) {
        return Ok(flag_result(
            flag,
            true,
            false,
            None,
            flag.default_value.clone(),
            "rollout_miss",
        ));
    }

    if let Some(experiment_id) = flag.experiment_id {
        let variant =
            services::experiments::assign_visitor(db, project_id, experiment_id, &ctx.visitor_id)
                .await?;
        let value = variant_value(&flag.variants, &variant).unwrap_or_else(|| json!(variant));
        return Ok(flag_result(
            flag,
            true,
            true,
            Some(variant),
            value,
            "experiment_assignment",
        ));
    }

    if let Some((variant, value)) =
        weighted_variant(project_id, &flag.key, &ctx.visitor_id, &flag.variants)
    {
        return Ok(flag_result(
            flag,
            true,
            true,
            Some(variant),
            value,
            "variant_assignment",
        ));
    }

    let value = if flag.flag_type == "boolean" {
        json!(true)
    } else if flag.remote_config != json!({}) {
        flag.remote_config.clone()
    } else {
        flag.default_value.clone()
    };
    Ok(flag_result(flag, true, true, None, value, "match"))
}

fn flag_result(
    flag: &FeatureFlag,
    matched: bool,
    enabled: bool,
    variant: Option<String>,
    value: serde_json::Value,
    reason: &str,
) -> FeatureFlagEvaluationResult {
    FeatureFlagEvaluationResult {
        key: flag.key.clone(),
        enabled,
        matched,
        variant,
        value,
        reason: reason.to_string(),
        experiment_id: flag.experiment_id,
    }
}

async fn record_flag_evaluation(
    db: &PgPool,
    project_id: Uuid,
    flag: &FeatureFlag,
    ctx: &EvaluationContext,
    result: &FeatureFlagEvaluationResult,
) -> AppResult<()> {
    let context = json!({
        "traits": ctx.traits,
        "context": ctx.context,
    });
    sqlx::query(
        "INSERT INTO feature_flag_evaluations \
         (project_id, flag_id, visitor_id, user_id, matched, enabled, variant, value, reason, context) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(project_id)
    .bind(flag.id)
    .bind(&ctx.visitor_id)
    .bind(&ctx.user_id)
    .bind(result.matched)
    .bind(result.enabled)
    .bind(&result.variant)
    .bind(&result.value)
    .bind(&result.reason)
    .bind(context)
    .execute(db)
    .await?;
    Ok(())
}

fn validate_flag_input(input: &FeatureFlagInput<'_>) -> AppResult<()> {
    validate_key(input.key)?;
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Feature flag name cannot be empty".to_string(),
        ));
    }
    if !matches!(input.flag_type, "boolean" | "string" | "number" | "json") {
        return Err(AppError::BadRequest(
            "flag_type must be boolean, string, number, or json".to_string(),
        ));
    }
    if !(0.0..=100.0).contains(&input.rollout_percentage) {
        return Err(AppError::BadRequest(
            "rollout_percentage must be between 0 and 100".to_string(),
        ));
    }
    if !input.variants.is_array() {
        return Err(AppError::BadRequest(
            "variants must be an array".to_string(),
        ));
    }
    validate_targeting_rules(&input.targeting_rules)?;
    Ok(())
}

async fn ensure_experiment_belongs_to_project(
    db: &PgPool,
    project_id: Uuid,
    experiment_id: Option<Uuid>,
) -> AppResult<()> {
    let Some(experiment_id) = experiment_id else {
        return Ok(());
    };

    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM experiments WHERE id = $1 AND project_id = $2)",
    )
    .bind(experiment_id)
    .bind(project_id)
    .fetch_one(db)
    .await?;

    if !exists.0 {
        return Err(AppError::BadRequest(
            "experiment_id does not belong to this project".to_string(),
        ));
    }

    Ok(())
}

fn validate_key(key: &str) -> AppResult<()> {
    if key.trim().is_empty() {
        return Err(AppError::BadRequest("Key cannot be empty".to_string()));
    }
    if key.len() > 255 {
        return Err(AppError::BadRequest(
            "Key must be 255 characters or fewer".to_string(),
        ));
    }
    Ok(())
}

fn validate_targeting_rules(rules: &serde_json::Value) -> AppResult<()> {
    let parsed: TargetingDefinition = serde_json::from_value(rules.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid targeting_rules: {e}")))?;
    if parsed.match_type != "all" && parsed.match_type != "any" {
        return Err(AppError::BadRequest(
            "targeting_rules.match must be 'all' or 'any'".to_string(),
        ));
    }
    Ok(())
}

fn matches_targeting_rules(rules: &serde_json::Value, ctx: &EvaluationContext) -> AppResult<bool> {
    let parsed: TargetingDefinition = serde_json::from_value(rules.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid targeting_rules: {e}")))?;
    if parsed.conditions.is_empty() {
        return Ok(true);
    }

    let matches = |condition: &TargetingCondition| -> bool {
        compare_value(context_value(ctx, &condition.field).as_ref(), condition)
    };

    Ok(if parsed.match_type == "any" {
        parsed.conditions.iter().any(matches)
    } else {
        parsed.conditions.iter().all(matches)
    })
}

fn context_value(ctx: &EvaluationContext, field: &str) -> Option<serde_json::Value> {
    match field {
        "visitor_id" => Some(json!(ctx.visitor_id)),
        "user_id" => ctx.user_id.as_ref().map(|user_id| json!(user_id)),
        f if f.starts_with("traits.") => {
            json_path_value(&ctx.traits, f.trim_start_matches("traits.")).cloned()
        }
        f if f.starts_with("trait.") => {
            json_path_value(&ctx.traits, f.trim_start_matches("trait.")).cloned()
        }
        f if f.starts_with("context.") => {
            json_path_value(&ctx.context, f.trim_start_matches("context.")).cloned()
        }
        f => json_path_value(&ctx.context, f)
            .or_else(|| json_path_value(&ctx.traits, f))
            .cloned(),
    }
}

fn compare_value(value: Option<&serde_json::Value>, condition: &TargetingCondition) -> bool {
    match condition.op.as_str() {
        "exists" => value.is_some_and(|v| !v.is_null()),
        "not_exists" => value.is_none_or(|v| v.is_null()),
        "eq" => values_equal(value, condition.value.as_ref()),
        "neq" => !values_equal(value, condition.value.as_ref()),
        "contains" => value.and_then(value_as_string).is_some_and(|actual| {
            condition_string(condition).is_some_and(|needle| actual.contains(&needle))
        }),
        "starts_with" => value.and_then(value_as_string).is_some_and(|actual| {
            condition_string(condition).is_some_and(|needle| actual.starts_with(&needle))
        }),
        "ends_with" => value.and_then(value_as_string).is_some_and(|actual| {
            condition_string(condition).is_some_and(|needle| actual.ends_with(&needle))
        }),
        "gt" | "gte" | "lt" | "lte" => compare_numeric(value, condition),
        "in" => condition
            .value
            .as_ref()
            .and_then(|v| v.as_array())
            .is_some_and(|items| items.iter().any(|item| values_equal(value, Some(item)))),
        _ => false,
    }
}

fn values_equal(actual: Option<&serde_json::Value>, expected: Option<&serde_json::Value>) -> bool {
    match (actual, expected) {
        (Some(a), Some(e)) if a == e => true,
        (Some(a), Some(e)) => value_as_string(a) == value_as_string(e),
        _ => false,
    }
}

fn compare_numeric(value: Option<&serde_json::Value>, condition: &TargetingCondition) -> bool {
    let Some(actual) = value.and_then(value_as_f64) else {
        return false;
    };
    let Some(expected) = condition.value.as_ref().and_then(value_as_f64) else {
        return false;
    };
    match condition.op.as_str() {
        "gt" => actual > expected,
        "gte" => actual >= expected,
        "lt" => actual < expected,
        "lte" => actual <= expected,
        _ => false,
    }
}

fn value_as_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn condition_string(condition: &TargetingCondition) -> Option<String> {
    condition.value.as_ref().and_then(value_as_string)
}

fn json_path_value<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn matches_rollout(project_id: Uuid, key: &str, visitor_id: &str, rollout_percentage: f64) -> bool {
    if rollout_percentage >= 100.0 {
        return true;
    }
    if rollout_percentage <= 0.0 {
        return false;
    }
    stable_roll(project_id, key, visitor_id, "rollout") < rollout_percentage
}

fn stable_roll(project_id: Uuid, key: &str, visitor_id: &str, salt: &str) -> f64 {
    let mut hash_input = Vec::new();
    hash_input.extend_from_slice(project_id.as_bytes());
    hash_input.extend_from_slice(key.as_bytes());
    hash_input.extend_from_slice(visitor_id.as_bytes());
    hash_input.extend_from_slice(salt.as_bytes());
    let digest = Sha256::digest(&hash_input);
    let mut roll_bytes = [0u8; 8];
    roll_bytes.copy_from_slice(&digest[..8]);
    (u64::from_be_bytes(roll_bytes) as f64 / u64::MAX as f64) * 100.0
}

fn weighted_variant(
    project_id: Uuid,
    key: &str,
    visitor_id: &str,
    variants: &serde_json::Value,
) -> Option<(String, serde_json::Value)> {
    let variants = variants.as_array()?;
    if variants.is_empty() {
        return None;
    }

    let mut parsed = Vec::new();
    let mut total_weight = 0.0;
    for variant in variants {
        let name = variant
            .get("name")
            .or_else(|| variant.get("variant"))
            .and_then(|v| v.as_str())?
            .to_string();
        let weight = variant
            .get("weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        if weight <= 0.0 {
            continue;
        }
        let value = variant.get("value").cloned().unwrap_or_else(|| json!(name));
        total_weight += weight;
        parsed.push((name, weight, value));
    }
    if parsed.is_empty() || total_weight <= 0.0 {
        return None;
    }

    let roll = stable_roll(project_id, key, visitor_id, "variant") / 100.0 * total_weight;
    let mut cumulative = 0.0;
    for (name, weight, value) in parsed {
        cumulative += weight;
        if roll < cumulative {
            return Some((name, value));
        }
    }
    None
}

fn variant_value(variants: &serde_json::Value, variant_name: &str) -> Option<serde_json::Value> {
    variants.as_array()?.iter().find_map(|variant| {
        let name = variant
            .get("name")
            .or_else(|| variant.get("variant"))
            .and_then(|v| v.as_str())?;
        if name == variant_name {
            Some(variant.get("value").cloned().unwrap_or_else(|| json!(name)))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{matches_rollout, matches_targeting_rules, weighted_variant, EvaluationContext};

    #[test]
    fn matches_trait_and_context_targeting() {
        let ctx = EvaluationContext {
            visitor_id: "v1".to_string(),
            user_id: Some("user_1".to_string()),
            traits: serde_json::json!({ "plan": "pro" }),
            context: serde_json::json!({ "country": "US" }),
        };
        let rules = serde_json::json!({
            "match": "all",
            "conditions": [
                { "field": "traits.plan", "op": "eq", "value": "pro" },
                { "field": "country", "op": "eq", "value": "US" }
            ]
        });

        assert!(matches_targeting_rules(&rules, &ctx).unwrap());
    }

    #[test]
    fn rollout_is_stable_and_bounded() {
        let project_id = uuid::Uuid::nil();
        assert!(matches_rollout(project_id, "new_nav", "v1", 100.0));
        assert!(!matches_rollout(project_id, "new_nav", "v1", 0.0));
        assert_eq!(
            matches_rollout(project_id, "new_nav", "v1", 25.0),
            matches_rollout(project_id, "new_nav", "v1", 25.0)
        );
    }

    #[test]
    fn selects_weighted_variant() {
        let variants = serde_json::json!([
            { "name": "control", "weight": 50, "value": false },
            { "name": "treatment", "weight": 50, "value": true }
        ]);

        let first = weighted_variant(uuid::Uuid::nil(), "checkout", "v1", &variants);
        let second = weighted_variant(uuid::Uuid::nil(), "checkout", "v1", &variants);

        assert_eq!(first, second);
        assert!(first.is_some());
    }
}
