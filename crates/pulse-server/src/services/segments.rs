#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedSegment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentDefinition {
    #[serde(default = "default_match_type", rename = "match")]
    pub match_type: String,
    #[serde(default)]
    pub conditions: Vec<SegmentCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentCondition {
    pub source: String,
    #[serde(default)]
    pub field: Option<String>,
    pub op: String,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub event: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SegmentEvaluation {
    pub segment_id: Uuid,
    pub total_visitors: usize,
    pub visitors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SegmentCompareRow {
    pub segment_id: Uuid,
    pub name: String,
    pub visitors: usize,
    pub pageviews: i64,
    pub sessions: i64,
    pub events: i64,
    pub conversions: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SegmentBreakdownRow {
    pub value: String,
    pub visitors: i64,
}

#[derive(Debug, Clone, Default)]
struct VisitorContext {
    visitor_id: String,
    user_id: Option<String>,
    traits: serde_json::Value,
    sessions: Vec<SessionContext>,
    pageviews: Vec<PageviewContext>,
    events: Vec<EventContext>,
}

#[derive(Debug, Clone, Default)]
struct SessionContext {
    country: Option<String>,
    browser: Option<String>,
    os: Option<String>,
    device: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct PageviewContext {
    path: String,
    referrer_domain: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct EventContext {
    name: String,
    data: Option<serde_json::Value>,
    path: Option<String>,
}

const SEGMENT_COLUMNS: &str = "id, project_id, name, description, definition, is_active, \
    created_at, updated_at";

fn default_match_type() -> String {
    "all".to_string()
}

pub async fn create_segment(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    definition: serde_json::Value,
) -> AppResult<SavedSegment> {
    validate_definition(&definition)?;

    let segment: SavedSegment = sqlx::query_as(&format!(
        "INSERT INTO saved_segments (project_id, name, description, definition) \
         VALUES ($1, $2, $3, $4) RETURNING {SEGMENT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name)
    .bind(description)
    .bind(definition)
    .fetch_one(db)
    .await?;

    Ok(segment)
}

pub async fn list_segments(db: &PgPool, project_id: Uuid) -> AppResult<Vec<SavedSegment>> {
    let segments: Vec<SavedSegment> = sqlx::query_as(&format!(
        "SELECT {SEGMENT_COLUMNS} FROM saved_segments \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(segments)
}

pub async fn get_segment(
    db: &PgPool,
    project_id: Uuid,
    segment_id: Uuid,
) -> AppResult<SavedSegment> {
    let segment: Option<SavedSegment> = sqlx::query_as(&format!(
        "SELECT {SEGMENT_COLUMNS} FROM saved_segments WHERE id = $1 AND project_id = $2"
    ))
    .bind(segment_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    segment.ok_or_else(|| AppError::NotFound("Segment not found".to_string()))
}

pub async fn update_segment(
    db: &PgPool,
    project_id: Uuid,
    segment_id: Uuid,
    name: &str,
    description: Option<&str>,
    definition: serde_json::Value,
    is_active: bool,
) -> AppResult<SavedSegment> {
    validate_definition(&definition)?;

    let segment: Option<SavedSegment> = sqlx::query_as(&format!(
        "UPDATE saved_segments SET name = $1, description = $2, definition = $3, \
         is_active = $4, updated_at = NOW() \
         WHERE id = $5 AND project_id = $6 RETURNING {SEGMENT_COLUMNS}"
    ))
    .bind(name)
    .bind(description)
    .bind(definition)
    .bind(is_active)
    .bind(segment_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    segment.ok_or_else(|| AppError::NotFound("Segment not found".to_string()))
}

pub async fn delete_segment(db: &PgPool, project_id: Uuid, segment_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM saved_segments WHERE id = $1 AND project_id = $2")
        .bind(segment_id)
        .bind(project_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Segment not found".to_string()));
    }

    Ok(())
}

pub async fn evaluate_segment(
    db: &PgPool,
    project_id: Uuid,
    segment_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: usize,
    offset: usize,
) -> AppResult<SegmentEvaluation> {
    let segment = get_segment(db, project_id, segment_id).await?;
    let visitors = evaluate_definition(db, project_id, &segment.definition, start, end).await?;
    let total_visitors = visitors.len();
    let visitors = visitors
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();

    Ok(SegmentEvaluation {
        segment_id,
        total_visitors,
        visitors,
    })
}

pub async fn compare_segments(
    db: &PgPool,
    project_id: Uuid,
    segment_ids: &[Uuid],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<Vec<SegmentCompareRow>> {
    let mut rows = Vec::new();

    for segment_id in segment_ids {
        let segment = get_segment(db, project_id, *segment_id).await?;
        let visitors = evaluate_definition(db, project_id, &segment.definition, start, end).await?;
        let metrics = segment_metrics(db, project_id, &visitors, start, end).await?;
        rows.push(SegmentCompareRow {
            segment_id: *segment_id,
            name: segment.name,
            visitors: visitors.len(),
            pageviews: metrics.0,
            sessions: metrics.1,
            events: metrics.2,
            conversions: metrics.3,
        });
    }

    Ok(rows)
}

pub async fn breakdown_segment(
    db: &PgPool,
    project_id: Uuid,
    segment_id: Uuid,
    property: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> AppResult<Vec<SegmentBreakdownRow>> {
    let segment = get_segment(db, project_id, segment_id).await?;
    let visitors = evaluate_definition(db, project_id, &segment.definition, start, end).await?;

    if visitors.is_empty() {
        return Ok(Vec::new());
    }

    match property {
        "country" | "browser" | "os" | "device" => {
            breakdown_sessions(db, project_id, &visitors, property, start, end, limit).await
        }
        "path" => breakdown_pageviews(db, project_id, &visitors, start, end, limit).await,
        "event" | "event_name" => {
            breakdown_events(db, project_id, &visitors, start, end, limit).await
        }
        p if p.starts_with("trait:") => {
            breakdown_trait(
                db,
                project_id,
                &visitors,
                p.trim_start_matches("trait:"),
                limit,
            )
            .await
        }
        _ => Err(AppError::BadRequest(format!(
            "Unsupported segment breakdown property: {property}"
        ))),
    }
}

async fn evaluate_definition(
    db: &PgPool,
    project_id: Uuid,
    definition: &serde_json::Value,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<Vec<String>> {
    let definition: SegmentDefinition = serde_json::from_value(definition.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid segment definition: {e}")))?;
    let contexts = load_visitor_contexts(db, project_id, start, end).await?;
    let mut matched = Vec::new();

    for context in contexts.values() {
        if matches_definition(context, &definition) {
            matched.push(context.visitor_id.clone());
        }
    }

    matched.sort();
    Ok(matched)
}

async fn load_visitor_contexts(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<HashMap<String, VisitorContext>> {
    let mut contexts = HashMap::new();

    let profiles: Vec<(String, Option<String>, serde_json::Value)> = sqlx::query_as(
        "SELECT visitor_id, user_id, traits FROM user_profiles WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    for (visitor_id, user_id, traits) in profiles {
        let context = contexts
            .entry(visitor_id.clone())
            .or_insert_with(|| VisitorContext {
                visitor_id,
                ..Default::default()
            });
        context.user_id = user_id;
        context.traits = traits;
    }

    let sessions: Vec<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT visitor_id, country, browser, os, device FROM sessions \
         WHERE project_id = $1 AND first_at >= $2 AND first_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;
    for (visitor_id, country, browser, os, device) in sessions {
        contexts
            .entry(visitor_id.clone())
            .or_insert_with(|| VisitorContext {
                visitor_id,
                ..Default::default()
            })
            .sessions
            .push(SessionContext {
                country,
                browser,
                os,
                device,
            });
    }

    let pageviews: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT visitor_id, path, referrer_domain FROM pageviews \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;
    for (visitor_id, path, referrer_domain) in pageviews {
        contexts
            .entry(visitor_id.clone())
            .or_insert_with(|| VisitorContext {
                visitor_id,
                ..Default::default()
            })
            .pageviews
            .push(PageviewContext {
                path,
                referrer_domain,
            });
    }

    let events: Vec<(String, String, Option<serde_json::Value>, Option<String>)> = sqlx::query_as(
        "SELECT visitor_id, event_name, event_data, path FROM events \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;
    for (visitor_id, name, data, path) in events {
        contexts
            .entry(visitor_id.clone())
            .or_insert_with(|| VisitorContext {
                visitor_id,
                ..Default::default()
            })
            .events
            .push(EventContext { name, data, path });
    }

    Ok(contexts)
}

fn validate_definition(definition: &serde_json::Value) -> AppResult<()> {
    let parsed: SegmentDefinition = serde_json::from_value(definition.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid segment definition: {e}")))?;
    if parsed.match_type != "all" && parsed.match_type != "any" {
        return Err(AppError::BadRequest(
            "Segment match must be 'all' or 'any'".to_string(),
        ));
    }
    Ok(())
}

fn matches_definition(context: &VisitorContext, definition: &SegmentDefinition) -> bool {
    if definition.conditions.is_empty() {
        return true;
    }
    if definition.match_type == "any" {
        definition
            .conditions
            .iter()
            .any(|condition| matches_condition(context, condition))
    } else {
        definition
            .conditions
            .iter()
            .all(|condition| matches_condition(context, condition))
    }
}

fn matches_condition(context: &VisitorContext, condition: &SegmentCondition) -> bool {
    match condition.source.as_str() {
        "profile" | "user" | "identity" => match_profile_condition(context, condition),
        "session" => context.sessions.iter().any(|session| {
            compare_any(
                session_value(session, condition.field.as_deref().unwrap_or("country")),
                condition,
            )
        }),
        "pageview" => context.pageviews.iter().any(|pageview| {
            compare_any(
                pageview_value(pageview, condition.field.as_deref().unwrap_or("path")),
                condition,
            )
        }),
        "event" => context.events.iter().any(|event| {
            if let Some(target_event) = &condition.event {
                if &event.name != target_event {
                    return false;
                }
            }
            compare_any(
                event_value(event, condition.field.as_deref().unwrap_or("event_name")),
                condition,
            )
        }),
        "metric" => match_metric_condition(context, condition),
        _ => false,
    }
}

fn match_profile_condition(context: &VisitorContext, condition: &SegmentCondition) -> bool {
    let field = condition.field.as_deref().unwrap_or("user_id");
    let value = if field == "user_id" {
        context.user_id.as_ref().map(|v| serde_json::json!(v))
    } else {
        let path = field
            .strip_prefix("traits.")
            .or_else(|| field.strip_prefix("trait:"))
            .unwrap_or(field);
        json_path_value(&context.traits, path).cloned()
    };
    compare_value(value.as_ref(), condition)
}

fn match_metric_condition(context: &VisitorContext, condition: &SegmentCondition) -> bool {
    let metric = condition.field.as_deref().unwrap_or("");
    let value = match metric {
        "sessions" => context.sessions.len() as f64,
        "pageviews" => context.pageviews.len() as f64,
        "events" => context.events.len() as f64,
        _ => 0.0,
    };
    compare_value(Some(&serde_json::json!(value)), condition)
}

fn session_value(session: &SessionContext, field: &str) -> Option<serde_json::Value> {
    match field {
        "country" => session.country.clone().map(serde_json::Value::String),
        "browser" => session.browser.clone().map(serde_json::Value::String),
        "os" => session.os.clone().map(serde_json::Value::String),
        "device" => session.device.clone().map(serde_json::Value::String),
        _ => None,
    }
}

fn pageview_value(pageview: &PageviewContext, field: &str) -> Option<serde_json::Value> {
    match field {
        "path" => Some(serde_json::json!(pageview.path)),
        "referrer_domain" => pageview
            .referrer_domain
            .clone()
            .map(serde_json::Value::String),
        _ => None,
    }
}

fn event_value(event: &EventContext, field: &str) -> Option<serde_json::Value> {
    match field {
        "name" | "event_name" => Some(serde_json::json!(event.name)),
        "path" => event.path.clone().map(serde_json::Value::String),
        f if f.starts_with("event_data.") => event
            .data
            .as_ref()
            .and_then(|data| json_path_value(data, f.trim_start_matches("event_data.")))
            .cloned(),
        f if f.starts_with("data.") => event
            .data
            .as_ref()
            .and_then(|data| json_path_value(data, f.trim_start_matches("data.")))
            .cloned(),
        _ => None,
    }
}

fn compare_any(value: Option<serde_json::Value>, condition: &SegmentCondition) -> bool {
    compare_value(value.as_ref(), condition)
}

fn compare_value(value: Option<&serde_json::Value>, condition: &SegmentCondition) -> bool {
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

fn compare_numeric(value: Option<&serde_json::Value>, condition: &SegmentCondition) -> bool {
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

fn condition_string(condition: &SegmentCondition) -> Option<String> {
    condition.value.as_ref().and_then(value_as_string)
}

fn json_path_value<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

async fn segment_metrics(
    db: &PgPool,
    project_id: Uuid,
    visitors: &[String],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<(i64, i64, i64, i64)> {
    if visitors.is_empty() {
        return Ok((0, 0, 0, 0));
    }

    let pageviews: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, COUNT(DISTINCT session_id)::bigint FROM pageviews \
         WHERE project_id = $1 AND visitor_id = ANY($2) AND created_at >= $3 AND created_at <= $4",
    )
    .bind(project_id)
    .bind(visitors)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;
    let events: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM events \
         WHERE project_id = $1 AND visitor_id = ANY($2) AND created_at >= $3 AND created_at <= $4",
    )
    .bind(project_id)
    .bind(visitors)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;
    let conversions: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM goal_conversions \
         WHERE project_id = $1 AND visitor_id = ANY($2) AND created_at >= $3 AND created_at <= $4",
    )
    .bind(project_id)
    .bind(visitors)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    Ok((pageviews.0, pageviews.1, events.0, conversions.0))
}

async fn breakdown_sessions(
    db: &PgPool,
    project_id: Uuid,
    visitors: &[String],
    property: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> AppResult<Vec<SegmentBreakdownRow>> {
    let column = match property {
        "country" => "country",
        "browser" => "browser",
        "os" => "os",
        "device" => "device",
        _ => unreachable!(),
    };
    let sql = format!(
        "SELECT COALESCE({column}, 'Unknown') AS value, COUNT(DISTINCT visitor_id)::bigint \
         FROM sessions \
         WHERE project_id = $1 AND visitor_id = ANY($2) AND first_at >= $3 AND first_at <= $4 \
         GROUP BY value ORDER BY 2 DESC LIMIT $5"
    );
    let rows: Vec<(String, i64)> = sqlx::query_as(&sql)
        .bind(project_id)
        .bind(visitors)
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(value, visitors)| SegmentBreakdownRow { value, visitors })
        .collect())
}

async fn breakdown_pageviews(
    db: &PgPool,
    project_id: Uuid,
    visitors: &[String],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> AppResult<Vec<SegmentBreakdownRow>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT path, COUNT(DISTINCT visitor_id)::bigint FROM pageviews \
         WHERE project_id = $1 AND visitor_id = ANY($2) AND created_at >= $3 AND created_at <= $4 \
         GROUP BY path ORDER BY 2 DESC LIMIT $5",
    )
    .bind(project_id)
    .bind(visitors)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(value, visitors)| SegmentBreakdownRow { value, visitors })
        .collect())
}

async fn breakdown_events(
    db: &PgPool,
    project_id: Uuid,
    visitors: &[String],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> AppResult<Vec<SegmentBreakdownRow>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT event_name, COUNT(DISTINCT visitor_id)::bigint FROM events \
         WHERE project_id = $1 AND visitor_id = ANY($2) AND created_at >= $3 AND created_at <= $4 \
         GROUP BY event_name ORDER BY 2 DESC LIMIT $5",
    )
    .bind(project_id)
    .bind(visitors)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(value, visitors)| SegmentBreakdownRow { value, visitors })
        .collect())
}

async fn breakdown_trait(
    db: &PgPool,
    project_id: Uuid,
    visitors: &[String],
    trait_name: &str,
    limit: i64,
) -> AppResult<Vec<SegmentBreakdownRow>> {
    let profiles: Vec<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT visitor_id, traits FROM user_profiles WHERE project_id = $1 AND visitor_id = ANY($2)",
    )
    .bind(project_id)
    .bind(visitors)
    .fetch_all(db)
    .await?;
    let mut counts: HashMap<String, HashSet<String>> = HashMap::new();
    for (visitor, traits) in profiles {
        let value = json_path_value(&traits, trait_name)
            .and_then(value_as_string)
            .unwrap_or_else(|| "Unknown".to_string());
        counts.entry(value).or_default().insert(visitor);
    }
    let mut rows: Vec<SegmentBreakdownRow> = counts
        .into_iter()
        .map(|(value, visitors)| SegmentBreakdownRow {
            value,
            visitors: visitors.len() as i64,
        })
        .collect();
    rows.sort_by(|a, b| b.visitors.cmp(&a.visitors));
    rows.truncate(limit as usize);
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::{matches_definition, SegmentCondition, SegmentDefinition, VisitorContext};

    #[test]
    fn matches_profile_trait_condition() {
        let context = VisitorContext {
            visitor_id: "v1".to_string(),
            traits: serde_json::json!({ "plan": "pro" }),
            ..Default::default()
        };
        let definition = SegmentDefinition {
            match_type: "all".to_string(),
            conditions: vec![SegmentCondition {
                source: "profile".to_string(),
                field: Some("traits.plan".to_string()),
                op: "eq".to_string(),
                value: Some(serde_json::json!("pro")),
                event: None,
            }],
        };

        assert!(matches_definition(&context, &definition));
    }

    #[test]
    fn supports_metric_threshold_conditions() {
        let context = VisitorContext {
            visitor_id: "v1".to_string(),
            pageviews: vec![Default::default(), Default::default()],
            ..Default::default()
        };
        let definition = SegmentDefinition {
            match_type: "all".to_string(),
            conditions: vec![SegmentCondition {
                source: "metric".to_string(),
                field: Some("pageviews".to_string()),
                op: "gte".to_string(),
                value: Some(serde_json::json!(2)),
                event: None,
            }],
        };

        assert!(matches_definition(&context, &definition));
    }
}
