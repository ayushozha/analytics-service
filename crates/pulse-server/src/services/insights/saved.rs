//! Saved insights (PostHog `Insight`): any insight query stored with a name,
//! tags, favourite flag, cached result and version history. Saved queries may
//! use relative dates (`-7d`, `mStart`, …), resolved each time they run.

use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::funnels::FunnelsQuery;
use super::paths::PathsQuery;
use super::retention::RetentionQuery;
use super::trends::TrendsQuery;
use super::{bad_request, funnels, paths, retention, stickiness, trends};
use crate::error::{AppError, AppResult};

/// Cached results younger than this are served without re-running the query.
pub const INSIGHT_CACHE_TTL_SECS: i64 = 15 * 60;
pub const MAX_TAGS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum InsightKind {
    Trends,
    Funnels,
    Retention,
    Stickiness,
    Lifecycle,
    Paths,
}

/// Resolve PostHog-style relative dates in `date_from` / `date_to`:
/// `-24h`, `-7d`, `-2w`, `-3m`, `-1y` (days and longer start at midnight UTC),
/// `dStart`, `mStart`, `yStart`; a missing/null `date_to` means now.
pub fn resolve_dates(query: &mut Value, now: DateTime<Utc>) -> AppResult<()> {
    let Some(obj) = query.as_object_mut() else {
        return Err(bad_request("query must be an object"));
    };
    for key in ["date_from", "date_to"] {
        let resolved = match obj.get(key) {
            None | Some(Value::Null) if key == "date_to" => Some(now),
            Some(Value::String(s)) if DateTime::parse_from_rfc3339(s).is_err() => Some(
                parse_relative(s, now)
                    .ok_or_else(|| bad_request(format!("{key}: unrecognised date '{s}'")))?,
            ),
            _ => None,
        };
        if let Some(dt) = resolved {
            obj.insert(key.to_string(), json!(dt.to_rfc3339()));
        }
    }
    Ok(())
}

fn midnight(d: NaiveDate) -> DateTime<Utc> {
    d.and_hms_opt(0, 0, 0).expect("midnight").and_utc()
}

pub(crate) fn parse_relative(s: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let today = now.date_naive();
    match s {
        "dStart" => return Some(midnight(today)),
        "mStart" => return Some(midnight(today.with_day(1)?)),
        "yStart" => return Some(midnight(NaiveDate::from_ymd_opt(today.year(), 1, 1)?)),
        _ => {}
    }
    let rest = s.strip_prefix('-')?;
    // The unit is one ASCII letter; anything else (e.g. "-7日") is invalid.
    let unit = rest.chars().last().filter(char::is_ascii_alphabetic)?;
    let num = &rest[..rest.len() - 1];
    let unit = unit.to_string();
    let unit = unit.as_str();
    let n: i64 = num.parse().ok().filter(|n| (0..=10_000).contains(n))?;
    match unit {
        "h" => now.checked_sub_signed(Duration::hours(n)),
        "d" => Some(midnight(today.checked_sub_signed(Duration::days(n))?)),
        "w" => Some(midnight(today.checked_sub_signed(Duration::weeks(n))?)),
        "m" => Some(midnight(today.checked_sub_months(Months::new(n as u32))?)),
        "y" => Some(midnight(
            today.checked_sub_months(Months::new(12 * n as u32))?,
        )),
        _ => None,
    }
}

/// A parsed, validated insight query of any kind.
pub enum ParsedQuery {
    Trends(TrendsQuery),
    Funnels(FunnelsQuery),
    Retention(RetentionQuery),
    Stickiness(TrendsQuery),
    Lifecycle(TrendsQuery),
    Paths(PathsQuery),
}

impl ParsedQuery {
    /// Resolve relative dates, deserialize and validate.
    pub fn parse(kind: InsightKind, query: &Value, now: DateTime<Utc>) -> AppResult<Self> {
        let mut q = query.clone();
        resolve_dates(&mut q, now)?;
        let de = |e: serde_json::Error| bad_request(format!("invalid {kind:?} query: {e}"));
        let parsed = match kind {
            InsightKind::Trends => Self::Trends(serde_json::from_value(q).map_err(de)?),
            InsightKind::Funnels => Self::Funnels(serde_json::from_value(q).map_err(de)?),
            InsightKind::Retention => Self::Retention(serde_json::from_value(q).map_err(de)?),
            InsightKind::Stickiness => Self::Stickiness(serde_json::from_value(q).map_err(de)?),
            InsightKind::Lifecycle => Self::Lifecycle(serde_json::from_value(q).map_err(de)?),
            InsightKind::Paths => Self::Paths(serde_json::from_value(q).map_err(de)?),
        };
        match &parsed {
            Self::Trends(t) => trends::validate(t)?,
            Self::Stickiness(t) => stickiness::validate_common(t)?,
            Self::Lifecycle(t) => stickiness::validate_lifecycle(t)?,
            Self::Funnels(f) => funnels::validate(f)?,
            Self::Retention(r) => retention::validate(r)?,
            Self::Paths(p) => paths::validate(p)?,
        }
        Ok(parsed)
    }

    pub fn data_needs(&self) -> super::filters::DataNeeds {
        match self {
            Self::Trends(t) | Self::Stickiness(t) | Self::Lifecycle(t) => t.data_needs(),
            Self::Funnels(f) => f.data_needs(),
            Self::Retention(r) => r.data_needs(),
            Self::Paths(p) => p.data_needs(),
        }
    }

    pub async fn run(&self, db: &PgPool, project_id: Uuid) -> AppResult<Value> {
        let to_json = |v: trends::TrendsResponse| {
            serde_json::to_value(v).map_err(|e| AppError::Internal(e.to_string()))
        };
        match self {
            Self::Trends(q) => to_json(trends::run_trends(db, project_id, q).await?),
            Self::Funnels(q) => funnels::run_funnel(db, project_id, q).await,
            Self::Retention(q) => retention::run_retention(db, project_id, q).await,
            Self::Stickiness(q) => stickiness::run_stickiness(db, project_id, q).await,
            Self::Lifecycle(q) => stickiness::run_lifecycle(db, project_id, q).await,
            Self::Paths(q) => paths::run_paths(db, project_id, q).await,
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Insight {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kind: InsightKind,
    pub query: Value,
    pub tags: Vec<String>,
    pub favorited: bool,
    pub version: i32,
    pub last_refresh_at: Option<DateTime<Utc>>,
    pub last_viewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const INSIGHT_COLUMNS: &str = "id, project_id, name, description, kind, query, tags, favorited, \
    version, last_refresh_at, last_viewed_at, created_at, updated_at";

#[derive(Debug, Clone, Deserialize)]
pub struct InsightInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: InsightKind,
    pub query: Value,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorited: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct InsightPatch {
    pub name: Option<String>,
    /// Omitted: keep. `null`: clear. String: set.
    #[serde(default, deserialize_with = "explicit_null")]
    pub description: Option<Option<String>>,
    pub query: Option<Value>,
    pub tags: Option<Vec<String>>,
    pub favorited: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct InsightListQuery {
    /// Case-insensitive search over name and description.
    pub search: Option<String>,
    pub kind: Option<InsightKind>,
    /// Comma-separated; insights must carry all of them.
    pub tags: Option<String>,
    pub favorited: Option<bool>,
    /// `updated` (default), `viewed`, `name` or `created`.
    pub order: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Distinguish an omitted field (`None`) from an explicit `null` (`Some(None)`).
fn explicit_null<'de, D>(d: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

fn validate_meta(name: &str, tags: &[String]) -> AppResult<()> {
    if name.trim().is_empty() || name.len() > 255 {
        return Err(bad_request("name must be 1-255 characters"));
    }
    if tags.len() > MAX_TAGS || tags.iter().any(|t| t.trim().is_empty() || t.len() > 64) {
        return Err(bad_request(format!(
            "at most {MAX_TAGS} tags of 1-64 characters"
        )));
    }
    Ok(())
}

pub async fn create(db: &PgPool, project_id: Uuid, input: &InsightInput) -> AppResult<Insight> {
    validate_meta(&input.name, &input.tags)?;
    ParsedQuery::parse(input.kind, &input.query, Utc::now())?;
    let mut tx = db.begin().await?;
    let insight: Insight = sqlx::query_as(&format!(
        "INSERT INTO insights (project_id, name, description, kind, query, tags, favorited) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {INSIGHT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(input.name.trim())
    .bind(&input.description)
    .bind(input.kind)
    .bind(&input.query)
    .bind(&input.tags)
    .bind(input.favorited)
    .fetch_one(&mut *tx)
    .await?;
    record_version(&mut tx, &insight).await?;
    tx.commit().await?;
    Ok(insight)
}

async fn record_version(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    i: &Insight,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO insight_versions (insight_id, version, name, description, query) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(i.id)
    .bind(i.version)
    .bind(&i.name)
    .bind(&i.description)
    .bind(&i.query)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn get(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Insight> {
    sqlx::query_as(&format!(
        "SELECT {INSIGHT_COLUMNS} FROM insights WHERE id = $1 AND project_id = $2 AND NOT deleted"
    ))
    .bind(id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Insight not found".into()))
}

pub async fn list(db: &PgPool, project_id: Uuid, f: &InsightListQuery) -> AppResult<Vec<Insight>> {
    let limit = f.limit.unwrap_or(100);
    if !(1..=500).contains(&limit) || f.offset.is_some_and(|o| o < 0) {
        return Err(bad_request("limit must be 1-500 and offset >= 0"));
    }
    let order = match f.order.as_deref().unwrap_or("updated") {
        "updated" => "updated_at DESC",
        "viewed" => "last_viewed_at DESC NULLS LAST",
        "name" => "lower(name) ASC",
        "created" => "created_at DESC",
        other => return Err(bad_request(format!("unknown order '{other}'"))),
    };
    let tags: Vec<String> = f
        .tags
        .as_deref()
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let search = f.search.as_deref().map(|s| {
        format!(
            "%{}%",
            s.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        )
    });
    Ok(sqlx::query_as(&format!(
        "SELECT {INSIGHT_COLUMNS} FROM insights WHERE project_id = $1 AND NOT deleted \
         AND ($2::text IS NULL OR name ILIKE $2 OR description ILIKE $2) \
         AND ($3::varchar IS NULL OR kind = $3) \
         AND tags @> $4 \
         AND ($5::boolean IS NULL OR favorited = $5) \
         ORDER BY {order}, id LIMIT $6 OFFSET $7"
    ))
    .bind(project_id)
    .bind(search)
    .bind(f.kind)
    .bind(&tags)
    .bind(f.favorited)
    .bind(limit)
    .bind(f.offset.unwrap_or(0))
    .fetch_all(db)
    .await?)
}

/// Apply a patch; query/name/description changes bump the version and clear
/// the cache. The row is locked for the read-merge-write so concurrent patches
/// cannot revert each other.
pub async fn update(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
    p: &InsightPatch,
) -> AppResult<Insight> {
    let mut tx = db.begin().await?;
    let cur: Insight = sqlx::query_as(&format!(
        "SELECT {INSIGHT_COLUMNS} FROM insights \
         WHERE id = $1 AND project_id = $2 AND NOT deleted FOR UPDATE"
    ))
    .bind(id)
    .bind(project_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Insight not found".into()))?;
    let name = p.name.clone().unwrap_or(cur.name.clone());
    let tags = p.tags.clone().unwrap_or(cur.tags.clone());
    validate_meta(&name, &tags)?;
    if let Some(q) = &p.query {
        ParsedQuery::parse(cur.kind, q, Utc::now())?;
    }
    let description = match &p.description {
        Some(d) => d.clone(),
        None => cur.description.clone(),
    };
    let query_changed = p.query.as_ref().is_some_and(|q| *q != cur.query);
    let versioned = query_changed || name != cur.name || description != cur.description;
    let updated: Insight = sqlx::query_as(&format!(
        "UPDATE insights SET name = $3, description = $4, query = $5, tags = $6, favorited = $7, \
         version = version + $8, \
         cached_result = CASE WHEN $9 THEN NULL ELSE cached_result END, \
         last_refresh_at = CASE WHEN $9 THEN NULL ELSE last_refresh_at END, \
         updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 RETURNING {INSIGHT_COLUMNS}"
    ))
    .bind(id)
    .bind(project_id)
    .bind(name.trim())
    .bind(&description)
    .bind(p.query.clone().unwrap_or(cur.query.clone()))
    .bind(&tags)
    .bind(p.favorited.unwrap_or(cur.favorited))
    .bind(i32::from(versioned))
    .bind(query_changed)
    .fetch_one(&mut *tx)
    .await?;
    if versioned {
        record_version(&mut tx, &updated).await?;
    }
    tx.commit().await?;
    Ok(updated)
}

/// Soft delete (restorable by patching back is not exposed; rows stay for audit).
pub async fn delete(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<()> {
    let r = sqlx::query(
        "UPDATE insights SET deleted = true, updated_at = NOW() WHERE id = $1 AND project_id = $2 AND NOT deleted",
    )
    .bind(id)
    .bind(project_id)
    .execute(db)
    .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::NotFound("Insight not found".into()));
    }
    Ok(())
}

pub async fn duplicate(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Insight> {
    let cur = get(db, project_id, id).await?;
    let name: String = format!("{} (copy)", cur.name).chars().take(255).collect();
    create(
        db,
        project_id,
        &InsightInput {
            name,
            description: cur.description,
            kind: cur.kind,
            query: cur.query,
            tags: cur.tags,
            favorited: false,
        },
    )
    .await
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct InsightVersion {
    pub version: i32,
    pub name: String,
    pub description: Option<String>,
    pub query: Value,
    pub created_at: DateTime<Utc>,
}

pub async fn versions(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Vec<InsightVersion>> {
    get(db, project_id, id).await?;
    Ok(sqlx::query_as(
        "SELECT version, name, description, query, created_at FROM insight_versions \
         WHERE insight_id = $1 ORDER BY version DESC",
    )
    .bind(id)
    .fetch_all(db)
    .await?)
}

/// Restore an old version as a new version (history is never rewritten).
pub async fn restore(db: &PgPool, project_id: Uuid, id: Uuid, version: i32) -> AppResult<Insight> {
    let v: InsightVersion = sqlx::query_as(
        "SELECT v.version, v.name, v.description, v.query, v.created_at FROM insight_versions v \
         JOIN insights i ON i.id = v.insight_id \
         WHERE v.insight_id = $1 AND i.project_id = $2 AND v.version = $3",
    )
    .bind(id)
    .bind(project_id)
    .bind(version)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Version not found".into()))?;
    update(
        db,
        project_id,
        id,
        &InsightPatch {
            name: Some(v.name),
            // Restore the snapshot exactly, including an absent description.
            description: Some(v.description),
            query: Some(v.query),
            ..Default::default()
        },
    )
    .await
}

/// Result of running a saved insight.
#[derive(Debug, Serialize)]
pub struct InsightResult {
    pub insight: Insight,
    pub result: Value,
    pub cached: bool,
    pub last_refresh_at: DateTime<Utc>,
}

/// Parse a saved insight for running (relative dates resolved to `now`).
pub fn parse_saved(i: &Insight) -> AppResult<ParsedQuery> {
    ParsedQuery::parse(i.kind, &i.query, Utc::now())
}

/// Serve the cached result if fresh, otherwise run and store it.
pub async fn run_saved(
    db: &PgPool,
    project_id: Uuid,
    insight: Insight,
    parsed: &ParsedQuery,
    force: bool,
) -> AppResult<InsightResult> {
    let now = Utc::now();
    if !force {
        if let Some(at) = insight.last_refresh_at {
            if now - at < Duration::seconds(INSIGHT_CACHE_TTL_SECS) {
                // Only a cache produced by this exact query version is valid.
                let cached: Option<(Option<Value>,)> = sqlx::query_as(
                    "SELECT cached_result FROM insights WHERE id = $1 AND version = $2",
                )
                .bind(insight.id)
                .bind(insight.version)
                .fetch_optional(db)
                .await?;
                if let Some((Some(result),)) = cached {
                    touch_viewed(db, insight.id).await?;
                    return Ok(InsightResult {
                        insight,
                        result,
                        cached: true,
                        last_refresh_at: at,
                    });
                }
            }
        }
    }
    let result = parsed.run(db, project_id).await?;
    // Never store a result under a newer query version (a concurrent patch).
    sqlx::query(
        "UPDATE insights SET cached_result = $2, last_refresh_at = $3, last_viewed_at = $3 \
         WHERE id = $1 AND version = $4",
    )
    .bind(insight.id)
    .bind(&result)
    .bind(now)
    .bind(insight.version)
    .execute(db)
    .await?;
    Ok(InsightResult {
        insight,
        result,
        cached: false,
        last_refresh_at: now,
    })
}

async fn touch_viewed(db: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE insights SET last_viewed_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-16T15:30:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn relative_dates() {
        let n = now();
        assert_eq!(
            parse_relative("-7d", n).unwrap().to_rfc3339(),
            "2026-11-09T00:00:00+00:00"
        );
        assert_eq!(
            parse_relative("-24h", n).unwrap().to_rfc3339(),
            "2026-11-15T15:30:00+00:00"
        );
        assert_eq!(
            parse_relative("-1m", n).unwrap().to_rfc3339(),
            "2026-10-16T00:00:00+00:00"
        );
        assert_eq!(
            parse_relative("mStart", n).unwrap().to_rfc3339(),
            "2026-11-01T00:00:00+00:00"
        );
        assert_eq!(
            parse_relative("yStart", n).unwrap().to_rfc3339(),
            "2026-01-01T00:00:00+00:00"
        );
        assert!(parse_relative("-7x", n).is_none());
        assert!(parse_relative("soon", n).is_none());
        assert!(parse_relative("-7日", n).is_none());
        assert!(parse_relative("-", n).is_none());
    }

    #[test]
    fn resolve_fills_date_to_and_keeps_absolute() {
        let mut q = json!({"date_from": "-7d"});
        resolve_dates(&mut q, now()).unwrap();
        assert_eq!(q["date_to"], json!("2026-11-16T15:30:00+00:00"));
        let mut q = json!({"date_from": "2026-01-01T00:00:00Z", "date_to": "2026-02-01T00:00:00Z"});
        resolve_dates(&mut q, now()).unwrap();
        assert_eq!(q["date_from"], json!("2026-01-01T00:00:00Z"));
        assert!(resolve_dates(&mut json!({"date_from": "whenever"}), now()).is_err());
    }

    #[test]
    fn parse_validates_each_kind() {
        let t = json!({"series": [{"event": "$pageview"}], "date_from": "-7d"});
        assert!(ParsedQuery::parse(InsightKind::Trends, &t, now()).is_ok());
        assert!(ParsedQuery::parse(InsightKind::Funnels, &t, now()).is_err());
        let f = json!({"steps": [{"event": "a"}, {"event": "b"}], "date_from": "-30d"});
        assert!(ParsedQuery::parse(InsightKind::Funnels, &f, now()).is_ok());
        let r = json!({"target": {"event": "a"}, "date_from": "-14d"});
        assert!(ParsedQuery::parse(InsightKind::Retention, &r, now()).is_ok());
        let p = json!({"date_from": "-7d"});
        assert!(ParsedQuery::parse(InsightKind::Paths, &p, now()).is_ok());
        let two = json!({"series": [{"event": "a"}, {"event": "b"}], "date_from": "-7d"});
        assert!(ParsedQuery::parse(InsightKind::Lifecycle, &two, now()).is_err());
        assert!(ParsedQuery::parse(InsightKind::Stickiness, &two, now()).is_ok());
        let bd =
            json!({"series": [{"event": "a"}], "date_from": "-7d", "breakdown": {"property": "x"}});
        assert!(ParsedQuery::parse(InsightKind::Stickiness, &bd, now()).is_err());
    }
}
