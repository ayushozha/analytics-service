//! Persons (PostHog `Person`): search and filter people by properties and
//! cohorts, a person's detail with their events and sessions, editing
//! properties, merging people, and deletion.
//!
//! A person is identified by `visitor_id`. Merging rewrites the merged
//! visitors' history onto the target and records a redirect so events that
//! still arrive under a merged-away id are attributed to the target.

use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::identity::{self, PersonUpdate};
use crate::services::insights::filters::{push_filter, PropertyFilter};
use crate::services::insights::{map_query_error, push_event_source, EventSource, PropertyType};
use crate::services::privacy::{self, VISITOR_TABLES};
use crate::state::SharedState;

pub const MAX_MERGE: usize = 100;

fn bad(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}

fn redirect_key(state: &SharedState, project_id: Uuid) -> String {
    state.redis_key(&format!("person_redirects:{project_id}"))
}

/// The visitor id an incoming event should be recorded under (follows merges).
/// Redis errors fall back to the original id so ingestion never fails here.
pub async fn resolve_visitor(state: &SharedState, project_id: Uuid, visitor_id: &str) -> String {
    let mut redis = state.redis.clone();
    match redis
        .hget::<_, _, Option<String>>(redirect_key(state, project_id), visitor_id)
        .await
    {
        Ok(Some(target)) => target,
        _ => visitor_id.to_string(),
    }
}

/// Load all redirects into Redis (startup; Redis may have been flushed).
pub async fn warm_redirects(state: &SharedState) -> AppResult<()> {
    let rows: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT project_id, from_visitor_id, to_visitor_id FROM person_redirects")
            .fetch_all(&state.db)
            .await?;
    let mut redis = state.redis.clone();
    for (project_id, from, to) in rows {
        let _: () = redis
            .hset(redirect_key(state, project_id), from, to)
            .await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PersonsQuery {
    /// Matches visitor id, user id or any property value (case-insensitive).
    #[serde(default)]
    pub search: Option<String>,
    /// Person-property and cohort filters (ANDed).
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    /// `last_seen` (default), `first_seen` or `identified`.
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

impl PersonsQuery {
    pub fn uses_cohorts(&self) -> bool {
        self.properties
            .iter()
            .any(|f| f.property_type == PropertyType::Cohort)
    }
}

pub async fn list(db: &PgPool, project_id: Uuid, q: &PersonsQuery) -> AppResult<Value> {
    if !(1..=1000).contains(&q.limit) || q.offset < 0 {
        return Err(bad("limit must be 1-1000 and offset >= 0"));
    }
    let order = match q.order.as_deref().unwrap_or("last_seen") {
        "last_seen" => "up.last_seen_at DESC",
        "first_seen" => "up.first_seen_at DESC",
        "identified" => "up.identified_at DESC NULLS LAST",
        other => return Err(bad(format!("unknown order '{other}'"))),
    };
    // `ev` exposes project_id/visitor_id so cohort filters work on persons too.
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT up.visitor_id, up.user_id, up.traits, up.first_seen_at, up.last_seen_at, \
         up.identified_at, COUNT(*) OVER () AS total \
         FROM user_profiles up \
         CROSS JOIN LATERAL (SELECT up.project_id AS project_id, up.visitor_id AS visitor_id) ev \
         WHERE up.project_id = ",
    );
    qb.push_bind(project_id);
    if let Some(s) = q.search.as_deref().filter(|s| !s.trim().is_empty()) {
        let like = format!(
            "%{}%",
            s.trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        qb.push(" AND (up.visitor_id ILIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR up.user_id ILIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR up.traits::text ILIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
    for f in &q.properties {
        if !matches!(f.property_type, PropertyType::Person | PropertyType::Cohort) {
            return Err(bad(
                "persons can be filtered by person properties and cohorts only",
            ));
        }
        qb.push(" AND ");
        push_filter(&mut qb, f)?;
    }
    qb.push(format!(" ORDER BY {order}, up.visitor_id LIMIT "));
    qb.push_bind(q.limit);
    qb.push(" OFFSET ");
    qb.push_bind(q.offset);
    type Row = (
        String,
        Option<String>,
        Value,
        DateTime<Utc>,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
        i64,
    );
    let rows: Vec<Row> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    let total = rows.first().map(|r| r.6).unwrap_or(0);
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(v, u, t, first, last, ident, _)| {
            json!({ "visitor_id": v, "user_id": u, "traits": t, "first_seen_at": first,
                    "last_seen_at": last, "identified_at": ident })
        })
        .collect();
    Ok(json!({ "data": data, "total": total, "limit": q.limit, "offset": q.offset }))
}

async fn exists(db: &PgPool, project_id: Uuid, visitor_id: &str) -> AppResult<bool> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM user_profiles WHERE project_id = $1 AND visitor_id = $2) \
         + (SELECT COUNT(*) FROM (SELECT 1 FROM sessions WHERE project_id = $1 AND visitor_id = $2 LIMIT 1) s)",
    )
    .bind(project_id)
    .bind(visitor_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

async fn require_person(db: &PgPool, project_id: Uuid, visitor_id: &str) -> AppResult<()> {
    if exists(db, project_id, visitor_id).await? {
        Ok(())
    } else {
        Err(AppError::NotFound("Person not found".into()))
    }
}

/// `include_cohorts` is false when the caller may not read the cohorts module.
pub async fn get(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    include_cohorts: bool,
) -> AppResult<Value> {
    require_person(db, project_id, visitor_id).await?;
    let profile: Option<(
        Option<String>,
        Value,
        DateTime<Utc>,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        "SELECT user_id, traits, first_seen_at, last_seen_at, identified_at FROM user_profiles \
             WHERE project_id = $1 AND visitor_id = $2",
    )
    .bind(project_id)
    .bind(visitor_id)
    .fetch_optional(db)
    .await?;
    let user_id = profile.as_ref().and_then(|p| p.0.clone());
    let aliases: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT visitor_id FROM user_aliases WHERE project_id = $1 AND user_id = $2 ORDER BY 1",
    )
    .bind(project_id)
    .bind(&user_id)
    .fetch_all(db)
    .await?;
    let stats: (i64, i64, i64, Option<DateTime<Utc>>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM pageviews WHERE project_id = $1 AND visitor_id = $2), \
         (SELECT COUNT(*) FROM events WHERE project_id = $1 AND visitor_id = $2), \
         COUNT(*), MIN(first_at), MAX(last_at) FROM sessions WHERE project_id = $1 AND visitor_id = $2",
    )
    .bind(project_id)
    .bind(visitor_id)
    .fetch_one(db)
    .await?;
    let cohorts: Vec<(Uuid, String, bool)> = if include_cohorts {
        sqlx::query_as(
            "SELECT c.id, c.name, c.is_static FROM cohort_people cp JOIN person_cohorts c ON c.id = cp.cohort_id \
             WHERE c.project_id = $1 AND cp.visitor_id = $2 AND NOT c.deleted ORDER BY c.name",
        )
        .bind(project_id)
        .bind(visitor_id)
        .fetch_all(db)
        .await?
    } else {
        Vec::new()
    };
    let merged_from: Vec<(String,)> = sqlx::query_as(
        "SELECT from_visitor_id FROM person_redirects WHERE project_id = $1 AND to_visitor_id = $2 ORDER BY 1",
    )
    .bind(project_id)
    .bind(visitor_id)
    .fetch_all(db)
    .await?;
    Ok(json!({
        "visitor_id": visitor_id,
        "user_id": user_id,
        "traits": profile.as_ref().map(|p| p.1.clone()).unwrap_or_else(|| json!({})),
        "first_seen_at": profile.as_ref().map(|p| p.2).or(stats.3),
        "last_seen_at": profile.as_ref().map(|p| p.3).or(stats.4),
        "identified_at": profile.as_ref().and_then(|p| p.4),
        "aliases": aliases.into_iter().map(|a| a.0).collect::<Vec<_>>(),
        "merged_from": merged_from.into_iter().map(|a| a.0).collect::<Vec<_>>(),
        "stats": { "pageviews": stats.0, "events": stats.1, "sessions": stats.2 },
        // Omitted (null) when the caller may not read cohorts.
        "cohorts": include_cohorts.then(|| cohorts.into_iter().map(|(id, name, s)| json!({"id": id, "name": name, "is_static": s})).collect::<Vec<_>>()),
    }))
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventsQuery {
    /// Only this event (`$pageview` for pageviews).
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// A person's events (pageviews and custom events), newest first.
pub async fn events(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    q: &EventsQuery,
) -> AppResult<Value> {
    require_person(db, project_id, visitor_id).await?;
    if !(1..=1000).contains(&q.limit) || q.offset < 0 {
        return Err(bad("limit must be 1-1000 and offset >= 0"));
    }
    let names: Option<Vec<String>> = q.event.clone().map(|e| vec![e]);
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT ev.event, ev.created_at, ev.session_id, ev.properties",
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
    qb.push(" WHERE ev.visitor_id = ");
    qb.push_bind(visitor_id.to_string());
    qb.push(" ORDER BY ev.created_at DESC LIMIT ");
    qb.push_bind(q.limit);
    qb.push(" OFFSET ");
    qb.push_bind(q.offset);
    let rows: Vec<(String, DateTime<Utc>, Uuid, Value)> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(e, at, s, p)| json!({ "event": e, "created_at": at, "session_id": s, "properties": p }))
        .collect();
    Ok(json!({ "data": data, "limit": q.limit, "offset": q.offset }))
}

/// A person's sessions, newest first, flagging those with a recording.
pub async fn sessions(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    limit: i64,
    offset: i64,
) -> AppResult<Value> {
    require_person(db, project_id, visitor_id).await?;
    if !(1..=1000).contains(&limit) || offset < 0 {
        return Err(bad("limit must be 1-1000 and offset >= 0"));
    }
    let rows: Vec<(Uuid, DateTime<Utc>, DateTime<Utc>, Option<String>, Option<String>, i32, i32, i64, bool, Option<String>, Option<String>, Option<String>, Option<String>, bool)> = sqlx::query_as(
        "SELECT s.id, s.first_at, s.last_at, s.entry_page, s.exit_page, s.pageview_count, s.event_count, \
         s.duration_ms, s.is_bounce, s.browser, s.os, s.device, s.country, \
         EXISTS (SELECT 1 FROM session_recordings r WHERE r.project_id = s.project_id AND r.session_id = s.id) \
         FROM sessions s WHERE s.project_id = $1 AND s.visitor_id = $2 ORDER BY s.first_at DESC LIMIT $3 OFFSET $4",
    )
    .bind(project_id)
    .bind(visitor_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({
            "session_id": r.0, "first_at": r.1, "last_at": r.2, "entry_page": r.3, "exit_page": r.4,
            "pageviews": r.5, "events": r.6, "duration_ms": r.7, "is_bounce": r.8,
            "browser": r.9, "os": r.10, "device": r.11, "country": r.12, "has_recording": r.13,
        }))
        .collect();
    Ok(json!({ "data": data, "limit": limit, "offset": offset }))
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PropertiesPatch {
    #[serde(default)]
    pub set: Option<Value>,
    #[serde(default)]
    pub set_once: Option<Value>,
    #[serde(default)]
    pub unset: Vec<String>,
}

pub async fn update_properties(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    p: &PropertiesPatch,
) -> AppResult<Value> {
    require_person(db, project_id, visitor_id).await?;
    let profile = identity::apply_person_update(
        db,
        project_id,
        visitor_id,
        None,
        &PersonUpdate {
            set: p.set.clone().unwrap_or_else(|| json!({})),
            set_once: p.set_once.clone().unwrap_or_else(|| json!({})),
            unset: p.unset.clone(),
        },
        Utc::now(),
        false,
    )
    .await?;
    Ok(json!({ "visitor_id": profile.visitor_id, "traits": profile.traits }))
}

/// Merge `others` into `target`: their events, sessions and memberships move
/// to the target, their properties fill gaps in the target's (the target's
/// values win), and future events under their ids follow a redirect.
pub async fn merge(
    state: &SharedState,
    project_id: Uuid,
    target: &str,
    others: &[String],
) -> AppResult<Value> {
    let db = &state.db;
    if others.is_empty() || others.len() > MAX_MERGE {
        return Err(bad(format!("merge 1-{MAX_MERGE} persons at a time")));
    }
    if others.iter().any(|o| o == target) {
        return Err(bad("cannot merge a person into themselves"));
    }
    require_person(db, project_id, target).await?;
    for o in others {
        require_person(db, project_id, o).await?;
    }
    let mut tx = db.begin().await?;
    // Properties: others fill gaps (in the given order), the target wins.
    let merged: (Value, Option<String>, Option<DateTime<Utc>>) = sqlx::query_as(
        // Later keys win in jsonb_object_agg, so aggregate in descending list
        // order: the first-listed person's value wins among the others.
        "SELECT COALESCE(jsonb_object_agg(k, v ORDER BY ord DESC) FILTER (WHERE k IS NOT NULL), '{}'::jsonb), \
                (array_agg(user_id ORDER BY ord) FILTER (WHERE user_id IS NOT NULL))[1], MIN(first_seen_at) \
         FROM (SELECT o.ord, up.user_id, up.first_seen_at, t.key AS k, t.value AS v \
               FROM unnest($2::varchar[]) WITH ORDINALITY AS o(visitor_id, ord) \
               JOIN user_profiles up ON up.project_id = $1 AND up.visitor_id = o.visitor_id \
               LEFT JOIN LATERAL jsonb_each(up.traits) t ON TRUE) x",
    )
    .bind(project_id)
    .bind(others)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO user_profiles (project_id, visitor_id, user_id, traits, first_seen_at, last_seen_at) \
         VALUES ($1, $2, $3, $4, COALESCE($5, NOW()), NOW()) \
         ON CONFLICT (project_id, visitor_id) DO UPDATE SET \
           traits = $4::jsonb || user_profiles.traits, \
           user_id = COALESCE(user_profiles.user_id, EXCLUDED.user_id), \
           first_seen_at = LEAST(user_profiles.first_seen_at, EXCLUDED.first_seen_at), \
           updated_at = NOW()",
    )
    .bind(project_id)
    .bind(target)
    .bind(&merged.1)
    .bind(&merged.0)
    .bind(merged.2)
    .execute(&mut *tx)
    .await?;
    // Rows unique per (key, visitor_id) would collide once rewritten to the
    // target: keep the target's row, else the first-listed source's.
    for (table, key) in [
        ("experiment_exposures", "experiment_id"),
        ("account_memberships", "account_id"),
    ] {
        sqlx::query(&format!(
            "DELETE FROM {table} o WHERE o.project_id = $1 AND o.visitor_id = ANY($2) AND EXISTS ( \
               SELECT 1 FROM {table} k WHERE k.project_id = $1 AND k.{key} = o.{key} AND ( \
                 k.visitor_id = $3 OR (k.visitor_id = ANY($2) \
                   AND array_position($2, k.visitor_id) < array_position($2, o.visitor_id))))"
        ))
        .bind(project_id)
        .bind(others)
        .bind(target)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO user_aliases (project_id, user_id, visitor_id) \
         SELECT project_id, user_id, $3 FROM user_aliases WHERE project_id = $1 AND visitor_id = ANY($2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(project_id).bind(others).bind(target).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO cohort_people (cohort_id, visitor_id) \
         SELECT cp.cohort_id, $3 FROM cohort_people cp JOIN person_cohorts c ON c.id = cp.cohort_id \
         WHERE c.project_id = $1 AND cp.visitor_id = ANY($2) ON CONFLICT DO NOTHING",
    )
    .bind(project_id).bind(others).bind(target).execute(&mut *tx).await?;
    sqlx::query(
        "DELETE FROM cohort_people cp USING person_cohorts c \
         WHERE cp.cohort_id = c.id AND c.project_id = $1 AND cp.visitor_id = ANY($2)",
    )
    .bind(project_id)
    .bind(others)
    .execute(&mut *tx)
    .await?;
    let mut moved = serde_json::Map::new();
    for table in VISITOR_TABLES {
        // Aliases and profiles were copied onto the target above; the rest move.
        let delete = matches!(*table, "user_aliases" | "user_profiles");
        let sql = if delete {
            format!("DELETE FROM {table} WHERE project_id = $1 AND visitor_id = ANY($2)")
        } else {
            format!(
                "UPDATE {table} SET visitor_id = $3 WHERE project_id = $1 AND visitor_id = ANY($2)"
            )
        };
        let q = sqlx::query(&sql).bind(project_id).bind(others);
        let q = if delete { q } else { q.bind(target) };
        let n = q.execute(&mut *tx).await?.rows_affected();
        moved.insert(table.to_string(), json!(n));
    }
    // Redirect the merged ids (and anything already redirected to them).
    sqlx::query(
        "UPDATE person_redirects SET to_visitor_id = $3 WHERE project_id = $1 AND to_visitor_id = ANY($2)",
    )
    .bind(project_id).bind(others).bind(target).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO person_redirects (project_id, from_visitor_id, to_visitor_id) \
         SELECT $1, unnest($2::varchar[]), $3 \
         ON CONFLICT (project_id, from_visitor_id) DO UPDATE SET to_visitor_id = EXCLUDED.to_visitor_id",
    )
    .bind(project_id).bind(others).bind(target).execute(&mut *tx).await?;
    let redirects: Vec<(String,)> = sqlx::query_as(
        "SELECT from_visitor_id FROM person_redirects WHERE project_id = $1 AND to_visitor_id = $2",
    )
    .bind(project_id)
    .bind(target)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut redis = state.redis.clone();
    for (from,) in redirects {
        let _: Result<(), _> = redis
            .hset(redirect_key(state, project_id), from, target)
            .await;
    }
    Ok(json!({ "visitor_id": target, "merged": others, "rows_moved": moved }))
}

/// Delete a person's profile; with `delete_events` also all of their data (GDPR).
pub async fn delete(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    delete_events: bool,
) -> AppResult<Value> {
    require_person(db, project_id, visitor_id).await?;
    if delete_events {
        let r = privacy::delete_visitor_data(db, project_id, visitor_id).await?;
        return Ok(serde_json::to_value(r).unwrap_or_else(|_| json!({ "ok": true })));
    }
    let mut tx = db.begin().await?;
    for sql in [
        "DELETE FROM user_aliases WHERE project_id = $1 AND visitor_id = $2",
        "DELETE FROM user_profiles WHERE project_id = $1 AND visitor_id = $2",
        "DELETE FROM cohort_people cp USING person_cohorts c WHERE cp.cohort_id = c.id AND c.project_id = $1 AND cp.visitor_id = $2",
    ] {
        sqlx::query(sql).bind(project_id).bind(visitor_id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(json!({ "ok": true, "deleted_events": false }))
}
