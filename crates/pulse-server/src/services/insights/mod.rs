//! General insights query engine (PostHog-style `TrendsQuery`).
//!
//! Every pageview and custom event is exposed as one virtual event stream:
//!
//! | column       | pageviews                       | events                          |
//! |--------------|---------------------------------|---------------------------------|
//! | `event`      | `'$pageview'`                   | `event_name`                    |
//! | `properties` | `properties` + path/title/utm_* | `event_data` + path/revenue     |
//!
//! Session properties come from `sessions` (joined on `session_id`) and person
//! properties from `user_profiles.traits` (joined on `visitor_id`). Group
//! properties come from `groups` (joined as `g<i>` on the `$group_<i>` key). All user
//! input is bound as parameters; only whitelisted identifiers are interpolated.

pub mod filters;
pub mod formula;
pub mod funnels;
pub mod paths;
pub mod retention;
pub mod saved;
pub mod stickiness;
pub mod trends;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use filters::Joins;

use crate::error::AppError;

/// Name of the virtual event that pageviews are exposed as.
pub const PAGEVIEW_EVENT: &str = "$pageview";
/// Label of the bucket that collects breakdown values outside the top N.
pub const BREAKDOWN_OTHER: &str = "$$_other";
/// Upper bound on buckets in one query (e.g. hourly over 7 months).
pub const MAX_BUCKETS: i64 = 5_000;
/// Upper bound on series per query.
pub const MAX_SERIES: usize = 26;
/// Default and maximum breakdown values returned (plus the "other" bucket).
pub const DEFAULT_BREAKDOWN_LIMIT: i64 = 25;
pub const MAX_BREAKDOWN_LIMIT: i64 = 500;

/// Where a property lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    /// `properties` of the event itself.
    #[default]
    Event,
    /// A column of the event's session (browser, country, entry_page, …).
    Session,
    /// A trait of the visitor's profile (set via identify).
    Person,
    /// Membership of a person cohort (`key` = cohort id). Filters only.
    Cohort,
    /// A property of the event's group of type `group_type_index`.
    Group,
}

/// Session columns that can be filtered or broken down on.
pub const SESSION_PROPERTIES: &[&str] = &[
    "browser",
    "os",
    "device",
    "screen",
    "language",
    "country",
    "region",
    "city",
    "hostname",
    "entry_page",
    "exit_page",
    "is_bounce",
    "duration_ms",
    "pageview_count",
    "event_count",
];

pub(crate) fn bad_request(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}

/// Turn Postgres errors caused by user input (bad regex, bad timezone, bad
/// cast) into 400s instead of opaque 500s.
pub(crate) fn map_query_error(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &e {
        if let Some(code) = db.code() {
            // 2201B invalid_regular_expression, 22023 invalid_parameter_value
            // (unknown time zone), 22P02 invalid_text_representation,
            // 22007/22008 datetime format/overflow.
            if matches!(
                code.as_ref(),
                "2201B" | "22023" | "22P02" | "22007" | "22008"
            ) {
                return bad_request(format!("Invalid query input: {}", db.message()));
            }
        }
    }
    AppError::Database(e)
}

/// Where events come from for one query.
pub(crate) struct EventSource<'a> {
    pub project_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    /// Event names to include (`$pageview` = pageviews); `None` = all events.
    pub events: Option<&'a [String]>,
    pub joins: Joins,
}

/// Push ` FROM (<virtual event stream>) ev [LEFT JOIN sessions s] [LEFT JOIN user_profiles up]`.
/// Only the tables an event list needs are scanned.
pub(crate) fn push_event_source(qb: &mut QueryBuilder<'_, Postgres>, src: &EventSource<'_>) {
    let custom: Option<Vec<String>> = src.events.map(|names| {
        names
            .iter()
            .filter(|n| n.as_str() != PAGEVIEW_EVENT)
            .cloned()
            .collect()
    });
    let want_pageviews = src
        .events
        .is_none_or(|names| names.iter().any(|n| n == PAGEVIEW_EVENT));
    let want_events = custom.as_ref().is_none_or(|c| !c.is_empty());

    qb.push(" FROM (");
    if want_pageviews {
        qb.push(
            "SELECT '$pageview'::text AS event, p.project_id, p.visitor_id, p.session_id, p.created_at, \
             COALESCE(p.properties, '{}'::jsonb) || jsonb_strip_nulls(jsonb_build_object('path', p.path, 'title', p.title, \
             'referrer', p.referrer, 'referrer_domain', p.referrer_domain, \
             'utm_source', p.utm_source, 'utm_medium', p.utm_medium, \
             'utm_campaign', p.utm_campaign, 'utm_content', p.utm_content, \
             'utm_term', p.utm_term)) AS properties \
             FROM pageviews p WHERE p.project_id = ",
        );
        qb.push_bind(src.project_id);
        qb.push(" AND p.created_at >= ");
        qb.push_bind(src.from);
        qb.push(" AND p.created_at < ");
        qb.push_bind(src.to);
    }
    if want_pageviews && want_events {
        qb.push(" UNION ALL ");
    }
    if want_events {
        qb.push(
            "SELECT e.event_name::text AS event, e.project_id, e.visitor_id, e.session_id, e.created_at, \
             (CASE WHEN jsonb_typeof(e.event_data) = 'object' THEN e.event_data ELSE '{}'::jsonb END) \
             || jsonb_strip_nulls(jsonb_build_object('path', e.path, \
             'revenue', e.revenue_amount, 'revenue_currency', e.revenue_currency)) AS properties \
             FROM events e WHERE e.project_id = ",
        );
        qb.push_bind(src.project_id);
        qb.push(" AND e.created_at >= ");
        qb.push_bind(src.from);
        qb.push(" AND e.created_at < ");
        qb.push_bind(src.to);
        if let Some(names) = custom {
            qb.push(" AND e.event_name = ANY(");
            qb.push_bind(names);
            qb.push(")");
        }
    }
    qb.push(") ev");
    if src.joins.session {
        qb.push(" LEFT JOIN sessions s ON s.id = ev.session_id");
    }
    if src.joins.person {
        qb.push(" LEFT JOIN user_profiles up ON up.project_id = ");
        qb.push_bind(src.project_id);
        qb.push(" AND up.visitor_id = ev.visitor_id");
    }
    for i in 0..crate::services::groups::MAX_GROUP_TYPES {
        if src.joins.groups & (1 << i) != 0 {
            qb.push(format!(
                " LEFT JOIN groups g{i} ON g{i}.project_id = ev.project_id \
                 AND g{i}.group_type_index = {i} AND g{i}.group_key = ev.properties ->> '$group_{i}'"
            ));
        }
    }
}

/// `visitor_id -> (user_id, traits)` for a page of actors.
pub(crate) async fn load_profiles(
    db: &PgPool,
    project_id: Uuid,
    visitor_ids: &[String],
) -> Result<HashMap<String, (Option<String>, serde_json::Value)>, sqlx::Error> {
    let rows: Vec<(String, Option<String>, serde_json::Value)> = sqlx::query_as(
        "SELECT visitor_id, user_id, traits FROM user_profiles \
         WHERE project_id = $1 AND visitor_id = ANY($2)",
    )
    .bind(project_id)
    .bind(visitor_ids)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(v, u, t)| (v, (u, t))).collect())
}

/// SQL for the actor a query counts: the visitor, or (PostHog
/// `aggregation_group_type_index`) the key of the event's group of that type.
/// `group` must already be validated with [`validate_aggregation`].
pub(crate) fn actor_expr(group: Option<i16>) -> String {
    match group {
        Some(i) => format!("(ev.properties ->> '$group_{i}')"),
        None => "ev.visitor_id".to_string(),
    }
}

pub(crate) fn validate_aggregation(group: Option<i16>) -> Result<(), AppError> {
    match group {
        Some(i) if !(0..crate::services::groups::MAX_GROUP_TYPES).contains(&i) => {
            Err(bad_request(format!(
                "aggregation_group_type_index must be 0-{}",
                crate::services::groups::MAX_GROUP_TYPES - 1
            )))
        }
        _ => Ok(()),
    }
}

/// Actor objects for a page of ids, in order: people (`visitor_id`, `user_id`,
/// `traits`) or groups of type `group` (`group_type_index`, `group_key`, `properties`).
pub(crate) async fn load_actors(
    db: &PgPool,
    project_id: Uuid,
    group: Option<i16>,
    ids: &[String],
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    use serde_json::json;
    let Some(i) = group else {
        let profiles = load_profiles(db, project_id, ids).await?;
        return Ok(ids
            .iter()
            .map(|v| {
                let (user_id, traits) = profiles.get(v).cloned().unwrap_or((None, json!({})));
                json!({ "visitor_id": v, "user_id": user_id, "traits": traits })
            })
            .collect());
    };
    let rows: Vec<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT group_key, properties FROM groups \
         WHERE project_id = $1 AND group_type_index = $2 AND group_key = ANY($3)",
    )
    .bind(project_id)
    .bind(i)
    .bind(ids)
    .fetch_all(db)
    .await?;
    let props: HashMap<String, serde_json::Value> = rows.into_iter().collect();
    Ok(ids
        .iter()
        .map(|k| {
            json!({ "group_type_index": i, "group_key": k,
                    "properties": props.get(k).cloned().unwrap_or_else(|| json!({})) })
        })
        .collect())
}
