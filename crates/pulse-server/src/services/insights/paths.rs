//! Paths insight (PostHog `PathsQuery`): the most common sequences of pages
//! and/or custom events within sessions, as a weighted edge list for a Sankey
//! chart. Supports start/end points, step and edge limits, edge weight bounds,
//! path cleaning (regex → alias), wildcard groupings, exclusions, and the
//! visitors behind any node or edge.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::filters::{push_and_filters, push_group, FilterGroup, Joins, PropertyFilter};
use super::{
    bad_request, load_profiles, map_query_error, push_event_source, EventSource, PAGEVIEW_EVENT,
};
use crate::error::{AppError, AppResult};

pub const DEFAULT_STEP_LIMIT: usize = 5;
pub const MAX_STEP_LIMIT: usize = 20;
pub const DEFAULT_EDGE_LIMIT: usize = 50;
pub const MAX_EDGE_LIMIT: usize = 1_000;
pub const MAX_CLEANING_RULES: usize = 50;
pub const MAX_PATH_EVENTS: i64 = 2_000_000;
/// Cleaned items are capped at the path column's size after every rule, so
/// chained replacements cannot grow an item without bound.
pub const MAX_ITEM_LEN: usize = 2_048;
pub const MAX_ALIAS_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathEventType {
    /// Pageviews, keyed by path.
    Pageview,
    /// Custom events, keyed by event name.
    CustomEvent,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CleaningRule {
    /// Postgres regular expression matched against the path item.
    pub regex: String,
    pub alias: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathsQuery {
    pub date_from: DateTime<Utc>,
    pub date_to: DateTime<Utc>,
    #[serde(default = "default_types")]
    pub include_event_types: Vec<PathEventType>,
    /// Begin every path at the first occurrence of this item.
    #[serde(default)]
    pub start_point: Option<String>,
    /// End every path at the first occurrence of this item.
    #[serde(default)]
    pub end_point: Option<String>,
    #[serde(default = "default_step_limit")]
    pub step_limit: usize,
    #[serde(default = "default_edge_limit")]
    pub edge_limit: usize,
    #[serde(default)]
    pub min_edge_weight: Option<i64>,
    #[serde(default)]
    pub max_edge_weight: Option<i64>,
    /// Applied in order, e.g. `{"regex": "/user/[0-9]+", "alias": "/user/:id"}`.
    #[serde(default)]
    pub path_cleaning_rules: Vec<CleaningRule>,
    /// Wildcard groupings, e.g. `/product/*`.
    #[serde(default)]
    pub path_groupings: Vec<String>,
    /// Items removed from paths (after cleaning).
    #[serde(default)]
    pub exclude_events: Vec<String>,
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    #[serde(default)]
    pub filter_group: Option<FilterGroup>,
}

fn default_types() -> Vec<PathEventType> {
    vec![PathEventType::Pageview]
}

fn default_step_limit() -> usize {
    DEFAULT_STEP_LIMIT
}

fn default_edge_limit() -> usize {
    DEFAULT_EDGE_LIMIT
}

impl PathsQuery {
    fn joins(&self) -> Joins {
        let mut j = Joins::default();
        j.add_filters(&self.properties);
        if let Some(g) = &self.filter_group {
            j.add_group(g);
        }
        j
    }

    pub fn data_needs(&self) -> super::filters::DataNeeds {
        self.joins().into()
    }
}

pub fn validate(q: &PathsQuery) -> AppResult<()> {
    if q.date_to <= q.date_from {
        return Err(bad_request("date_to must be after date_from"));
    }
    if q.include_event_types.is_empty() {
        return Err(bad_request("include_event_types must not be empty"));
    }
    if !(2..=MAX_STEP_LIMIT).contains(&q.step_limit) {
        return Err(bad_request(format!(
            "step_limit must be 2-{MAX_STEP_LIMIT}"
        )));
    }
    if !(1..=MAX_EDGE_LIMIT).contains(&q.edge_limit) {
        return Err(bad_request(format!(
            "edge_limit must be 1-{MAX_EDGE_LIMIT}"
        )));
    }
    if q.path_cleaning_rules
        .iter()
        .any(|r| r.alias.len() > MAX_ALIAS_LEN)
        || q.path_groupings.iter().any(|g| g.len() > MAX_ALIAS_LEN)
    {
        return Err(bad_request(format!(
            "cleaning aliases and groupings must be at most {MAX_ALIAS_LEN} characters"
        )));
    }
    if q.path_cleaning_rules.len() + q.path_groupings.len() > MAX_CLEANING_RULES {
        return Err(bad_request(format!(
            "at most {MAX_CLEANING_RULES} cleaning rules and groupings"
        )));
    }
    Ok(())
}

/// Turn a wildcard grouping like `/product/*` into an anchored regex.
pub(crate) fn wildcard_to_regex(pattern: &str) -> String {
    let mut re = String::from("^");
    for c in pattern.chars() {
        match c {
            '*' => re.push_str(".*"),
            c if "\\.+?()[]{}|^$".contains(c) => {
                re.push('\\');
                re.push(c);
            }
            c => re.push(c),
        }
    }
    re.push('$');
    re
}

/// One session's path after cleaning: `(item, epoch seconds)`.
type Path = Vec<(String, f64)>;

struct SessionPath {
    visitor_id: String,
    items: Path,
}

async fn load_paths(db: &PgPool, project_id: Uuid, q: &PathsQuery) -> AppResult<Vec<SessionPath>> {
    validate(q)?;
    let want_pv = q.include_event_types.contains(&PathEventType::Pageview);
    let want_custom = q.include_event_types.contains(&PathEventType::CustomEvent);

    let cleaner = Cleaner::new(q)?;
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT ev.session_id::text, ev.visitor_id, \
         (CASE WHEN ev.event = '$pageview' THEN COALESCE(ev.properties ->> 'path', '/') ELSE ev.event END), \
         EXTRACT(EPOCH FROM ev.created_at)::double precision",
    );

    let pageview_only = [PAGEVIEW_EVENT.to_string()];
    push_event_source(
        &mut qb,
        &EventSource {
            project_id,
            from: q.date_from,
            to: q.date_to,
            events: (want_pv && !want_custom).then_some(&pageview_only[..]),
            joins: q.joins(),
        },
    );
    qb.push(" WHERE TRUE");
    if want_custom && !want_pv {
        qb.push(" AND ev.event <> '$pageview'");
    }
    push_and_filters(&mut qb, &q.properties)?;
    if let Some(g) = &q.filter_group {
        push_group(&mut qb, g)?;
    }
    qb.push(" ORDER BY ev.session_id, ev.created_at LIMIT ");
    qb.push_bind(MAX_PATH_EVENTS + 1);

    let rows: Vec<(String, String, String, f64)> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    if rows.len() as i64 > MAX_PATH_EVENTS {
        return Err(bad_request(
            "too many matching events; narrow the date range or add filters",
        ));
    }
    let q = q.clone();
    tokio::task::spawn_blocking(move || build_paths(rows, &q, &cleaner))
        .await
        .map_err(|e| AppError::Internal(format!("paths worker failed: {e}")))
}

fn truncate_chars(s: &mut String, max_bytes: usize) {
    if s.len() > max_bytes {
        let mut cut = max_bytes;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
    }
}

/// Cleaning rules then wildcard groupings, compiled once per query.
struct Cleaner {
    rules: Vec<(regex::Regex, String)>,
    groups: Vec<(regex::Regex, String)>,
}

impl Cleaner {
    fn new(q: &PathsQuery) -> AppResult<Self> {
        let compile = |re: &str| {
            regex::Regex::new(re).map_err(|e| bad_request(format!("invalid regex '{re}': {e}")))
        };
        Ok(Self {
            rules: q
                .path_cleaning_rules
                .iter()
                .map(|r| Ok((compile(&r.regex)?, r.alias.clone())))
                .collect::<AppResult<_>>()?,
            groups: q
                .path_groupings
                .iter()
                .map(|g| Ok((compile(&wildcard_to_regex(g))?, g.clone())))
                .collect::<AppResult<_>>()?,
        })
    }

    fn clean(&self, item: String) -> String {
        let mut item = item;
        for (re, alias) in &self.rules {
            if re.is_match(&item) {
                item = re.replace_all(&item, alias.as_str()).into_owned();
                truncate_chars(&mut item, MAX_ITEM_LEN);
            }
        }
        for (re, alias) in &self.groups {
            if re.is_match(&item) {
                return alias.clone();
            }
        }
        item
    }
}

fn build_paths(
    rows: Vec<(String, String, String, f64)>,
    q: &PathsQuery,
    cleaner: &Cleaner,
) -> Vec<SessionPath> {
    let excluded: HashSet<&str> = q.exclude_events.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    let mut rows = rows.into_iter().peekable();
    while let Some((session, visitor, item, ts)) = rows.next() {
        let mut raw = vec![(cleaner.clean(item), ts)];
        while rows.peek().is_some_and(|r| r.0 == session) {
            let (_, _, item, ts) = rows.next().expect("peeked");
            raw.push((cleaner.clean(item), ts));
        }
        let items = shape_path(raw, q, &excluded);
        if !items.is_empty() {
            out.push(SessionPath {
                visitor_id: visitor,
                items,
            });
        }
    }
    out
}

/// Drop excluded items, collapse repeats, then apply start/end points and the
/// step limit (an end point keeps the steps leading up to it).
pub(crate) fn shape_path(raw: Path, q: &PathsQuery, excluded: &HashSet<&str>) -> Path {
    let mut items: Path = Vec::with_capacity(raw.len());
    for (item, ts) in raw {
        if excluded.contains(item.as_str()) {
            continue;
        }
        if items.last().is_some_and(|(last, _)| *last == item) {
            continue;
        }
        items.push((item, ts));
    }
    if let Some(start) = &q.start_point {
        match items.iter().position(|(i, _)| i == start) {
            Some(p) => {
                items.drain(..p);
            }
            None => return Vec::new(),
        }
    }
    if let Some(end) = &q.end_point {
        match items.iter().position(|(i, _)| i == end) {
            Some(p) => {
                items.truncate(p + 1);
                if q.start_point.is_none() && items.len() > q.step_limit {
                    items.drain(..items.len() - q.step_limit);
                }
            }
            None => return Vec::new(),
        }
    }
    items.truncate(q.step_limit);
    items
}

fn node_key(step: usize, item: &str) -> String {
    format!("{}_{item}", step + 1)
}

pub async fn run_paths(db: &PgPool, project_id: Uuid, q: &PathsQuery) -> AppResult<Value> {
    let paths = load_paths(db, project_id, q).await?;
    // (source, target) -> (count, total seconds)
    let mut edges: HashMap<(String, String), (i64, f64)> = HashMap::new();
    for p in &paths {
        for (i, w) in p.items.windows(2).enumerate() {
            let e = edges
                .entry((node_key(i, &w[0].0), node_key(i + 1, &w[1].0)))
                .or_default();
            e.0 += 1;
            e.1 += w[1].1 - w[0].1;
        }
    }
    let mut list: Vec<((String, String), (i64, f64))> = edges
        .into_iter()
        .filter(|(_, (n, _))| {
            q.min_edge_weight.is_none_or(|m| *n >= m) && q.max_edge_weight.is_none_or(|m| *n <= m)
        })
        .collect();
    list.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(&b.0)));
    list.truncate(q.edge_limit);
    let results: Vec<Value> = list
        .into_iter()
        .map(|((source, target), (value, secs))| {
            json!({
                "source": source,
                "target": target,
                "value": value,
                "average_conversion_time_secs": secs / value as f64,
            })
        })
        .collect();
    Ok(json!({ "results": results, "sessions": paths.len() }))
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathsActorsQuery {
    pub query: PathsQuery,
    /// Node key like `1_/home`. Alone: visitors who passed through that node.
    pub path_start_key: String,
    /// With a start key: visitors who took the edge start → end.
    #[serde(default)]
    pub path_end_key: Option<String>,
    /// Visitors whose path ended at `path_start_key` (dropped off there).
    #[serde(default)]
    pub dropoff: bool,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

pub async fn run_paths_actors(
    db: &PgPool,
    project_id: Uuid,
    a: &PathsActorsQuery,
) -> AppResult<Value> {
    if !(1..=1000).contains(&a.limit) {
        return Err(bad_request("limit must be 1-1000"));
    }
    let paths = load_paths(db, project_id, &a.query).await?;
    let mut ids: HashSet<String> = HashSet::new();
    for p in &paths {
        let keys: Vec<String> = p
            .items
            .iter()
            .enumerate()
            .map(|(i, (item, _))| node_key(i, item))
            .collect();
        let hit = match (&a.path_end_key, a.dropoff) {
            (Some(end), _) => keys
                .windows(2)
                .any(|w| w[0] == a.path_start_key && &w[1] == end),
            (None, true) => keys.last() == Some(&a.path_start_key),
            (None, false) => keys.contains(&a.path_start_key),
        };
        if hit {
            ids.insert(p.visitor_id.clone());
        }
    }
    let mut ids: Vec<String> = ids.into_iter().collect();
    ids.sort();
    let total = ids.len();
    let page: Vec<String> = ids.into_iter().skip(a.offset).take(a.limit).collect();
    let profiles = load_profiles(db, project_id, &page).await?;
    let data: Vec<Value> = page
        .iter()
        .map(|v| {
            let (user_id, traits) = profiles.get(v).cloned().unwrap_or((None, json!({})));
            json!({ "visitor_id": v, "user_id": user_id, "traits": traits })
        })
        .collect();
    Ok(json!({ "data": data, "total": total, "limit": a.limit, "offset": a.offset }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: Value) -> PathsQuery {
        let mut base =
            json!({"date_from": "2026-11-01T00:00:00Z", "date_to": "2026-11-08T00:00:00Z"});
        base.as_object_mut()
            .unwrap()
            .extend(v.as_object().unwrap().clone());
        serde_json::from_value(base).unwrap()
    }

    fn raw(items: &[&str]) -> Path {
        items
            .iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), i as f64))
            .collect()
    }

    fn names(p: &Path) -> Vec<&str> {
        p.iter().map(|(s, _)| s.as_str()).collect()
    }

    #[test]
    fn collapses_repeats_and_excludes() {
        let qq = q(json!({"exclude_events": ["/x"]}));
        let ex: HashSet<&str> = qq.exclude_events.iter().map(String::as_str).collect();
        let p = shape_path(raw(&["/", "/", "/x", "/a", "/a", "/b"]), &qq, &ex);
        assert_eq!(names(&p), ["/", "/a", "/b"]);
    }

    #[test]
    fn start_end_and_step_limit() {
        let ex = HashSet::new();
        let p = shape_path(
            raw(&["/", "/a", "/b", "/c"]),
            &q(json!({"start_point": "/a"})),
            &ex,
        );
        assert_eq!(names(&p), ["/a", "/b", "/c"]);
        let p = shape_path(
            raw(&["/", "/a", "/b", "/c"]),
            &q(json!({"end_point": "/b"})),
            &ex,
        );
        assert_eq!(names(&p), ["/", "/a", "/b"]);
        let p = shape_path(
            raw(&["/1", "/2", "/3", "/4", "/end"]),
            &q(json!({"end_point": "/end", "step_limit": 2})),
            &ex,
        );
        assert_eq!(names(&p), ["/4", "/end"]);
        let p = shape_path(raw(&["/", "/a"]), &q(json!({"start_point": "/zzz"})), &ex);
        assert!(p.is_empty());
        let p = shape_path(
            raw(&["/1", "/2", "/3", "/4"]),
            &q(json!({"step_limit": 3})),
            &ex,
        );
        assert_eq!(names(&p), ["/1", "/2", "/3"]);
    }

    #[test]
    fn wildcard_groupings_escape_regex() {
        assert_eq!(wildcard_to_regex("/product/*"), "^/product/.*$");
        assert_eq!(wildcard_to_regex("/a.b?/*"), "^/a\\.b\\?/.*$");
    }

    #[test]
    fn cleaning_rules_then_groupings() {
        let qq = q(json!({
            "path_cleaning_rules": [{"regex": "/user/[0-9]+", "alias": "/user/:id"}],
            "path_groupings": ["/product/*"]
        }));
        let c = Cleaner::new(&qq).unwrap();
        assert_eq!(c.clean("/user/42/settings".into()), "/user/:id/settings");
        assert_eq!(c.clean("/product/shoes/1".into()), "/product/*");
        assert_eq!(c.clean("/about".into()), "/about");
        let bad = q(json!({"path_cleaning_rules": [{"regex": "(", "alias": "x"}]}));
        assert!(Cleaner::new(&bad).is_err());
    }

    #[test]
    fn chained_rules_cannot_blow_up_items() {
        let rules: Vec<Value> = (0..50)
            .map(|_| json!({"regex": ".", "alias": "xx"}))
            .collect();
        let qq = q(json!({"path_cleaning_rules": rules}));
        assert!(validate(&qq).is_ok());
        let c = Cleaner::new(&qq).unwrap();
        assert_eq!(c.clean("/".into()).len(), MAX_ITEM_LEN);
        let long_alias =
            q(json!({"path_cleaning_rules": [{"regex": ".", "alias": "x".repeat(300)}]}));
        assert!(validate(&long_alias).is_err());
    }

    #[test]
    fn validation() {
        assert!(validate(&q(json!({}))).is_ok());
        assert!(validate(&q(json!({"step_limit": 1}))).is_err());
        assert!(validate(&q(json!({"include_event_types": []}))).is_err());
        assert!(validate(&q(json!({"edge_limit": 0}))).is_err());
    }
}
