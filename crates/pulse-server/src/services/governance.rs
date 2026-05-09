use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrackingPlan {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enforcement_mode: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventSchemaDefinition {
    pub id: Uuid,
    pub project_id: Uuid,
    pub tracking_plan_id: Option<Uuid>,
    pub event_name: String,
    pub description: Option<String>,
    pub status: String,
    pub required_properties: Vec<String>,
    pub property_schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DataDictionaryEntry {
    pub id: Uuid,
    pub project_id: Uuid,
    pub entry_type: String,
    pub name: String,
    pub data_type: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub is_pii: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventQualityViolation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub tracking_plan_id: Option<Uuid>,
    pub event_schema_id: Option<Uuid>,
    pub event_name: String,
    pub visitor_id: Option<String>,
    pub violation_type: String,
    pub message: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventValidationOutcome {
    pub accepted: bool,
    pub violations: Vec<EventValidationViolation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventValidationViolation {
    pub violation_type: String,
    pub message: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct GovernanceHealth {
    pub status: String,
    pub active_tracking_plan: Option<TrackingPlan>,
    pub observed_events_24h: i64,
    pub covered_events_24h: i64,
    pub coverage_ratio: f64,
    pub approved_event_schemas: i64,
    pub violations_24h: i64,
    pub unknown_events_24h: i64,
    pub unapproved_events_24h: i64,
    pub missing_property_violations_24h: i64,
    pub type_mismatch_violations_24h: i64,
    pub top_violations: Vec<EventQualitySummaryRow>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct EventQualitySummaryRow {
    pub event_name: String,
    pub violation_type: String,
    pub count: i64,
}

const TRACKING_PLAN_COLUMNS: &str = "id, project_id, name, description, enforcement_mode, \
    is_active, created_at, updated_at";
const EVENT_SCHEMA_COLUMNS: &str = "id, project_id, tracking_plan_id, event_name, description, \
    status, required_properties, property_schema, created_at, updated_at";
const DICTIONARY_COLUMNS: &str =
    "id, project_id, entry_type, name, data_type, description, owner, \
    is_pii, created_at, updated_at";
const VIOLATION_COLUMNS: &str = "id, project_id, tracking_plan_id, event_schema_id, event_name, \
    visitor_id, violation_type, message, details, created_at";

pub async fn create_tracking_plan(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    enforcement_mode: &str,
    is_active: bool,
) -> AppResult<TrackingPlan> {
    validate_name(name)?;
    validate_enforcement_mode(enforcement_mode)?;

    let plan: TrackingPlan = sqlx::query_as(&format!(
        "INSERT INTO tracking_plans (project_id, name, description, enforcement_mode, is_active) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {TRACKING_PLAN_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name)
    .bind(description)
    .bind(enforcement_mode)
    .bind(is_active)
    .fetch_one(db)
    .await?;

    Ok(plan)
}

pub async fn list_tracking_plans(db: &PgPool, project_id: Uuid) -> AppResult<Vec<TrackingPlan>> {
    let plans = sqlx::query_as(&format!(
        "SELECT {TRACKING_PLAN_COLUMNS} FROM tracking_plans \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(plans)
}

pub async fn get_tracking_plan(
    db: &PgPool,
    project_id: Uuid,
    plan_id: Uuid,
) -> AppResult<TrackingPlan> {
    let plan = sqlx::query_as(&format!(
        "SELECT {TRACKING_PLAN_COLUMNS} FROM tracking_plans WHERE id = $1 AND project_id = $2"
    ))
    .bind(plan_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    plan.ok_or_else(|| AppError::NotFound("Tracking plan not found".to_string()))
}

pub async fn update_tracking_plan(
    db: &PgPool,
    project_id: Uuid,
    plan_id: Uuid,
    name: &str,
    description: Option<&str>,
    enforcement_mode: &str,
    is_active: bool,
) -> AppResult<TrackingPlan> {
    validate_name(name)?;
    validate_enforcement_mode(enforcement_mode)?;

    let plan = sqlx::query_as(&format!(
        "UPDATE tracking_plans SET name = $1, description = $2, enforcement_mode = $3, \
         is_active = $4, updated_at = NOW() \
         WHERE id = $5 AND project_id = $6 RETURNING {TRACKING_PLAN_COLUMNS}"
    ))
    .bind(name)
    .bind(description)
    .bind(enforcement_mode)
    .bind(is_active)
    .bind(plan_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    plan.ok_or_else(|| AppError::NotFound("Tracking plan not found".to_string()))
}

pub async fn delete_tracking_plan(db: &PgPool, project_id: Uuid, plan_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM tracking_plans WHERE id = $1 AND project_id = $2")
        .bind(plan_id)
        .bind(project_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Tracking plan not found".to_string()));
    }
    Ok(())
}

pub async fn create_event_schema(
    db: &PgPool,
    project_id: Uuid,
    tracking_plan_id: Option<Uuid>,
    event_name: &str,
    description: Option<&str>,
    status: &str,
    required_properties: &[String],
    property_schema: serde_json::Value,
) -> AppResult<EventSchemaDefinition> {
    validate_event_name(event_name)?;
    validate_schema_status(status)?;
    validate_property_schema(&property_schema)?;
    ensure_tracking_plan_belongs_to_project(db, project_id, tracking_plan_id).await?;

    let schema = sqlx::query_as(&format!(
        "INSERT INTO event_schema_definitions \
         (project_id, tracking_plan_id, event_name, description, status, required_properties, property_schema) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {EVENT_SCHEMA_COLUMNS}"
    ))
    .bind(project_id)
    .bind(tracking_plan_id)
    .bind(event_name)
    .bind(description)
    .bind(status)
    .bind(required_properties)
    .bind(property_schema)
    .fetch_one(db)
    .await?;

    Ok(schema)
}

pub async fn list_event_schemas(
    db: &PgPool,
    project_id: Uuid,
    tracking_plan_id: Option<Uuid>,
) -> AppResult<Vec<EventSchemaDefinition>> {
    let schemas = if let Some(plan_id) = tracking_plan_id {
        sqlx::query_as(&format!(
            "SELECT {EVENT_SCHEMA_COLUMNS} FROM event_schema_definitions \
             WHERE project_id = $1 AND tracking_plan_id = $2 ORDER BY event_name ASC"
        ))
        .bind(project_id)
        .bind(plan_id)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(&format!(
            "SELECT {EVENT_SCHEMA_COLUMNS} FROM event_schema_definitions \
             WHERE project_id = $1 ORDER BY event_name ASC"
        ))
        .bind(project_id)
        .fetch_all(db)
        .await?
    };

    Ok(schemas)
}

pub async fn get_event_schema(
    db: &PgPool,
    project_id: Uuid,
    schema_id: Uuid,
) -> AppResult<EventSchemaDefinition> {
    let schema = sqlx::query_as(&format!(
        "SELECT {EVENT_SCHEMA_COLUMNS} FROM event_schema_definitions WHERE id = $1 AND project_id = $2"
    ))
    .bind(schema_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    schema.ok_or_else(|| AppError::NotFound("Event schema not found".to_string()))
}

pub async fn update_event_schema(
    db: &PgPool,
    project_id: Uuid,
    schema_id: Uuid,
    tracking_plan_id: Option<Uuid>,
    event_name: &str,
    description: Option<&str>,
    status: &str,
    required_properties: &[String],
    property_schema: serde_json::Value,
) -> AppResult<EventSchemaDefinition> {
    validate_event_name(event_name)?;
    validate_schema_status(status)?;
    validate_property_schema(&property_schema)?;
    ensure_tracking_plan_belongs_to_project(db, project_id, tracking_plan_id).await?;

    let schema = sqlx::query_as(&format!(
        "UPDATE event_schema_definitions SET tracking_plan_id = $1, event_name = $2, \
         description = $3, status = $4, required_properties = $5, property_schema = $6, \
         updated_at = NOW() WHERE id = $7 AND project_id = $8 RETURNING {EVENT_SCHEMA_COLUMNS}"
    ))
    .bind(tracking_plan_id)
    .bind(event_name)
    .bind(description)
    .bind(status)
    .bind(required_properties)
    .bind(property_schema)
    .bind(schema_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    schema.ok_or_else(|| AppError::NotFound("Event schema not found".to_string()))
}

pub async fn update_event_schema_status(
    db: &PgPool,
    project_id: Uuid,
    schema_id: Uuid,
    status: &str,
) -> AppResult<EventSchemaDefinition> {
    validate_schema_status(status)?;

    let schema = sqlx::query_as(&format!(
        "UPDATE event_schema_definitions SET status = $1, updated_at = NOW() \
         WHERE id = $2 AND project_id = $3 RETURNING {EVENT_SCHEMA_COLUMNS}"
    ))
    .bind(status)
    .bind(schema_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    schema.ok_or_else(|| AppError::NotFound("Event schema not found".to_string()))
}

pub async fn delete_event_schema(db: &PgPool, project_id: Uuid, schema_id: Uuid) -> AppResult<()> {
    let result =
        sqlx::query("DELETE FROM event_schema_definitions WHERE id = $1 AND project_id = $2")
            .bind(schema_id)
            .bind(project_id)
            .execute(db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Event schema not found".to_string()));
    }
    Ok(())
}

pub async fn create_dictionary_entry(
    db: &PgPool,
    project_id: Uuid,
    entry_type: &str,
    name: &str,
    data_type: Option<&str>,
    description: Option<&str>,
    owner: Option<&str>,
    is_pii: bool,
) -> AppResult<DataDictionaryEntry> {
    validate_dictionary_entry(entry_type, name)?;

    let entry = sqlx::query_as(&format!(
        "INSERT INTO data_dictionary_entries \
         (project_id, entry_type, name, data_type, description, owner, is_pii) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {DICTIONARY_COLUMNS}"
    ))
    .bind(project_id)
    .bind(entry_type)
    .bind(name)
    .bind(data_type)
    .bind(description)
    .bind(owner)
    .bind(is_pii)
    .fetch_one(db)
    .await?;

    Ok(entry)
}

pub async fn list_dictionary_entries(
    db: &PgPool,
    project_id: Uuid,
    entry_type: Option<&str>,
) -> AppResult<Vec<DataDictionaryEntry>> {
    let entries = if let Some(entry_type) = entry_type {
        sqlx::query_as(&format!(
            "SELECT {DICTIONARY_COLUMNS} FROM data_dictionary_entries \
             WHERE project_id = $1 AND entry_type = $2 ORDER BY entry_type ASC, name ASC"
        ))
        .bind(project_id)
        .bind(entry_type)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(&format!(
            "SELECT {DICTIONARY_COLUMNS} FROM data_dictionary_entries \
             WHERE project_id = $1 ORDER BY entry_type ASC, name ASC"
        ))
        .bind(project_id)
        .fetch_all(db)
        .await?
    };

    Ok(entries)
}

pub async fn update_dictionary_entry(
    db: &PgPool,
    project_id: Uuid,
    entry_id: Uuid,
    entry_type: &str,
    name: &str,
    data_type: Option<&str>,
    description: Option<&str>,
    owner: Option<&str>,
    is_pii: bool,
) -> AppResult<DataDictionaryEntry> {
    validate_dictionary_entry(entry_type, name)?;

    let entry = sqlx::query_as(&format!(
        "UPDATE data_dictionary_entries SET entry_type = $1, name = $2, data_type = $3, \
         description = $4, owner = $5, is_pii = $6, updated_at = NOW() \
         WHERE id = $7 AND project_id = $8 RETURNING {DICTIONARY_COLUMNS}"
    ))
    .bind(entry_type)
    .bind(name)
    .bind(data_type)
    .bind(description)
    .bind(owner)
    .bind(is_pii)
    .bind(entry_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    entry.ok_or_else(|| AppError::NotFound("Data dictionary entry not found".to_string()))
}

pub async fn delete_dictionary_entry(
    db: &PgPool,
    project_id: Uuid,
    entry_id: Uuid,
) -> AppResult<()> {
    let result =
        sqlx::query("DELETE FROM data_dictionary_entries WHERE id = $1 AND project_id = $2")
            .bind(entry_id)
            .bind(project_id)
            .execute(db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Data dictionary entry not found".to_string(),
        ));
    }
    Ok(())
}

pub async fn list_quality_violations(
    db: &PgPool,
    project_id: Uuid,
    event_name: Option<&str>,
    violation_type: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<EventQualityViolation>> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    let rows = sqlx::query_as(&format!(
        "SELECT {VIOLATION_COLUMNS} FROM event_quality_violations \
         WHERE project_id = $1 \
           AND ($2::text IS NULL OR event_name = $2) \
           AND ($3::text IS NULL OR violation_type = $3) \
         ORDER BY created_at DESC LIMIT $4 OFFSET $5"
    ))
    .bind(project_id)
    .bind(event_name)
    .bind(violation_type)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

pub async fn governance_health(db: &PgPool, project_id: Uuid) -> AppResult<GovernanceHealth> {
    let active_plan = active_tracking_plan(db, project_id).await?;
    let plan_id = active_plan.as_ref().map(|p| p.id);

    let observed_events: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT event_name)::bigint FROM events \
         WHERE project_id = $1 AND created_at >= NOW() - INTERVAL '24 hours'",
    )
    .bind(project_id)
    .fetch_one(db)
    .await?;

    let covered_events: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT e.event_name)::bigint FROM events e \
         JOIN event_schema_definitions s ON s.project_id = e.project_id \
           AND s.event_name = e.event_name \
           AND s.status = 'approved' \
           AND ($2::uuid IS NULL OR s.tracking_plan_id = $2 OR s.tracking_plan_id IS NULL) \
         WHERE e.project_id = $1 AND e.created_at >= NOW() - INTERVAL '24 hours'",
    )
    .bind(project_id)
    .bind(plan_id)
    .fetch_one(db)
    .await?;

    let approved_event_schemas: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM event_schema_definitions \
         WHERE project_id = $1 AND status = 'approved' \
           AND ($2::uuid IS NULL OR tracking_plan_id = $2 OR tracking_plan_id IS NULL)",
    )
    .bind(project_id)
    .bind(plan_id)
    .fetch_one(db)
    .await?;

    let violation_counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT violation_type, COUNT(*)::bigint FROM event_quality_violations \
         WHERE project_id = $1 AND created_at >= NOW() - INTERVAL '24 hours' \
         GROUP BY violation_type",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    let mut violations_24h = 0;
    let mut unknown_events_24h = 0;
    let mut unapproved_events_24h = 0;
    let mut missing_property_violations_24h = 0;
    let mut type_mismatch_violations_24h = 0;
    for (violation_type, count) in violation_counts {
        violations_24h += count;
        match violation_type.as_str() {
            "unknown_event" => unknown_events_24h = count,
            "unapproved_event" => unapproved_events_24h = count,
            "missing_required_property" => missing_property_violations_24h = count,
            "property_type_mismatch" => type_mismatch_violations_24h = count,
            _ => {}
        }
    }

    let top_violations = sqlx::query_as(
        "SELECT event_name, violation_type, COUNT(*)::bigint AS count \
         FROM event_quality_violations \
         WHERE project_id = $1 AND created_at >= NOW() - INTERVAL '24 hours' \
         GROUP BY event_name, violation_type ORDER BY count DESC LIMIT 10",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    let coverage_ratio = if observed_events.0 == 0 {
        1.0
    } else {
        covered_events.0 as f64 / observed_events.0 as f64
    };
    let status = if active_plan.is_none() {
        "not_configured"
    } else if unknown_events_24h > 0 || coverage_ratio < 0.8 {
        "critical"
    } else if violations_24h > 0 || coverage_ratio < 1.0 {
        "warning"
    } else {
        "healthy"
    }
    .to_string();

    Ok(GovernanceHealth {
        status,
        active_tracking_plan: active_plan,
        observed_events_24h: observed_events.0,
        covered_events_24h: covered_events.0,
        coverage_ratio,
        approved_event_schemas: approved_event_schemas.0,
        violations_24h,
        unknown_events_24h,
        unapproved_events_24h,
        missing_property_violations_24h,
        type_mismatch_violations_24h,
        top_violations,
    })
}

pub async fn validate_event_payload(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    event_name: &str,
    event_data: Option<&serde_json::Value>,
    now: DateTime<Utc>,
) -> AppResult<EventValidationOutcome> {
    let Some(plan) = active_tracking_plan(db, project_id).await? else {
        return Ok(EventValidationOutcome {
            accepted: true,
            violations: Vec::new(),
        });
    };

    let schema = find_schema_for_event(db, project_id, Some(plan.id), event_name).await?;
    let violations = match &schema {
        Some(schema) => collect_schema_violations(schema, event_data),
        None => vec![EventValidationViolation {
            violation_type: "unknown_event".to_string(),
            message: format!("Event '{event_name}' is not defined in the active tracking plan"),
            details: json!({ "event_name": event_name }),
        }],
    };

    if !violations.is_empty() {
        record_quality_violations(
            db,
            project_id,
            Some(plan.id),
            schema.as_ref().map(|s| s.id),
            event_name,
            Some(visitor_id),
            &violations,
            now,
        )
        .await?;
    }

    Ok(EventValidationOutcome {
        accepted: plan.enforcement_mode != "reject" || violations.is_empty(),
        violations,
    })
}

async fn active_tracking_plan(db: &PgPool, project_id: Uuid) -> AppResult<Option<TrackingPlan>> {
    let plan = sqlx::query_as(&format!(
        "SELECT {TRACKING_PLAN_COLUMNS} FROM tracking_plans \
         WHERE project_id = $1 AND is_active = true ORDER BY updated_at DESC LIMIT 1"
    ))
    .bind(project_id)
    .fetch_optional(db)
    .await?;
    Ok(plan)
}

async fn ensure_tracking_plan_belongs_to_project(
    db: &PgPool,
    project_id: Uuid,
    tracking_plan_id: Option<Uuid>,
) -> AppResult<()> {
    let Some(tracking_plan_id) = tracking_plan_id else {
        return Ok(());
    };

    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM tracking_plans WHERE id = $1 AND project_id = $2)",
    )
    .bind(tracking_plan_id)
    .bind(project_id)
    .fetch_one(db)
    .await?;

    if !exists.0 {
        return Err(AppError::BadRequest(
            "tracking_plan_id does not belong to this project".to_string(),
        ));
    }

    Ok(())
}

async fn find_schema_for_event(
    db: &PgPool,
    project_id: Uuid,
    tracking_plan_id: Option<Uuid>,
    event_name: &str,
) -> AppResult<Option<EventSchemaDefinition>> {
    let schema = sqlx::query_as(&format!(
        "SELECT {EVENT_SCHEMA_COLUMNS} FROM event_schema_definitions \
         WHERE project_id = $1 AND event_name = $2 \
           AND ($3::uuid IS NULL OR tracking_plan_id = $3 OR tracking_plan_id IS NULL) \
         ORDER BY CASE WHEN tracking_plan_id = $3 THEN 0 ELSE 1 END, updated_at DESC LIMIT 1"
    ))
    .bind(project_id)
    .bind(event_name)
    .bind(tracking_plan_id)
    .fetch_optional(db)
    .await?;
    Ok(schema)
}

async fn record_quality_violations(
    db: &PgPool,
    project_id: Uuid,
    tracking_plan_id: Option<Uuid>,
    event_schema_id: Option<Uuid>,
    event_name: &str,
    visitor_id: Option<&str>,
    violations: &[EventValidationViolation],
    now: DateTime<Utc>,
) -> AppResult<()> {
    for violation in violations {
        sqlx::query(
            "INSERT INTO event_quality_violations \
             (project_id, tracking_plan_id, event_schema_id, event_name, visitor_id, violation_type, message, details, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(project_id)
        .bind(tracking_plan_id)
        .bind(event_schema_id)
        .bind(event_name)
        .bind(visitor_id)
        .bind(&violation.violation_type)
        .bind(&violation.message)
        .bind(&violation.details)
        .bind(now)
        .execute(db)
        .await?;
    }
    Ok(())
}

fn collect_schema_violations(
    schema: &EventSchemaDefinition,
    event_data: Option<&serde_json::Value>,
) -> Vec<EventValidationViolation> {
    let mut violations = Vec::new();

    if schema.status != "approved" {
        violations.push(EventValidationViolation {
            violation_type: "unapproved_event".to_string(),
            message: format!(
                "Event '{}' has schema status '{}'",
                schema.event_name, schema.status
            ),
            details: json!({ "status": schema.status }),
        });
    }

    let mut required_properties: HashSet<String> =
        schema.required_properties.iter().cloned().collect();
    if let Some(properties) = schema.property_schema.as_object() {
        for (field, rule) in properties {
            if rule_required(rule) {
                required_properties.insert(field.clone());
            }
        }
    }

    for field in required_properties {
        if json_path_value(event_data, &field).is_none_or(|value| value.is_null()) {
            violations.push(EventValidationViolation {
                violation_type: "missing_required_property".to_string(),
                message: format!(
                    "Event '{}' is missing required property '{}'",
                    schema.event_name, field
                ),
                details: json!({ "property": field }),
            });
        }
    }

    if let Some(properties) = schema.property_schema.as_object() {
        for (field, rule) in properties {
            let Some(expected_type) = expected_type_from_rule(rule) else {
                continue;
            };
            let Some(actual_value) = json_path_value(event_data, field) else {
                continue;
            };
            if actual_value.is_null() {
                continue;
            }
            if !value_matches_type(actual_value, expected_type) {
                violations.push(EventValidationViolation {
                    violation_type: "property_type_mismatch".to_string(),
                    message: format!(
                        "Event '{}' property '{}' expected type '{}'",
                        schema.event_name, field, expected_type
                    ),
                    details: json!({
                        "property": field,
                        "expected_type": expected_type,
                        "actual_type": json_type_name(actual_value),
                    }),
                });
            }
        }
    }

    violations
}

fn validate_name(name: &str) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::BadRequest("Name cannot be empty".to_string()));
    }
    Ok(())
}

fn validate_event_name(event_name: &str) -> AppResult<()> {
    if event_name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Event name cannot be empty".to_string(),
        ));
    }
    if event_name.len() > 255 {
        return Err(AppError::BadRequest(
            "Event name must be 255 characters or fewer".to_string(),
        ));
    }
    Ok(())
}

fn validate_enforcement_mode(mode: &str) -> AppResult<()> {
    match mode {
        "observe" | "reject" => Ok(()),
        _ => Err(AppError::BadRequest(
            "enforcement_mode must be 'observe' or 'reject'".to_string(),
        )),
    }
}

fn validate_schema_status(status: &str) -> AppResult<()> {
    match status {
        "draft" | "approved" | "deprecated" => Ok(()),
        _ => Err(AppError::BadRequest(
            "status must be 'draft', 'approved', or 'deprecated'".to_string(),
        )),
    }
}

fn validate_dictionary_entry(entry_type: &str, name: &str) -> AppResult<()> {
    validate_name(name)?;
    match entry_type {
        "event" | "property" | "metric" | "dimension" => Ok(()),
        _ => Err(AppError::BadRequest(
            "entry_type must be 'event', 'property', 'metric', or 'dimension'".to_string(),
        )),
    }
}

fn validate_property_schema(property_schema: &serde_json::Value) -> AppResult<()> {
    let Some(properties) = property_schema.as_object() else {
        return Err(AppError::BadRequest(
            "property_schema must be a JSON object".to_string(),
        ));
    };

    for (field, rule) in properties {
        if field.trim().is_empty() {
            return Err(AppError::BadRequest(
                "property_schema fields cannot be empty".to_string(),
            ));
        }
        let Some(expected_type) = expected_type_from_rule(rule) else {
            return Err(AppError::BadRequest(format!(
                "property_schema field '{field}' must be a string type or an object with a type"
            )));
        };
        if !is_supported_type(expected_type) {
            return Err(AppError::BadRequest(format!(
                "Unsupported property type '{expected_type}' for '{field}'"
            )));
        }
    }

    Ok(())
}

fn expected_type_from_rule(rule: &serde_json::Value) -> Option<&str> {
    if let Some(type_name) = rule.as_str() {
        return Some(type_name);
    }
    rule.as_object()
        .and_then(|obj| obj.get("type"))
        .and_then(|value| value.as_str())
}

fn rule_required(rule: &serde_json::Value) -> bool {
    rule.as_object()
        .and_then(|obj| obj.get("required"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn is_supported_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "any"
            | "string"
            | "number"
            | "integer"
            | "boolean"
            | "bool"
            | "object"
            | "array"
            | "null"
            | "timestamp"
            | "datetime"
    )
}

fn json_path_value<'a>(
    value: Option<&'a serde_json::Value>,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value?;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn value_matches_type(value: &serde_json::Value, expected_type: &str) -> bool {
    match expected_type {
        "any" => true,
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" | "bool" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "null" => value.is_null(),
        "timestamp" | "datetime" => value
            .as_str()
            .is_some_and(|s| DateTime::parse_from_rfc3339(s).is_ok()),
        _ => false,
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(n) if n.as_i64().is_some() || n.as_u64().is_some() => "integer",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_schema_violations, validate_property_schema, EventSchemaDefinition};

    fn schema() -> EventSchemaDefinition {
        EventSchemaDefinition {
            id: uuid::Uuid::nil(),
            project_id: uuid::Uuid::nil(),
            tracking_plan_id: None,
            event_name: "purchase".to_string(),
            description: None,
            status: "approved".to_string(),
            required_properties: vec!["amount".to_string()],
            property_schema: serde_json::json!({
                "amount": "number",
                "currency": { "type": "string", "required": true },
                "metadata.coupon": "string"
            }),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn validates_required_properties_and_nested_types() {
        let event_data = serde_json::json!({
            "amount": 42.5,
            "currency": "USD",
            "metadata": { "coupon": 123 }
        });

        let violations = collect_schema_violations(&schema(), Some(&event_data));

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, "property_type_mismatch");
    }

    #[test]
    fn reports_missing_required_properties() {
        let event_data = serde_json::json!({ "amount": 42.5 });

        let violations = collect_schema_violations(&schema(), Some(&event_data));

        assert!(violations
            .iter()
            .any(|v| v.violation_type == "missing_required_property"));
    }

    #[test]
    fn rejects_invalid_property_schema() {
        assert!(validate_property_schema(&serde_json::json!({"amount": "number"})).is_ok());
        assert!(validate_property_schema(&serde_json::json!({"amount": "currency"})).is_err());
        assert!(validate_property_schema(&serde_json::json!(["amount"])).is_err());
    }
}
