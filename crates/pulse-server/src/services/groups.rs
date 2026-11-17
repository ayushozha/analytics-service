//! Group analytics (PostHog groups): up to five group types per project
//! (company, workspace, …), each mapped to a fixed index on first use.
//!
//! Events are tied to groups at ingestion: a `{type: key}` map (the envelope's
//! `groups`, or `$groups` in event properties) is stored on the event as
//! `$group_<index>: key`, so insights filter, break down and count unique
//! groups without resolving type names. Group properties are set with a
//! `$groupidentify` event (`$group_type`, `$group_key`, `$group_set`) or the API.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::identity::PersonUpdate;
use crate::services::insights::filters::{push_filter, PropertyFilter};
use crate::services::insights::{map_query_error, push_event_source, EventSource, PropertyType};

/// Group types per project (indices 0-4), as in PostHog.
pub const MAX_GROUP_TYPES: i16 = 5;
pub const MAX_GROUP_TYPE_LEN: usize = 100;
pub const MAX_GROUP_KEY_LEN: usize = 400;
/// Event property holding a `{group_type: group_key}` map.
pub const GROUPS_PROPERTY: &str = "$groups";
/// Event that sets a group's properties.
pub const GROUP_IDENTIFY_EVENT: &str = "$groupidentify";

fn bad(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}

/// Event property that stores the key of the group of type `index`.
pub fn key_property(index: i16) -> String {
    format!("$group_{index}")
}

fn check_index(index: i16) -> AppResult<()> {
    if (0..MAX_GROUP_TYPES).contains(&index) {
        Ok(())
    } else {
        Err(bad(format!(
            "group_type_index must be 0-{}",
            MAX_GROUP_TYPES - 1
        )))
    }
}

/// Indices never change once assigned, so resolved types are cached in-process.
fn type_cache() -> &'static RwLock<HashMap<(Uuid, String), i16>> {
    static CACHE: OnceLock<RwLock<HashMap<(Uuid, String), i16>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Index of a group type, assigning the lowest free index when `create` is
/// set. `None` when the type is unknown, or all five indices are taken.
pub async fn type_index(
    db: &PgPool,
    project_id: Uuid,
    group_type: &str,
    create: bool,
) -> AppResult<Option<i16>> {
    let cache_key = (project_id, group_type.to_string());
    if let Some(i) = type_cache()
        .read()
        .ok()
        .and_then(|c| c.get(&cache_key).copied())
    {
        return Ok(Some(i));
    }
    if group_type.is_empty() || group_type.len() > MAX_GROUP_TYPE_LEN {
        return Err(bad(format!(
            "group type must be 1-{MAX_GROUP_TYPE_LEN} characters"
        )));
    }
    // A concurrent insert of another type can take the same free index; the
    // loser's insert is skipped by ON CONFLICT and the next attempt retries.
    for _ in 0..MAX_GROUP_TYPES {
        let found: Option<(i16,)> = sqlx::query_as(
            "SELECT group_type_index FROM group_type_mappings WHERE project_id = $1 AND group_type = $2",
        )
        .bind(project_id)
        .bind(group_type)
        .fetch_optional(db)
        .await?;
        if let Some((i,)) = found {
            if let Ok(mut c) = type_cache().write() {
                c.insert(cache_key, i);
            }
            return Ok(Some(i));
        }
        if !create {
            return Ok(None);
        }
        let inserted = sqlx::query(
            "INSERT INTO group_type_mappings (project_id, group_type_index, group_type) \
             SELECT $1, i, $2 FROM generate_series(0, $3 - 1) i \
             WHERE NOT EXISTS (SELECT 1 FROM group_type_mappings m \
                               WHERE m.project_id = $1 AND m.group_type_index = i) \
             ORDER BY i LIMIT 1 ON CONFLICT DO NOTHING",
        )
        .bind(project_id)
        .bind(group_type)
        .bind(MAX_GROUP_TYPES as i32)
        .execute(db)
        .await?
        .rows_affected();
        if inserted == 0 {
            let (n,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM group_type_mappings WHERE project_id = $1")
                    .bind(project_id)
                    .fetch_one(db)
                    .await?;
            if n >= MAX_GROUP_TYPES as i64 {
                return Ok(None);
            }
        }
    }
    Ok(None)
}

/// Turn `{group_type: key}` into `{"$group_<index>": key}`, creating types as
/// needed. Types beyond the limit and non-scalar keys are dropped.
pub async fn stamp(
    db: &PgPool,
    project_id: Uuid,
    groups: &Map<String, Value>,
) -> AppResult<Map<String, Value>> {
    let mut out = Map::new();
    for (group_type, key) in groups {
        let key = match key {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => continue,
        };
        if key.is_empty() || key.len() > MAX_GROUP_KEY_LEN {
            continue;
        }
        match type_index(db, project_id, group_type, true).await {
            Ok(Some(i)) => {
                out.insert(key_property(i), Value::String(key));
            }
            Ok(None) => tracing::warn!(
                %project_id, group_type, "group type limit ({MAX_GROUP_TYPES}) reached; group dropped"
            ),
            Err(e) => tracing::warn!(%project_id, group_type, "invalid group type: {e}"),
        }
    }
    Ok(out)
}

/// Apply `$group_set`-style changes: `properties = ((set_once || props) || set) - unset`.
pub async fn upsert(
    db: &PgPool,
    project_id: Uuid,
    index: i16,
    key: &str,
    update: &PersonUpdate,
) -> AppResult<Value> {
    check_index(index)?;
    if key.is_empty() || key.len() > MAX_GROUP_KEY_LEN {
        return Err(bad(format!(
            "group key must be 1-{MAX_GROUP_KEY_LEN} characters"
        )));
    }
    let object = |v: &Value, name: &str| -> AppResult<Value> {
        match v {
            Value::Null => Ok(json!({})),
            Value::Object(_) => Ok(v.clone()),
            _ => Err(bad(format!("{name} must be an object"))),
        }
    };
    let row: (Value, DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO groups (project_id, group_type_index, group_key, properties) \
         VALUES ($1, $2, $3, ($5::jsonb || $4::jsonb) - $6::text[]) \
         ON CONFLICT (project_id, group_type_index, group_key) DO UPDATE SET \
           properties = (($5::jsonb || groups.properties) || $4::jsonb) - $6::text[], \
           updated_at = NOW() \
         RETURNING properties, created_at, updated_at",
    )
    .bind(project_id)
    .bind(index)
    .bind(key)
    .bind(object(&update.set, "set")?)
    .bind(object(&update.set_once, "set_once")?)
    .bind(&update.unset)
    .fetch_one(db)
    .await?;
    Ok(
        json!({ "group_type_index": index, "group_key": key, "properties": row.0,
               "created_at": row.1, "updated_at": row.2 }),
    )
}

/// Handle a `$groupidentify` event's properties. Returns false if it is not one.
pub async fn identify_from_event(
    db: &PgPool,
    project_id: Uuid,
    data: Option<&Value>,
) -> AppResult<bool> {
    let Some(obj) = data.and_then(Value::as_object) else {
        return Ok(false);
    };
    let (Some(group_type), Some(key)) = (
        obj.get("$group_type").and_then(Value::as_str),
        obj.get("$group_key").and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }),
    ) else {
        return Err(bad("$groupidentify needs $group_type and $group_key"));
    };
    let Some(index) = type_index(db, project_id, group_type, true).await? else {
        return Err(bad(format!(
            "this project already has {MAX_GROUP_TYPES} group types"
        )));
    };
    let set = obj.get("$group_set").cloned().unwrap_or_else(|| json!({}));
    upsert(
        db,
        project_id,
        index,
        &key,
        &PersonUpdate {
            set,
            set_once: json!({}),
            unset: Vec::new(),
        },
    )
    .await?;
    Ok(true)
}

/// Group types with their group counts.
pub async fn list_types(db: &PgPool, project_id: Uuid) -> AppResult<Value> {
    let rows: Vec<(i16, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
        "SELECT m.group_type_index, m.group_type, m.name_singular, m.name_plural, \
                (SELECT COUNT(*) FROM groups g WHERE g.project_id = m.project_id \
                 AND g.group_type_index = m.group_type_index) \
         FROM group_type_mappings m WHERE m.project_id = $1 ORDER BY m.group_type_index",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(i, t, s, p, n)| {
            json!({ "group_type_index": i, "group_type": t, "name_singular": s,
                    "name_plural": p, "group_count": n })
        })
        .collect();
    Ok(json!({ "data": data, "max_group_types": MAX_GROUP_TYPES }))
}

#[derive(Debug, Deserialize)]
pub struct TypeNames {
    #[serde(default)]
    pub name_singular: Option<String>,
    #[serde(default)]
    pub name_plural: Option<String>,
}

pub async fn rename_type(
    db: &PgPool,
    project_id: Uuid,
    index: i16,
    names: &TypeNames,
) -> AppResult<Value> {
    check_index(index)?;
    for n in [&names.name_singular, &names.name_plural]
        .into_iter()
        .flatten()
    {
        if n.len() > MAX_GROUP_TYPE_LEN {
            return Err(bad(format!(
                "names must be at most {MAX_GROUP_TYPE_LEN} characters"
            )));
        }
    }
    let n = sqlx::query(
        "UPDATE group_type_mappings SET name_singular = COALESCE($3, name_singular), \
         name_plural = COALESCE($4, name_plural) WHERE project_id = $1 AND group_type_index = $2",
    )
    .bind(project_id)
    .bind(index)
    .bind(&names.name_singular)
    .bind(&names.name_plural)
    .execute(db)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound("Group type not found".into()));
    }
    list_types(db, project_id).await
}

#[derive(Debug, Deserialize, Default)]
pub struct GroupsQuery {
    /// Case-insensitive match on the key or any property value.
    #[serde(default)]
    pub search: Option<String>,
    /// Group-property filters (ANDed); `group_type_index` defaults to the path's.
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

fn check_page(limit: i64, offset: i64) -> AppResult<()> {
    if !(1..=1000).contains(&limit) || offset < 0 {
        return Err(bad("limit must be 1-1000 and offset >= 0"));
    }
    Ok(())
}

/// Groups of one type, most recently updated first.
pub async fn list(db: &PgPool, project_id: Uuid, index: i16, q: &GroupsQuery) -> AppResult<Value> {
    check_index(index)?;
    check_page(q.limit, q.offset)?;
    let alias = format!("g{index}");
    let mut qb = QueryBuilder::<Postgres>::new(format!(
        "SELECT {alias}.group_key, {alias}.properties, {alias}.created_at, {alias}.updated_at, \
         COUNT(*) OVER () FROM groups {alias} WHERE {alias}.project_id = "
    ));
    qb.push_bind(project_id);
    qb.push(format!(" AND {alias}.group_type_index = {index}"));
    if let Some(s) = q.search.as_deref().filter(|s| !s.trim().is_empty()) {
        let like = format!(
            "%{}%",
            s.trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        qb.push(format!(" AND ({alias}.group_key ILIKE "));
        qb.push_bind(like.clone());
        qb.push(format!(" OR {alias}.properties::text ILIKE "));
        qb.push_bind(like);
        qb.push(")");
    }
    for f in &q.properties {
        if f.property_type != PropertyType::Group || f.group_type_index.is_some_and(|i| i != index)
        {
            return Err(bad(
                "groups can be filtered by their own group properties only",
            ));
        }
        let mut f = f.clone();
        f.group_type_index = Some(index);
        qb.push(" AND ");
        push_filter(&mut qb, &f)?;
    }
    qb.push(format!(
        " ORDER BY {alias}.updated_at DESC, {alias}.group_key LIMIT "
    ));
    qb.push_bind(q.limit);
    qb.push(" OFFSET ");
    qb.push_bind(q.offset);
    let rows: Vec<(String, Value, DateTime<Utc>, DateTime<Utc>, i64)> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    let total = rows.first().map(|r| r.4).unwrap_or(0);
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(k, p, c, u, _)| {
            json!({ "group_type_index": index, "group_key": k, "properties": p,
                    "created_at": c, "updated_at": u })
        })
        .collect();
    Ok(json!({ "data": data, "total": total, "limit": q.limit, "offset": q.offset }))
}

/// A group's properties plus activity: events, pageviews, people, first/last seen.
pub async fn get(db: &PgPool, project_id: Uuid, index: i16, key: &str) -> AppResult<Value> {
    check_index(index)?;
    let row: Option<(Value, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT properties, created_at, updated_at FROM groups \
         WHERE project_id = $1 AND group_type_index = $2 AND group_key = $3",
    )
    .bind(project_id)
    .bind(index)
    .bind(key)
    .fetch_optional(db)
    .await?;
    let prop = key_property(index);
    let stats: (i64, i64, i64, Option<DateTime<Utc>>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT COUNT(*) FILTER (WHERE kind = 'e'), COUNT(*) FILTER (WHERE kind = 'p'), \
                COUNT(DISTINCT visitor_id), MIN(created_at), MAX(created_at) FROM ( \
           SELECT 'e' AS kind, visitor_id, created_at FROM events \
             WHERE project_id = $1 AND event_data ->> $2 = $3 \
           UNION ALL \
           SELECT 'p', visitor_id, created_at FROM pageviews \
             WHERE project_id = $1 AND properties ->> $2 = $3) x",
    )
    .bind(project_id)
    .bind(&prop)
    .bind(key)
    .fetch_one(db)
    .await?;
    if row.is_none() && stats.3.is_none() {
        return Err(AppError::NotFound("Group not found".into()));
    }
    Ok(json!({
        "group_type_index": index,
        "group_key": key,
        "properties": row.as_ref().map(|r| r.0.clone()).unwrap_or_else(|| json!({})),
        "created_at": row.as_ref().map(|r| r.1),
        "updated_at": row.as_ref().map(|r| r.2),
        "stats": { "events": stats.0, "pageviews": stats.1, "persons": stats.2,
                   "first_seen_at": stats.3, "last_seen_at": stats.4 },
    }))
}

#[derive(Debug, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// The group's events (pageviews included), newest first.
pub async fn events(
    db: &PgPool,
    project_id: Uuid,
    index: i16,
    key: &str,
    p: &Page,
) -> AppResult<Value> {
    check_index(index)?;
    check_page(p.limit, p.offset)?;
    let names: Option<Vec<String>> = p.event.clone().map(|e| vec![e]);
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT ev.event, ev.visitor_id, ev.created_at, ev.session_id, ev.properties",
    );
    push_event_source(
        &mut qb,
        &EventSource {
            project_id,
            from: DateTime::<Utc>::UNIX_EPOCH,
            to: Utc::now() + chrono::Duration::days(1),
            events: names.as_deref(),
            joins: Default::default(),
        },
    );
    qb.push(" WHERE ev.properties ->> ");
    qb.push_bind(key_property(index));
    qb.push(" = ");
    qb.push_bind(key.to_string());
    qb.push(" ORDER BY ev.created_at DESC LIMIT ");
    qb.push_bind(p.limit);
    qb.push(" OFFSET ");
    qb.push_bind(p.offset);
    let rows: Vec<(String, String, DateTime<Utc>, Uuid, Value)> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(e, v, at, s, props)| {
            json!({ "event": e, "visitor_id": v, "created_at": at, "session_id": s, "properties": props })
        })
        .collect();
    Ok(json!({ "data": data, "limit": p.limit, "offset": p.offset }))
}

/// People who sent events for the group, most active first.
pub async fn persons(
    db: &PgPool,
    project_id: Uuid,
    index: i16,
    key: &str,
    p: &Page,
) -> AppResult<Value> {
    check_index(index)?;
    check_page(p.limit, p.offset)?;
    let prop = key_property(index);
    let rows: Vec<(String, i64, DateTime<Utc>, Option<String>, Option<Value>)> = sqlx::query_as(
        "SELECT x.visitor_id, COUNT(*), MAX(x.created_at), up.user_id, up.traits FROM ( \
           SELECT visitor_id, created_at FROM events WHERE project_id = $1 AND event_data ->> $2 = $3 \
           UNION ALL \
           SELECT visitor_id, created_at FROM pageviews WHERE project_id = $1 AND properties ->> $2 = $3) x \
         LEFT JOIN user_profiles up ON up.project_id = $1 AND up.visitor_id = x.visitor_id \
         GROUP BY x.visitor_id, up.user_id, up.traits \
         ORDER BY COUNT(*) DESC, x.visitor_id LIMIT $4 OFFSET $5",
    )
    .bind(project_id)
    .bind(&prop)
    .bind(key)
    .bind(p.limit)
    .bind(p.offset)
    .fetch_all(db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(v, n, last, u, t)| {
            json!({ "visitor_id": v, "user_id": u, "traits": t, "event_count": n, "last_seen_at": last })
        })
        .collect();
    Ok(json!({ "data": data, "limit": p.limit, "offset": p.offset }))
}

/// Delete a group's properties. Its key stays on past events.
pub async fn delete(db: &PgPool, project_id: Uuid, index: i16, key: &str) -> AppResult<()> {
    check_index(index)?;
    let n = sqlx::query(
        "DELETE FROM groups WHERE project_id = $1 AND group_type_index = $2 AND group_key = $3",
    )
    .bind(project_id)
    .bind(index)
    .bind(key)
    .execute(db)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound("Group not found".into()));
    }
    Ok(())
}

/// Merge `$groups` from event properties with the envelope's groups (event wins)
/// and stamp `$group_<i>` keys into the properties.
pub async fn stamp_event_data(
    db: &PgPool,
    project_id: Uuid,
    envelope_groups: Option<&Map<String, Value>>,
    data: Option<Value>,
) -> AppResult<Option<Value>> {
    let mut groups = envelope_groups.cloned().unwrap_or_default();
    if let Some(Value::Object(g)) = data.as_ref().and_then(|d| d.get(GROUPS_PROPERTY)) {
        groups.extend(g.clone());
    }
    if groups.is_empty() {
        return Ok(data);
    }
    let stamped = stamp(db, project_id, &groups).await?;
    let mut obj = match data {
        Some(Value::Object(o)) => o,
        None | Some(Value::Null) => Map::new(),
        // Non-object payloads are kept as-is; there is nowhere to put group keys.
        other => return Ok(other),
    };
    obj.extend(stamped);
    Ok(Some(Value::Object(obj)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_property_names() {
        assert_eq!(key_property(0), "$group_0");
        assert_eq!(key_property(4), "$group_4");
        assert!(check_index(4).is_ok());
        assert!(check_index(5).is_err());
        assert!(check_index(-1).is_err());
    }
}
