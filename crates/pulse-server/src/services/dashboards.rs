//! Dashboards (PostHog `Dashboard` + `DashboardTile`): saved-insight and text
//! tiles on a grid, dashboard-wide filters (date range + properties) applied
//! to every insight tile, per-tile result caching, duplicate, move, and
//! built-in templates.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::insights::saved::{
    self, Insight, InsightInput, InsightKind, ParsedQuery, INSIGHT_CACHE_TTL_SECS,
};

pub const MAX_TILES: i64 = 100;
pub const MAX_TEXT_LEN: usize = 10_000;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Dashboard {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub is_home: bool,
    pub filters: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const DASHBOARD_COLUMNS: &str =
    "id, project_id, name, description, tags, pinned, is_home, filters, created_at, updated_at";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Tile {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub insight_id: Option<Uuid>,
    pub text: Option<String>,
    pub layout: Value,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const TILE_COLUMNS: &str =
    "id, dashboard_id, insight_id, text, layout, color, created_at, updated_at";

/// Dashboard-wide filters, applied on top of each insight's own query.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DashboardFilters {
    /// Absolute or relative (`-7d`, `mStart`, …) dates override the insight's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,
    /// Extra property filters ANDed into every insight.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Value>,
}

impl DashboardFilters {
    fn from_value(v: &Value) -> AppResult<Self> {
        serde_json::from_value(v.clone())
            .map_err(|e| AppError::BadRequest(format!("invalid dashboard filters: {e}")))
    }

    fn is_empty(&self) -> bool {
        self.date_from.is_none() && self.date_to.is_none() && self.properties.is_empty()
    }

    /// The insight query with these filters applied.
    pub fn apply(&self, query: &Value) -> Value {
        let mut q = query.clone();
        if let Some(obj) = q.as_object_mut() {
            if let Some(f) = &self.date_from {
                obj.insert("date_from".into(), json!(f));
            }
            if let Some(t) = &self.date_to {
                obj.insert("date_to".into(), json!(t));
            }
            if !self.properties.is_empty() {
                let props = obj.entry("properties").or_insert_with(|| json!([]));
                if let Some(arr) = props.as_array_mut() {
                    arr.extend(self.properties.iter().cloned());
                }
            }
        }
        q
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DashboardInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub filters: Option<Value>,
    /// Create the dashboard (and its insights) from a built-in template.
    #[serde(default)]
    pub template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DashboardPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned: Option<bool>,
    pub is_home: Option<bool>,
    pub filters: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DashboardListQuery {
    pub search: Option<String>,
    pub tags: Option<String>,
    pub pinned: Option<bool>,
}

fn validate_meta(name: &str, tags: &[String]) -> AppResult<()> {
    if name.trim().is_empty() || name.len() > 255 {
        return Err(AppError::BadRequest("name must be 1-255 characters".into()));
    }
    if tags.len() > saved::MAX_TAGS || tags.iter().any(|t| t.trim().is_empty() || t.len() > 64) {
        return Err(AppError::BadRequest("invalid tags".into()));
    }
    Ok(())
}

pub async fn create(db: &PgPool, project_id: Uuid, input: &DashboardInput) -> AppResult<Dashboard> {
    validate_meta(&input.name, &input.tags)?;
    let filters = input.filters.clone().unwrap_or_else(|| json!({}));
    DashboardFilters::from_value(&filters)?;
    let template = match &input.template {
        Some(t) => Some(
            templates()
                .into_iter()
                .find(|x| x.id == t)
                .ok_or_else(|| AppError::BadRequest(format!("unknown template '{t}'")))?,
        ),
        None => None,
    };
    let d: Dashboard = sqlx::query_as(&format!(
        "INSERT INTO dashboards (project_id, name, description, tags, pinned, filters) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {DASHBOARD_COLUMNS}"
    ))
    .bind(project_id)
    .bind(input.name.trim())
    .bind(&input.description)
    .bind(&input.tags)
    .bind(input.pinned)
    .bind(&filters)
    .fetch_one(db)
    .await?;
    if let Some(t) = template {
        for (i, ti) in t.tiles.into_iter().enumerate() {
            let layout = json!({ "x": (i % 2) * 6, "y": (i / 2) * 5, "w": 6, "h": 5 });
            let insight = saved::create(db, project_id, &ti).await?;
            add_tile(
                db,
                project_id,
                d.id,
                &TileInput {
                    insight_id: Some(insight.id),
                    text: None,
                    layout: Some(layout),
                    color: None,
                },
            )
            .await?;
        }
    }
    Ok(d)
}

pub async fn get(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Dashboard> {
    sqlx::query_as(&format!(
        "SELECT {DASHBOARD_COLUMNS} FROM dashboards WHERE id = $1 AND project_id = $2 AND NOT deleted"
    ))
    .bind(id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Dashboard not found".into()))
}

pub async fn list(
    db: &PgPool,
    project_id: Uuid,
    f: &DashboardListQuery,
) -> AppResult<Vec<Dashboard>> {
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
        "SELECT {DASHBOARD_COLUMNS} FROM dashboards WHERE project_id = $1 AND NOT deleted \
         AND ($2::text IS NULL OR name ILIKE $2 OR description ILIKE $2) \
         AND tags @> $3 AND ($4::boolean IS NULL OR pinned = $4) \
         ORDER BY is_home DESC, pinned DESC, lower(name), id"
    ))
    .bind(project_id)
    .bind(search)
    .bind(&tags)
    .bind(f.pinned)
    .fetch_all(db)
    .await?)
}

pub async fn update(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
    p: &DashboardPatch,
) -> AppResult<Dashboard> {
    let cur = get(db, project_id, id).await?;
    let name = p.name.clone().unwrap_or(cur.name.clone());
    let tags = p.tags.clone().unwrap_or(cur.tags.clone());
    validate_meta(&name, &tags)?;
    if let Some(f) = &p.filters {
        DashboardFilters::from_value(f)?;
    }
    let mut tx = db.begin().await?;
    if p.is_home == Some(true) {
        sqlx::query("UPDATE dashboards SET is_home = false WHERE project_id = $1 AND is_home")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;
    }
    let d: Dashboard = sqlx::query_as(&format!(
        "UPDATE dashboards SET name = $3, description = $4, tags = $5, pinned = $6, is_home = $7, \
         filters = $8, updated_at = NOW() WHERE id = $1 AND project_id = $2 RETURNING {DASHBOARD_COLUMNS}"
    ))
    .bind(id)
    .bind(project_id)
    .bind(name.trim())
    .bind(p.description.clone().or(cur.description))
    .bind(&tags)
    .bind(p.pinned.unwrap_or(cur.pinned))
    .bind(p.is_home.unwrap_or(cur.is_home))
    .bind(p.filters.clone().unwrap_or(cur.filters))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(d)
}

pub async fn delete(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<()> {
    let r = sqlx::query(
        "UPDATE dashboards SET deleted = true, is_home = false, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 AND NOT deleted",
    )
    .bind(id)
    .bind(project_id)
    .execute(db)
    .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::NotFound("Dashboard not found".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileInput {
    #[serde(default)]
    pub insight_id: Option<Uuid>,
    /// Markdown text tile (mutually exclusive with `insight_id`).
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub layout: Option<Value>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TilePatch {
    pub text: Option<String>,
    pub layout: Option<Value>,
    pub color: Option<String>,
}

fn validate_layout(v: &Value) -> AppResult<()> {
    let ok = v.as_object().is_some_and(|o| {
        ["x", "y", "w", "h"].iter().all(|k| {
            o.get(*k)
                .is_none_or(|n| n.as_u64().is_some_and(|n| n <= 1_000))
        })
    });
    if ok {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "layout must be {x, y, w, h} with integers 0-1000".into(),
        ))
    }
}

fn validate_text(t: &str) -> AppResult<()> {
    if t.trim().is_empty() || t.len() > MAX_TEXT_LEN {
        return Err(AppError::BadRequest(format!(
            "text must be 1-{MAX_TEXT_LEN} characters"
        )));
    }
    Ok(())
}

pub async fn tiles(db: &PgPool, dashboard_id: Uuid) -> AppResult<Vec<Tile>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {TILE_COLUMNS} FROM dashboard_tiles WHERE dashboard_id = $1 \
         ORDER BY (layout->>'y')::int NULLS LAST, (layout->>'x')::int NULLS LAST, created_at"
    ))
    .bind(dashboard_id)
    .fetch_all(db)
    .await?)
}

pub async fn add_tile(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
    t: &TileInput,
) -> AppResult<Tile> {
    get(db, project_id, dashboard_id).await?;
    match (&t.insight_id, &t.text) {
        (Some(i), None) => {
            saved::get(db, project_id, *i).await?;
        }
        (None, Some(text)) => validate_text(text)?,
        _ => {
            return Err(AppError::BadRequest(
                "a tile needs exactly one of insight_id or text".into(),
            ))
        }
    }
    let layout = t.layout.clone().unwrap_or_else(|| json!({}));
    validate_layout(&layout)?;
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM dashboard_tiles WHERE dashboard_id = $1")
            .bind(dashboard_id)
            .fetch_one(db)
            .await?;
    if n >= MAX_TILES {
        return Err(AppError::BadRequest(format!(
            "dashboards hold at most {MAX_TILES} tiles"
        )));
    }
    Ok(sqlx::query_as(&format!(
        "INSERT INTO dashboard_tiles (dashboard_id, insight_id, text, layout, color) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {TILE_COLUMNS}"
    ))
    .bind(dashboard_id)
    .bind(t.insight_id)
    .bind(&t.text)
    .bind(&layout)
    .bind(&t.color)
    .fetch_one(db)
    .await?)
}

async fn tile_owned(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
    tile_id: Uuid,
) -> AppResult<Tile> {
    get(db, project_id, dashboard_id).await?;
    sqlx::query_as(&format!(
        "SELECT {TILE_COLUMNS} FROM dashboard_tiles WHERE id = $1 AND dashboard_id = $2"
    ))
    .bind(tile_id)
    .bind(dashboard_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Tile not found".into()))
}

pub async fn update_tile(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
    tile_id: Uuid,
    p: &TilePatch,
) -> AppResult<Tile> {
    let cur = tile_owned(db, project_id, dashboard_id, tile_id).await?;
    if p.text.is_some() && cur.text.is_none() {
        return Err(AppError::BadRequest("only text tiles have text".into()));
    }
    if let Some(t) = &p.text {
        validate_text(t)?;
    }
    if let Some(l) = &p.layout {
        validate_layout(l)?;
    }
    Ok(sqlx::query_as(&format!(
        "UPDATE dashboard_tiles SET text = COALESCE($2, text), layout = COALESCE($3, layout), \
         color = COALESCE($4, color), updated_at = NOW() WHERE id = $1 RETURNING {TILE_COLUMNS}"
    ))
    .bind(tile_id)
    .bind(&p.text)
    .bind(&p.layout)
    .bind(&p.color)
    .fetch_one(db)
    .await?)
}

pub async fn delete_tile(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
    tile_id: Uuid,
) -> AppResult<()> {
    tile_owned(db, project_id, dashboard_id, tile_id).await?;
    sqlx::query("DELETE FROM dashboard_tiles WHERE id = $1")
        .bind(tile_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Move a tile to another dashboard of the same project.
pub async fn move_tile(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
    tile_id: Uuid,
    to: Uuid,
) -> AppResult<Tile> {
    tile_owned(db, project_id, dashboard_id, tile_id).await?;
    get(db, project_id, to).await?;
    Ok(sqlx::query_as(&format!(
        "UPDATE dashboard_tiles SET dashboard_id = $2, cache_key = NULL, cached_result = NULL, \
         updated_at = NOW() WHERE id = $1 RETURNING {TILE_COLUMNS}"
    ))
    .bind(tile_id)
    .bind(to)
    .fetch_one(db)
    .await?)
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutItem {
    pub id: Uuid,
    pub layout: Value,
}

/// Save the whole grid at once (after a drag/resize).
pub async fn update_layouts(
    db: &PgPool,
    project_id: Uuid,
    dashboard_id: Uuid,
    items: &[LayoutItem],
) -> AppResult<Vec<Tile>> {
    get(db, project_id, dashboard_id).await?;
    for i in items {
        validate_layout(&i.layout)?;
    }
    let mut tx = db.begin().await?;
    for i in items {
        let r = sqlx::query(
            "UPDATE dashboard_tiles SET layout = $3, updated_at = NOW() WHERE id = $1 AND dashboard_id = $2",
        )
        .bind(i.id)
        .bind(dashboard_id)
        .bind(&i.layout)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Tile {} not found", i.id)));
        }
    }
    tx.commit().await?;
    tiles(db, dashboard_id).await
}

/// Copy a dashboard; with `duplicate_insights` the insights are copied too,
/// otherwise the new tiles point at the same insights.
pub async fn duplicate(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
    duplicate_insights: bool,
) -> AppResult<Dashboard> {
    let src = get(db, project_id, id).await?;
    let name: String = format!("{} (copy)", src.name).chars().take(255).collect();
    let copy = create(
        db,
        project_id,
        &DashboardInput {
            name,
            description: src.description.clone(),
            tags: src.tags.clone(),
            pinned: false,
            filters: Some(src.filters.clone()),
            template: None,
        },
    )
    .await?;
    for t in tiles(db, id).await? {
        let insight_id = match t.insight_id {
            Some(i) if duplicate_insights => Some(saved::duplicate(db, project_id, i).await?.id),
            other => other,
        };
        add_tile(
            db,
            project_id,
            copy.id,
            &TileInput {
                insight_id,
                text: t.text,
                layout: Some(t.layout),
                color: t.color,
            },
        )
        .await?;
    }
    Ok(copy)
}

/// One rendered tile.
#[derive(Debug, Serialize)]
pub struct TileResult {
    pub tile: Tile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insight: Option<Insight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Decides whether a tile may read person data (module gating); returns an
/// error message when it may not.
pub type PersonGate<'a> = &'a (dyn Fn(&ParsedQuery) -> Option<String> + Sync);

fn cache_key(insight: &Insight, filters: &DashboardFilters) -> String {
    let mut h = Sha256::new();
    h.update(insight.id.as_bytes());
    h.update(insight.version.to_be_bytes());
    h.update(serde_json::to_vec(filters).unwrap_or_default());
    hex::encode(h.finalize())
}

/// Run every insight tile with the dashboard filters applied. A failing tile
/// reports its error without failing the whole dashboard.
pub async fn run(
    db: &PgPool,
    project_id: Uuid,
    dashboard: &Dashboard,
    refresh: bool,
    gate: PersonGate<'_>,
) -> AppResult<Vec<TileResult>> {
    let filters = DashboardFilters::from_value(&dashboard.filters)?;
    let mut out = Vec::new();
    for tile in tiles(db, dashboard.id).await? {
        let Some(insight_id) = tile.insight_id else {
            out.push(TileResult {
                tile,
                insight: None,
                result: None,
                cached: None,
                last_refresh_at: None,
                error: None,
            });
            continue;
        };
        let insight = match saved::get(db, project_id, insight_id).await {
            Ok(i) => i,
            Err(e) => {
                out.push(TileResult {
                    tile,
                    insight: None,
                    result: None,
                    cached: None,
                    last_refresh_at: None,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };
        let rendered = run_tile(db, project_id, &tile, &insight, &filters, refresh, gate).await;
        out.push(match rendered {
            Ok((result, cached, at)) => TileResult {
                tile,
                insight: Some(insight),
                result: Some(result),
                cached: Some(cached),
                last_refresh_at: Some(at),
                error: None,
            },
            Err(e) => TileResult {
                tile,
                insight: Some(insight),
                result: None,
                cached: None,
                last_refresh_at: None,
                error: Some(e.to_string()),
            },
        });
    }
    Ok(out)
}

async fn run_tile(
    db: &PgPool,
    project_id: Uuid,
    tile: &Tile,
    insight: &Insight,
    filters: &DashboardFilters,
    refresh: bool,
    gate: PersonGate<'_>,
) -> AppResult<(Value, bool, DateTime<Utc>)> {
    let query = filters.apply(&insight.query);
    let parsed = ParsedQuery::parse(insight.kind, &query, Utc::now())?;
    if let Some(msg) = gate(&parsed) {
        return Err(AppError::Forbidden(msg));
    }
    if filters.is_empty() {
        // Same query as the insight itself: share its cache.
        let r = saved::run_saved(db, project_id, insight.clone(), &parsed, refresh).await?;
        return Ok((r.result, r.cached, r.last_refresh_at));
    }
    let key = cache_key(insight, filters);
    let now = Utc::now();
    if !refresh {
        let row: Option<(Option<String>, Option<Value>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT cache_key, cached_result, last_refresh_at FROM dashboard_tiles WHERE id = $1",
        )
        .bind(tile.id)
        .fetch_optional(db)
        .await?;
        if let Some((Some(k), Some(result), Some(at))) = row {
            if k == key && now - at < Duration::seconds(INSIGHT_CACHE_TTL_SECS) {
                return Ok((result, true, at));
            }
        }
    }
    let result = parsed.run(db, project_id).await?;
    sqlx::query(
        "UPDATE dashboard_tiles SET cache_key = $2, cached_result = $3, last_refresh_at = $4 WHERE id = $1",
    )
    .bind(tile.id)
    .bind(&key)
    .bind(&result)
    .bind(now)
    .execute(db)
    .await?;
    Ok((result, false, now))
}

/// A built-in dashboard template.
#[derive(Debug, Clone, Serialize)]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    #[serde(skip)]
    pub tiles: Vec<InsightInput>,
}

fn tpl(name: &str, kind: InsightKind, query: Value) -> InsightInput {
    InsightInput {
        name: name.to_string(),
        description: None,
        kind,
        query,
        tags: vec!["template".to_string()],
        favorited: false,
    }
}

/// Built-in templates. Queries use relative dates so they stay current.
pub fn templates() -> Vec<Template> {
    let pv = json!({"event": "$pageview"});
    let any = json!({"event": null});
    vec![
        Template {
            id: "web-analytics",
            name: "Web analytics",
            description:
                "Traffic, visitors, top pages, referrers and countries for the last 30 days.",
            tiles: vec![
                tpl(
                    "Pageviews",
                    InsightKind::Trends,
                    json!({"series": [pv], "date_from": "-30d"}),
                ),
                tpl(
                    "Unique visitors",
                    InsightKind::Trends,
                    json!({"series": [{"event": "$pageview", "math": "dau"}], "date_from": "-30d"}),
                ),
                tpl(
                    "Top pages",
                    InsightKind::Trends,
                    json!({"series": [pv], "date_from": "-30d", "interval": "month", "breakdown": {"property": "path", "limit": 10}}),
                ),
                tpl(
                    "Top referrers",
                    InsightKind::Trends,
                    json!({"series": [{"event": "$pageview", "math": "dau"}], "date_from": "-30d", "interval": "month", "breakdown": {"property": "referrer_domain", "limit": 10}}),
                ),
                tpl(
                    "Countries",
                    InsightKind::Trends,
                    json!({"series": [{"event": "$pageview", "math": "dau"}], "date_from": "-30d", "interval": "month", "breakdown": {"property": "country", "type": "session", "limit": 10}}),
                ),
                tpl(
                    "Top paths",
                    InsightKind::Paths,
                    json!({"date_from": "-30d"}),
                ),
            ],
        },
        Template {
            id: "product-analytics",
            name: "Product analytics",
            description: "Active users, stickiness, lifecycle and retention for the last 30 days.",
            tiles: vec![
                tpl(
                    "Daily active users",
                    InsightKind::Trends,
                    json!({"series": [{"event": null, "math": "dau"}], "date_from": "-30d"}),
                ),
                tpl(
                    "Weekly active users",
                    InsightKind::Trends,
                    json!({"series": [{"event": null, "math": "dau"}], "date_from": "-90d", "interval": "week"}),
                ),
                tpl(
                    "Stickiness",
                    InsightKind::Stickiness,
                    json!({"series": [any], "date_from": "-30d"}),
                ),
                tpl(
                    "Lifecycle",
                    InsightKind::Lifecycle,
                    json!({"series": [any], "date_from": "-30d"}),
                ),
                tpl(
                    "Weekly retention",
                    InsightKind::Retention,
                    json!({"target": {"event": null}, "retention_type": "recurring", "period": "week", "total_intervals": 8, "date_from": "-8w"}),
                ),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_override_dates_and_append_properties() {
        let f = DashboardFilters {
            date_from: Some("-7d".into()),
            date_to: None,
            properties: vec![json!({"key": "country", "type": "session", "value": "US"})],
        };
        let q =
            json!({"series": [], "date_from": "-30d", "properties": [{"key": "a", "value": "b"}]});
        let out = f.apply(&q);
        assert_eq!(out["date_from"], "-7d");
        assert_eq!(out["properties"].as_array().unwrap().len(), 2);
        let out = f.apply(&json!({"series": []}));
        assert_eq!(out["properties"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn templates_are_valid_queries() {
        let now = Utc::now();
        for t in templates() {
            for i in &t.tiles {
                ParsedQuery::parse(i.kind, &i.query, now)
                    .unwrap_or_else(|e| panic!("template {} / {}: {e}", t.id, i.name));
            }
        }
    }

    #[test]
    fn layout_validation() {
        assert!(validate_layout(&json!({"x": 0, "y": 2, "w": 6, "h": 4})).is_ok());
        assert!(validate_layout(&json!({"x": -1})).is_err());
        assert!(validate_layout(&json!([1, 2])).is_err());
    }
}
