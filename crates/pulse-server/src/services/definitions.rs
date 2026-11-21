//! Event and property definitions (PostHog `EventDefinition` / `PropertyDefinition`)
//! and the events explorer.
//!
//! Names, volumes and property types are observed from the same virtual event
//! stream the insights engine queries (`insights::push_event_source`), so what
//! the definitions list shows is exactly what insights can query. Curated
//! metadata (description, tags, owner, verified, hidden, PII) is stored in
//! `data_dictionary_entries`, keyed by `(entry_type, name)`.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::insights::filters::{
    push_and_filters, validate_property, Joins, PropertyFilter,
};
use crate::services::insights::{
    load_profiles, map_query_error, push_event_source, EventSource, SESSION_PROPERTIES,
};

/// Default look-back for observed definitions.
pub const DEFAULT_DAYS: i64 = 30;
pub const MAX_DAYS: i64 = 365;
/// Most recent rows sampled to discover property keys and types.
const PROPERTY_SAMPLE_ROWS: i64 = 10_000;
/// Upper bound on event definitions returned.
const MAX_EVENT_DEFINITIONS: i64 = 1_000;
pub const DEFAULT_EXPLORER_LIMIT: i64 = 100;
pub const MAX_EXPLORER_LIMIT: i64 = 500;

/// What a definition describes; the `data_dictionary_entries.entry_type` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionKind {
    Event,
    EventProperty,
    PersonProperty,
    SessionProperty,
}

impl DefinitionKind {
    fn entry_type(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::EventProperty => "property",
            Self::PersonProperty => "person_property",
            Self::SessionProperty => "session_property",
        }
    }
}

/// Curated metadata shared by event and property definitions.
#[derive(Debug, Clone, Default, Serialize, sqlx::FromRow)]
pub struct Metadata {
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub verified: bool,
    pub hidden: bool,
    pub is_pii: bool,
}

#[derive(Debug, Serialize)]
pub struct EventDefinition {
    pub name: String,
    /// Events in the look-back window.
    pub volume: i64,
    pub unique_users: i64,
    pub last_seen_at: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub meta: Metadata,
}

#[derive(Debug, Serialize)]
pub struct PropertyDefinition {
    pub name: String,
    pub kind: DefinitionKind,
    /// `String`, `Numeric`, `Boolean`, `DateTime`, `Object` or `Array`.
    pub property_type: String,
    /// Rows in the sample that carry the property (`None` for session columns).
    pub seen: Option<i64>,
    #[serde(flatten)]
    pub meta: Metadata,
}

#[derive(Debug, Deserialize)]
pub struct DefinitionsQuery {
    pub search: Option<String>,
    pub days: Option<i64>,
    #[serde(default)]
    pub include_hidden: bool,
}

#[derive(Debug, Deserialize)]
pub struct PropertyDefinitionsQuery {
    #[serde(flatten)]
    pub base: DefinitionsQuery,
    /// `event_property` (default), `person_property` or `session_property`.
    pub kind: Option<DefinitionKind>,
    /// Only properties seen on this event.
    pub event: Option<String>,
}

/// Fields to change on a definition; omitted fields keep their value.
#[derive(Debug, Deserialize)]
pub struct DefinitionPatch {
    pub kind: DefinitionKind,
    pub name: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tags: Option<Vec<String>>,
    pub verified: Option<bool>,
    pub hidden: Option<bool>,
    pub is_pii: Option<bool>,
}

fn window(days: Option<i64>) -> AppResult<(DateTime<Utc>, DateTime<Utc>)> {
    let days = days.unwrap_or(DEFAULT_DAYS);
    if !(1..=MAX_DAYS).contains(&days) {
        return Err(AppError::BadRequest(format!("days must be 1-{MAX_DAYS}")));
    }
    let to = Utc::now();
    Ok((to - Duration::days(days), to))
}

fn matches(name: &str, meta: &Metadata, q: &DefinitionsQuery) -> bool {
    if meta.hidden && !q.include_hidden {
        return false;
    }
    match q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => true,
        Some(s) => {
            let s = s.to_lowercase();
            name.to_lowercase().contains(&s)
                || meta
                    .description
                    .as_deref()
                    .is_some_and(|d| d.to_lowercase().contains(&s))
                || meta.tags.iter().any(|t| t.to_lowercase().contains(&s))
        }
    }
}

async fn load_metadata(
    db: &PgPool,
    project_id: Uuid,
    kind: DefinitionKind,
) -> AppResult<HashMap<String, Metadata>> {
    let rows: Vec<(
        String,
        Option<String>,
        Option<String>,
        Vec<String>,
        bool,
        bool,
        bool,
    )> = sqlx::query_as(
        "SELECT name, description, owner, tags, verified, hidden, is_pii \
             FROM data_dictionary_entries WHERE project_id = $1 AND entry_type = $2",
    )
    .bind(project_id)
    .bind(kind.entry_type())
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(name, description, owner, tags, verified, hidden, is_pii)| {
                (
                    name,
                    Metadata {
                        description,
                        owner,
                        tags,
                        verified,
                        hidden,
                        is_pii,
                    },
                )
            },
        )
        .collect())
}

/// Events seen in the window plus any documented-but-unseen events, busiest first.
pub async fn list_event_definitions(
    db: &PgPool,
    project_id: Uuid,
    q: &DefinitionsQuery,
) -> AppResult<Vec<EventDefinition>> {
    let (from, to) = window(q.days)?;
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT ev.event, COUNT(*)::bigint, COUNT(DISTINCT ev.visitor_id)::bigint, MAX(ev.created_at)",
    );
    push_event_source(
        &mut qb,
        &EventSource {
            project_id,
            from,
            to,
            events: None,
            joins: Joins::default(),
        },
    );
    qb.push(" GROUP BY ev.event ORDER BY 2 DESC LIMIT ");
    qb.push_bind(MAX_EVENT_DEFINITIONS);
    let seen: Vec<(String, i64, i64, DateTime<Utc>)> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;

    let mut meta = load_metadata(db, project_id, DefinitionKind::Event).await?;
    let mut out: Vec<EventDefinition> = seen
        .into_iter()
        .map(|(name, volume, unique_users, last)| {
            let m = meta.remove(&name).unwrap_or_default();
            EventDefinition {
                name,
                volume,
                unique_users,
                last_seen_at: Some(last),
                meta: m,
            }
        })
        .collect();
    let mut documented: Vec<EventDefinition> = meta
        .into_iter()
        .map(|(name, m)| EventDefinition {
            name,
            volume: 0,
            unique_users: 0,
            last_seen_at: None,
            meta: m,
        })
        .collect();
    documented.sort_by(|a, b| a.name.cmp(&b.name));
    out.extend(documented);
    out.retain(|d| matches(&d.name, &d.meta, q));
    Ok(out)
}

/// Map `jsonb_typeof` (plus a date check on strings) to a PostHog property type.
const TYPE_EXPR: &str = "CASE jsonb_typeof(kv.value) \
    WHEN 'number' THEN 'Numeric' WHEN 'boolean' THEN 'Boolean' \
    WHEN 'object' THEN 'Object' WHEN 'array' THEN 'Array' \
    WHEN 'string' THEN CASE WHEN kv.value #>> '{}' ~ '^\\d{4}-\\d{2}-\\d{2}([T ]\\d{2}:\\d{2})?' \
        THEN 'DateTime' ELSE 'String' END \
    ELSE 'String' END";

/// Session columns and their types (they are columns, not JSON).
fn session_property_type(name: &str) -> &'static str {
    match name {
        "duration_ms" | "pageview_count" | "event_count" => "Numeric",
        "is_bounce" => "Boolean",
        _ => "String",
    }
}

/// Collapse `(key, type, count)` rows to one dominant type per key, most-seen first.
fn dominant_types(rows: Vec<(String, String, i64)>) -> Vec<(String, String, i64)> {
    let mut by_key: HashMap<String, (String, i64, i64)> = HashMap::new();
    for (key, ty, n) in rows {
        let e = by_key.entry(key).or_insert_with(|| (ty.clone(), 0, 0));
        e.2 += n;
        if n > e.1 {
            e.0 = ty;
            e.1 = n;
        }
    }
    let mut out: Vec<_> = by_key
        .into_iter()
        .map(|(k, (ty, _, total))| (k, ty, total))
        .collect();
    out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    out
}

pub async fn list_property_definitions(
    db: &PgPool,
    project_id: Uuid,
    q: &PropertyDefinitionsQuery,
) -> AppResult<Vec<PropertyDefinition>> {
    let kind = q.kind.unwrap_or(DefinitionKind::EventProperty);
    let observed: Vec<(String, String, Option<i64>)> = match kind {
        DefinitionKind::Event => {
            return Err(AppError::BadRequest("kind must be a property kind".into()))
        }
        DefinitionKind::SessionProperty => SESSION_PROPERTIES
            .iter()
            .map(|p| (p.to_string(), session_property_type(p).to_string(), None))
            .collect(),
        DefinitionKind::EventProperty => {
            let (from, to) = window(q.base.days)?;
            let only = q.event.clone().map(|e| vec![e]);
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(format!(
                "SELECT kv.key, {TYPE_EXPR}, COUNT(*)::bigint FROM (SELECT ev.properties"
            ));
            push_event_source(
                &mut qb,
                &EventSource {
                    project_id,
                    from,
                    to,
                    events: only.as_deref(),
                    joins: Joins::default(),
                },
            );
            qb.push(" ORDER BY ev.created_at DESC LIMIT ");
            qb.push_bind(PROPERTY_SAMPLE_ROWS);
            qb.push(") s CROSS JOIN LATERAL jsonb_each(s.properties) kv GROUP BY 1, 2");
            let rows: Vec<(String, String, i64)> = qb
                .build_query_as()
                .fetch_all(db)
                .await
                .map_err(map_query_error)?;
            dominant_types(rows)
                .into_iter()
                .map(|(k, t, n)| (k, t, Some(n)))
                .collect()
        }
        DefinitionKind::PersonProperty => {
            let rows: Vec<(String, String, i64)> = sqlx::query_as(&format!(
                "SELECT kv.key, {TYPE_EXPR}, COUNT(*)::bigint FROM \
                 (SELECT traits FROM user_profiles WHERE project_id = $1 \
                  AND jsonb_typeof(traits) = 'object' ORDER BY updated_at DESC LIMIT $2) s \
                 CROSS JOIN LATERAL jsonb_each(s.traits) kv GROUP BY 1, 2"
            ))
            .bind(project_id)
            .bind(PROPERTY_SAMPLE_ROWS)
            .fetch_all(db)
            .await?;
            dominant_types(rows)
                .into_iter()
                .map(|(k, t, n)| (k, t, Some(n)))
                .collect()
        }
    };

    let mut meta = load_metadata(db, project_id, kind).await?;
    let mut out: Vec<PropertyDefinition> = observed
        .into_iter()
        .map(|(name, property_type, seen)| {
            let m = meta.remove(&name).unwrap_or_default();
            PropertyDefinition {
                name,
                kind,
                property_type,
                seen,
                meta: m,
            }
        })
        .collect();
    // Documented-but-unseen properties only make sense without an event filter.
    if q.event.is_none() {
        let mut documented: Vec<PropertyDefinition> = meta
            .into_iter()
            .map(|(name, m)| PropertyDefinition {
                name,
                kind,
                property_type: "String".into(),
                seen: Some(0),
                meta: m,
            })
            .collect();
        documented.sort_by(|a, b| a.name.cmp(&b.name));
        out.extend(documented);
    }
    out.retain(|d| matches(&d.name, &d.meta, &q.base));
    Ok(out)
}

/// Create or update a definition's metadata.
pub async fn upsert_definition(
    db: &PgPool,
    project_id: Uuid,
    p: &DefinitionPatch,
) -> AppResult<Metadata> {
    let name = p.name.trim();
    if name.is_empty() || name.len() > 255 {
        return Err(AppError::BadRequest("name must be 1-255 characters".into()));
    }
    let tags = p.tags.as_ref().map(|t| {
        let mut t: Vec<String> = t
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        t.sort();
        t.dedup();
        t
    });
    let row: Metadata = sqlx::query_as(
        "INSERT INTO data_dictionary_entries \
            (project_id, entry_type, name, description, owner, tags, verified, hidden, is_pii) \
         VALUES ($1, $2, $3, $4, $5, COALESCE($6, '{}'), COALESCE($7, false), COALESCE($8, false), COALESCE($9, false)) \
         ON CONFLICT (project_id, entry_type, name) DO UPDATE SET \
            description = COALESCE($4, data_dictionary_entries.description), \
            owner = COALESCE($5, data_dictionary_entries.owner), \
            tags = COALESCE($6, data_dictionary_entries.tags), \
            verified = COALESCE($7, data_dictionary_entries.verified), \
            hidden = COALESCE($8, data_dictionary_entries.hidden), \
            is_pii = COALESCE($9, data_dictionary_entries.is_pii), \
            updated_at = NOW() \
         RETURNING description, owner, tags, verified, hidden, is_pii",
    )
    .bind(project_id)
    .bind(p.kind.entry_type())
    .bind(name)
    .bind(&p.description)
    .bind(&p.owner)
    .bind(tags)
    .bind(p.verified)
    .bind(p.hidden)
    .bind(p.is_pii)
    .fetch_one(db)
    .await?;
    Ok(row)
}

// ── Events explorer ──

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    /// Only this event (`$pageview` for pageviews); all events when omitted.
    pub event: Option<String>,
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    pub date_from: Option<DateTime<Utc>>,
    /// Exclusive upper bound; pass the last row's `created_at` to page back.
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

impl EventsQuery {
    pub fn joins(&self) -> Joins {
        let mut j = Joins::default();
        j.add_filters(&self.properties);
        j
    }
}

#[derive(Debug, Serialize)]
pub struct ExplorerEvent {
    pub event: String,
    pub visitor_id: String,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub properties: serde_json::Value,
    /// Present when the caller may read person data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<ExplorerPerson>,
}

#[derive(Debug, Serialize)]
pub struct ExplorerPerson {
    pub user_id: Option<String>,
    pub traits: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct EventsPage {
    pub results: Vec<ExplorerEvent>,
    /// Pass as `before` to load older events; `None` when there are no more.
    pub next_before: Option<DateTime<Utc>>,
}

/// Newest-first raw events, optionally filtered by event name and properties.
pub async fn query_events(
    db: &PgPool,
    project_id: Uuid,
    q: &EventsQuery,
    include_persons: bool,
) -> AppResult<EventsPage> {
    for f in &q.properties {
        validate_property(&f.key, f.property_type, f.group_type_index)?;
    }
    let limit = q
        .limit
        .unwrap_or(DEFAULT_EXPLORER_LIMIT)
        .clamp(1, MAX_EXPLORER_LIMIT);
    let to = q.before.unwrap_or_else(Utc::now);
    let from = q.date_from.unwrap_or(to - Duration::days(DEFAULT_DAYS));
    if from >= to {
        return Err(AppError::BadRequest(
            "date_from must be before `before`".into(),
        ));
    }
    let only = q.event.clone().map(|e| vec![e]);

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT ev.event, ev.visitor_id, ev.session_id, ev.created_at, ev.properties",
    );
    push_event_source(
        &mut qb,
        &EventSource {
            project_id,
            from,
            to,
            events: only.as_deref(),
            joins: q.joins(),
        },
    );
    qb.push(" WHERE TRUE");
    push_and_filters(&mut qb, &q.properties)?;
    qb.push(" ORDER BY ev.created_at DESC LIMIT ");
    qb.push_bind(limit + 1);

    let mut rows: Vec<(
        String,
        String,
        Option<Uuid>,
        DateTime<Utc>,
        serde_json::Value,
    )> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    let more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);

    let mut profiles = if include_persons {
        let mut ids: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();
        ids.sort();
        ids.dedup();
        load_profiles(db, project_id, &ids).await?
    } else {
        HashMap::new()
    };
    let next_before = if more { rows.last().map(|r| r.3) } else { None };
    let results = rows
        .into_iter()
        .map(|(event, visitor_id, session_id, created_at, properties)| {
            let person = include_persons.then(|| {
                let (user_id, traits) = profiles
                    .remove(&visitor_id)
                    .unwrap_or((None, serde_json::Value::Null));
                ExplorerPerson { user_id, traits }
            });
            ExplorerEvent {
                event,
                visitor_id,
                session_id,
                created_at,
                properties,
                person,
            }
        })
        .collect();
    Ok(EventsPage {
        results,
        next_before,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominant_type_wins_and_counts_sum() {
        let rows = vec![
            ("plan".into(), "String".into(), 90),
            ("plan".into(), "Numeric".into(), 10),
            ("seats".into(), "Numeric".into(), 40),
        ];
        assert_eq!(
            dominant_types(rows),
            vec![
                ("plan".into(), "String".into(), 100),
                ("seats".into(), "Numeric".into(), 40)
            ]
        );
    }

    #[test]
    fn search_matches_name_description_and_tags_and_hides_hidden() {
        let q = |s: &str, hidden: bool| DefinitionsQuery {
            search: Some(s.into()),
            days: None,
            include_hidden: hidden,
        };
        let m = Metadata {
            description: Some("User finished signup".into()),
            tags: vec!["growth".into()],
            ..Default::default()
        };
        assert!(matches("signup_completed", &m, &q("SIGNUP", false)));
        assert!(matches("x", &m, &q("finished", false)));
        assert!(matches("x", &m, &q("grow", false)));
        assert!(!matches("x", &m, &q("billing", false)));
        let hidden = Metadata {
            hidden: true,
            ..Default::default()
        };
        assert!(!matches("x", &hidden, &q("", false)));
        assert!(matches("x", &hidden, &q("", true)));
    }

    #[test]
    fn window_bounds_days() {
        assert!(window(Some(0)).is_err());
        assert!(window(Some(MAX_DAYS + 1)).is_err());
        let (from, to) = window(None).unwrap();
        assert_eq!((to - from).num_days(), DEFAULT_DAYS);
    }

    #[test]
    fn session_types() {
        assert_eq!(session_property_type("duration_ms"), "Numeric");
        assert_eq!(session_property_type("is_bounce"), "Boolean");
        assert_eq!(session_property_type("browser"), "String");
    }
}
