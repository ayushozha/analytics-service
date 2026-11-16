//! Funnels insight (PostHog `FunnelsQuery`): ordered / strict / unordered
//! funnels with per-step filters, a conversion window, exclusion steps,
//! breakdowns with attribution, time-to-convert, conversion trends, and the
//! visitors who converted or dropped at each step.
//!
//! Matching events are streamed from Postgres ordered by visitor and time, and
//! each visitor's best attempt is computed in Rust (the same per-visitor
//! "window funnel" PostHog runs in ClickHouse).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::filters::{
    push_and_filters, push_filter, push_group, push_property_expr, validate_property, FilterGroup,
    Joins, PropertyFilter,
};
use super::trends::{bucket_labels, Breakdown, Interval};
use super::{
    bad_request, load_profiles, map_query_error, push_event_source, EventSource, BREAKDOWN_OTHER,
    DEFAULT_BREAKDOWN_LIMIT, MAX_BREAKDOWN_LIMIT,
};
use crate::error::AppResult;

pub const MIN_STEPS: usize = 2;
pub const MAX_STEPS: usize = 20;
/// Upper bound on matching events streamed for one funnel.
pub const MAX_FUNNEL_EVENTS: i64 = 2_000_000;
const MAX_WINDOW_DAYS: i64 = 365;
const NONE_LABEL: &str = "$$_none";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunnelStepDef {
    /// Event name; `$pageview` for pageviews; `null` matches any event.
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub custom_name: Option<String>,
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Exclusion {
    pub event: Option<String>,
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    /// Exclude if this event happens after step `from_step` ...
    pub from_step: usize,
    /// ... and before step `to_step` is reached.
    pub to_step: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FunnelOrder {
    /// Steps in order; other events may happen in between.
    #[default]
    Ordered,
    /// Steps in order with no other events in between.
    Strict,
    /// Steps in any order.
    Unordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowUnit {
    Minute,
    Hour,
    #[default]
    Day,
    Week,
    Month,
}

impl WindowUnit {
    fn seconds(self) -> f64 {
        match self {
            WindowUnit::Minute => 60.0,
            WindowUnit::Hour => 3_600.0,
            WindowUnit::Day => 86_400.0,
            WindowUnit::Week => 7.0 * 86_400.0,
            WindowUnit::Month => 30.0 * 86_400.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Attribution {
    /// Breakdown value of the event that entered the funnel.
    #[default]
    FirstTouch,
    /// Breakdown value of the last step reached.
    LastTouch,
    /// Breakdown value at `breakdown_attribution_step` (falls back to first touch).
    Step,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FunnelViz {
    #[default]
    Steps,
    TimeToConvert,
    Trends,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunnelsQuery {
    pub steps: Vec<FunnelStepDef>,
    pub date_from: DateTime<Utc>,
    pub date_to: DateTime<Utc>,
    #[serde(default = "default_tz")]
    pub timezone: String,
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    #[serde(default)]
    pub filter_group: Option<FilterGroup>,
    #[serde(default = "default_window")]
    pub funnel_window: i64,
    #[serde(default)]
    pub funnel_window_unit: WindowUnit,
    #[serde(default)]
    pub order: FunnelOrder,
    #[serde(default)]
    pub exclusions: Vec<Exclusion>,
    #[serde(default)]
    pub breakdown: Option<Breakdown>,
    #[serde(default)]
    pub breakdown_attribution: Attribution,
    #[serde(default)]
    pub breakdown_attribution_step: Option<usize>,
    #[serde(default)]
    pub viz: FunnelViz,
    /// For `trends`: bucket size of the entry time.
    #[serde(default)]
    pub interval: Interval,
    /// For `time_to_convert` / `trends`: measure from this step ...
    #[serde(default)]
    pub from_step: Option<usize>,
    /// ... to this step (defaults: first and last step).
    #[serde(default)]
    pub to_step: Option<usize>,
    /// For `time_to_convert`: number of histogram bins.
    #[serde(default)]
    pub bins: Option<usize>,
}

fn default_tz() -> String {
    "UTC".to_string()
}

fn default_window() -> i64 {
    14
}

impl FunnelsQuery {
    /// Protected data the query reads (person properties, cohorts).
    pub fn data_needs(&self) -> super::filters::DataNeeds {
        self.joins().into()
    }

    fn window_secs(&self) -> f64 {
        self.funnel_window as f64 * self.funnel_window_unit.seconds()
    }

    fn range(&self) -> (usize, usize) {
        (
            self.from_step.unwrap_or(0),
            self.to_step.unwrap_or(self.steps.len() - 1),
        )
    }

    fn joins(&self) -> Joins {
        let mut j = Joins::default();
        j.add_filters(&self.properties);
        if let Some(g) = &self.filter_group {
            j.add_group(g);
        }
        for s in &self.steps {
            j.add_filters(&s.properties);
        }
        for x in &self.exclusions {
            j.add_filters(&x.properties);
        }
        if let Some(b) = &self.breakdown {
            j.add(b.property_type);
        }
        j
    }
}

pub fn validate(q: &FunnelsQuery) -> AppResult<()> {
    if !(MIN_STEPS..=MAX_STEPS).contains(&q.steps.len()) {
        return Err(bad_request(format!(
            "funnels need {MIN_STEPS}-{MAX_STEPS} steps"
        )));
    }
    if q.date_to <= q.date_from {
        return Err(bad_request("date_to must be after date_from"));
    }
    if q.funnel_window <= 0 || q.window_secs() > (MAX_WINDOW_DAYS * 86_400) as f64 {
        return Err(bad_request(format!(
            "funnel_window must be positive and at most {MAX_WINDOW_DAYS} days"
        )));
    }
    if q.timezone.is_empty() || q.timezone.len() > 64 {
        return Err(bad_request("invalid timezone"));
    }
    let n = q.steps.len();
    for x in &q.exclusions {
        if x.from_step >= x.to_step || x.to_step >= n {
            return Err(bad_request(
                "exclusion needs from_step < to_step < number of steps",
            ));
        }
    }
    let (from, to) = q.range();
    if from >= to || to >= n {
        return Err(bad_request(
            "from_step must be before to_step and within the funnel",
        ));
    }
    if q.breakdown_attribution == Attribution::Step
        && q.breakdown_attribution_step.is_none_or(|s| s >= n)
    {
        return Err(bad_request(
            "breakdown_attribution_step must be a valid step index",
        ));
    }
    if let Some(b) = &q.breakdown {
        validate_property(&b.property, b.property_type)?;
        let limit = b.limit.unwrap_or(DEFAULT_BREAKDOWN_LIMIT);
        if !(1..=MAX_BREAKDOWN_LIMIT).contains(&limit) {
            return Err(bad_request(format!(
                "breakdown limit must be 1-{MAX_BREAKDOWN_LIMIT}"
            )));
        }
    }
    if q.bins.is_some_and(|b| b == 0 || b > 200) {
        return Err(bad_request("bins must be 1-200"));
    }
    Ok(())
}

/// One matching event of one visitor.
#[derive(Debug, Clone)]
struct Ev {
    ts: f64,
    steps: Vec<bool>,
    excl: Vec<bool>,
    bd: String,
    bucket: String,
}

/// A visitor's best attempt through the funnel.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Attempt {
    /// Number of steps reached (0 = never entered).
    pub reached: usize,
    /// Event index used for each reached step.
    pub idx: Vec<usize>,
}

/// Candidate events per step considered by unordered matching. At most
/// `MAX_STEPS` events can be used, so more candidates per step never help.
const UNORDERED_CANDIDATES: usize = MAX_STEPS;

/// Precomputed lookups so each attempt costs O(steps · log n), not O(n).
struct Index {
    /// `next[s][j]`: first event index >= j matching step s (or `len`).
    next: Vec<Vec<usize>>,
    /// `excl_prefix[x][j]`: number of exclusion-x events in `evs[..j]`.
    excl_prefix: Vec<Vec<u32>>,
}

impl Index {
    fn new(evs: &[Ev], n_steps: usize, n_excl: usize) -> Self {
        let len = evs.len();
        let mut next = vec![vec![len; len + 1]; n_steps];
        for (s, row) in next.iter_mut().enumerate() {
            for j in (0..len).rev() {
                row[j] = if evs[j].steps[s] { j } else { row[j + 1] };
            }
        }
        let mut excl_prefix = vec![vec![0_u32; len + 1]; n_excl];
        for (x, row) in excl_prefix.iter_mut().enumerate() {
            for j in 0..len {
                row[j + 1] = row[j] + u32::from(evs[j].excl[x]);
            }
        }
        Self { next, excl_prefix }
    }
}

/// Steps reached for one visitor, given time-ordered events. Attempts may only
/// enter the funnel before `entry_limit` (epoch seconds; `date_to`), but may
/// complete within one conversion window after it.
fn best_attempt(evs: &[Ev], q: &FunnelsQuery, entry_limit: f64) -> Attempt {
    let n = q.steps.len();
    let window = q.window_secs();
    let ix = Index::new(evs, n, q.exclusions.len());
    let mut best = Attempt {
        reached: 0,
        idx: Vec::new(),
    };
    for start in 0..evs.len() {
        let anchor_ok = match q.order {
            FunnelOrder::Unordered => evs[start].steps.iter().any(|&m| m),
            _ => evs[start].steps[0],
        };
        if !anchor_ok || evs[start].ts >= entry_limit {
            continue;
        }
        let t0 = evs[start].ts;
        let attempt = match q.order {
            FunnelOrder::Ordered | FunnelOrder::Strict => {
                let strict = q.order == FunnelOrder::Strict;
                let mut idx = vec![start];
                let mut k = start;
                while idx.len() < n {
                    let step = idx.len();
                    let j = if strict { k + 1 } else { ix.next[step][k + 1] };
                    if j >= evs.len() || !evs[j].steps[step] || evs[j].ts - t0 > window {
                        break;
                    }
                    idx.push(j);
                    k = j;
                }
                Attempt {
                    reached: idx.len(),
                    idx,
                }
            }
            FunnelOrder::Unordered => unordered_attempt(evs, &ix, start, t0 + window, n),
        };
        if q.order != FunnelOrder::Unordered && excluded(evs, q, &ix, &attempt) {
            continue;
        }
        if attempt.reached > best.reached {
            best = attempt;
            if best.reached == n {
                break;
            }
        }
    }
    best
}

/// Maximum number of steps that can each be matched to a distinct event in
/// `[start, window_end]` (bipartite matching, so overlapping step predicates
/// are assigned optimally).
fn unordered_attempt(evs: &[Ev], ix: &Index, start: usize, window_end: f64, n: usize) -> Attempt {
    // Candidate events per step: the first few matches inside the window.
    let candidates: Vec<Vec<usize>> = (0..n)
        .map(|s| {
            let mut c = Vec::new();
            let mut j = ix.next[s][start];
            while j < evs.len() && evs[j].ts <= window_end && c.len() < UNORDERED_CANDIDATES {
                c.push(j);
                j = ix.next[s][j + 1];
            }
            c
        })
        .collect();
    let mut owner: HashMap<usize, usize> = HashMap::new();
    fn augment(
        s: usize,
        candidates: &[Vec<usize>],
        owner: &mut HashMap<usize, usize>,
        seen: &mut Vec<usize>,
    ) -> bool {
        for &e in &candidates[s] {
            if seen.contains(&e) {
                continue;
            }
            seen.push(e);
            let free = match owner.get(&e) {
                None => true,
                Some(&other) => augment(other, candidates, owner, seen),
            };
            if free {
                owner.insert(e, s);
                return true;
            }
        }
        false
    }
    for s in 0..n {
        let mut seen = Vec::new();
        augment(s, &candidates, &mut owner, &mut seen);
    }
    let mut idx: Vec<usize> = owner.into_keys().collect();
    idx.sort_unstable();
    Attempt {
        reached: idx.len(),
        idx,
    }
}

/// True if an exclusion event happened between reaching `from_step` and
/// reaching `to_step` (or before the window closed if `to_step` was missed).
fn excluded(evs: &[Ev], q: &FunnelsQuery, ix: &Index, a: &Attempt) -> bool {
    let window_end = evs[a.idx[0]].ts + q.window_secs();
    q.exclusions.iter().enumerate().any(|(xi, x)| {
        let Some(&from_idx) = a.idx.get(x.from_step) else {
            return false;
        };
        let until = a
            .idx
            .get(x.to_step)
            .map(|&i| evs[i].ts)
            .unwrap_or(window_end);
        // First index after `from_idx` whose timestamp is past `until`.
        let end = from_idx + 1 + evs[from_idx + 1..].partition_point(|e| e.ts <= until);
        ix.excl_prefix[xi][end] > ix.excl_prefix[xi][from_idx + 1]
    })
}

fn attributed_breakdown(evs: &[Ev], q: &FunnelsQuery, a: &Attempt) -> String {
    let pick = match q.breakdown_attribution {
        Attribution::FirstTouch => a.idx.first(),
        Attribution::LastTouch => a.idx.last(),
        Attribution::Step => a
            .idx
            .get(q.breakdown_attribution_step.unwrap_or(0))
            .or(a.idx.first()),
    };
    pick.map(|&i| evs[i].bd.clone()).unwrap_or_default()
}

/// Per-visitor outcome used by every visualisation.
struct Outcome {
    visitor_id: String,
    bd: String,
    reached: usize,
    times: Vec<f64>,
    entry_bucket: String,
}

async fn load_outcomes(db: &PgPool, project_id: Uuid, q: &FunnelsQuery) -> AppResult<Vec<Outcome>> {
    validate(q)?;
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT ev.visitor_id, EXTRACT(EPOCH FROM ev.created_at)::double precision, ARRAY[",
    );
    let push_step_pred = |qb: &mut QueryBuilder<'_, Postgres>,
                          event: &Option<String>,
                          props: &[PropertyFilter]|
     -> AppResult<()> {
        // A filter on a missing property yields NULL; treat it as no match.
        qb.push("COALESCE((TRUE");
        if let Some(e) = event {
            qb.push(" AND ev.event = ");
            qb.push_bind(e.clone());
        }
        for f in props {
            qb.push(" AND ");
            push_filter(qb, f)?;
        }
        qb.push("), FALSE)");
        Ok(())
    };
    for (i, s) in q.steps.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        push_step_pred(&mut qb, &s.event, &s.properties)?;
    }
    qb.push("]::boolean[], ARRAY[");
    for (i, x) in q.exclusions.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        push_step_pred(&mut qb, &x.event, &x.properties)?;
    }
    qb.push("]::boolean[], ");
    match &q.breakdown {
        Some(b) => {
            qb.push("COALESCE(");
            push_property_expr(&mut qb, &b.property, b.property_type);
            qb.push(format!(", '{NONE_LABEL}')"));
        }
        None => {
            qb.push("''::text");
        }
    }
    qb.push(", to_char(date_trunc(");
    qb.push_bind(q.interval_sql());
    qb.push(", ev.created_at AT TIME ZONE ");
    qb.push_bind(q.timezone.clone());
    qb.push("), 'YYYY-MM-DD\"T\"HH24:MI:SS')");

    // Strict funnels must see every event to detect interruptions; other
    // orders only need the step and exclusion events.
    let names: Option<Vec<String>> = if q.order == FunnelOrder::Strict {
        None
    } else {
        let all: Option<Vec<String>> = q
            .steps
            .iter()
            .map(|s| s.event.clone())
            .chain(q.exclusions.iter().map(|x| x.event.clone()))
            .collect();
        all.map(|mut v| {
            v.sort();
            v.dedup();
            v
        })
    };
    // Conversions may complete up to one window after date_to.
    let window = chrono::Duration::seconds(q.window_secs() as i64);
    push_event_source(
        &mut qb,
        &EventSource {
            project_id,
            from: q.date_from,
            to: q.date_to + window,
            events: names.as_deref(),
            joins: q.joins(),
        },
    );
    qb.push(" WHERE TRUE");
    push_and_filters(&mut qb, &q.properties)?;
    if let Some(g) = &q.filter_group {
        push_group(&mut qb, g)?;
    }
    if q.order != FunnelOrder::Strict {
        qb.push(" AND (FALSE");
        for s in &q.steps {
            qb.push(" OR ");
            push_step_pred(&mut qb, &s.event, &s.properties)?;
        }
        for x in &q.exclusions {
            qb.push(" OR ");
            push_step_pred(&mut qb, &x.event, &x.properties)?;
        }
        qb.push(")");
    }
    qb.push(" ORDER BY ev.visitor_id, ev.created_at LIMIT ");
    qb.push_bind(MAX_FUNNEL_EVENTS + 1);

    type Row = (String, f64, Vec<bool>, Vec<bool>, String, String);
    let rows: Vec<Row> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    if rows.len() as i64 > MAX_FUNNEL_EVENTS {
        return Err(bad_request(
            "too many matching events; narrow the date range or add filters",
        ));
    }

    // Matching is CPU-bound; keep it off the async worker threads.
    let q = q.clone();
    tokio::task::spawn_blocking(move || compute_outcomes(rows, &q))
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("funnel worker failed: {e}")))
}

fn compute_outcomes(
    rows: Vec<(String, f64, Vec<bool>, Vec<bool>, String, String)>,
    q: &FunnelsQuery,
) -> Vec<Outcome> {
    let entry_limit = q.date_to.timestamp() as f64;
    let mut outcomes = Vec::new();
    let mut rows = rows.into_iter().peekable();
    while let Some((visitor, ts, steps, excl, bd, bucket)) = rows.next() {
        let mut evs = vec![Ev {
            ts,
            steps,
            excl,
            bd,
            bucket,
        }];
        while rows.peek().is_some_and(|r| r.0 == visitor) {
            let (_, ts, steps, excl, bd, bucket) = rows.next().expect("peeked");
            evs.push(Ev {
                ts,
                steps,
                excl,
                bd,
                bucket,
            });
        }
        let a = best_attempt(&evs, q, entry_limit);
        if a.reached == 0 {
            continue;
        }
        outcomes.push(Outcome {
            visitor_id: visitor,
            bd: attributed_breakdown(&evs, q, &a),
            reached: a.reached,
            times: a.idx.iter().map(|&j| evs[j].ts).collect(),
            entry_bucket: evs[a.idx[0]].bucket.clone(),
        });
    }
    outcomes
}

impl FunnelsQuery {
    fn interval_sql(&self) -> &'static str {
        match self.interval {
            Interval::Minute => "minute",
            Interval::Hour => "hour",
            Interval::Day => "day",
            Interval::Week => "week",
            Interval::Month => "month",
        }
    }
}

/// Group outcomes by breakdown value, keeping the top N by entries.
fn group_by_breakdown<'a>(
    q: &FunnelsQuery,
    outcomes: &'a [Outcome],
) -> Vec<(String, Vec<&'a Outcome>)> {
    let mut groups: HashMap<&str, Vec<&Outcome>> = HashMap::new();
    for o in outcomes {
        groups.entry(o.bd.as_str()).or_default().push(o);
    }
    let mut ranked: Vec<(String, Vec<&Outcome>)> = groups
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    if q.breakdown.is_none() {
        return ranked;
    }
    let limit = q
        .breakdown
        .as_ref()
        .and_then(|b| b.limit)
        .unwrap_or(DEFAULT_BREAKDOWN_LIMIT) as usize;
    if ranked.len() > limit {
        let rest: Vec<&Outcome> = ranked.drain(limit..).flat_map(|(_, v)| v).collect();
        ranked.push((BREAKDOWN_OTHER.to_string(), rest));
    }
    ranked
}

fn breakdown_label(bd: &str) -> Value {
    match bd {
        "" | NONE_LABEL => Value::Null,
        other => json!(other),
    }
}

fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = v.len() / 2;
    Some(if v.len().is_multiple_of(2) {
        (v[m - 1] + v[m]) / 2.0
    } else {
        v[m]
    })
}

fn step_name(q: &FunnelsQuery, i: usize) -> String {
    let s = &q.steps[i];
    s.custom_name
        .clone()
        .or_else(|| s.event.clone())
        .unwrap_or_else(|| "Any event".to_string())
}

fn steps_result(q: &FunnelsQuery, group: &[&Outcome]) -> Vec<Value> {
    let n = q.steps.len();
    let entered = group.len() as f64;
    let mut prev = entered;
    (0..n)
        .map(|i| {
            let count = group.iter().filter(|o| o.reached > i).count() as f64;
            let mut secs: Vec<f64> = if i == 0 {
                Vec::new()
            } else {
                group
                    .iter()
                    .filter(|o| o.reached > i)
                    .map(|o| o.times[i] - o.times[i - 1])
                    .collect()
            };
            let avg = (!secs.is_empty()).then(|| secs.iter().sum::<f64>() / secs.len() as f64);
            let med = median(&mut secs);
            let row = json!({
                "order": i,
                "name": step_name(q, i),
                "count": count as i64,
                "conversion_rate": if entered > 0.0 { count / entered * 100.0 } else { 0.0 },
                "conversion_rate_from_previous": if prev > 0.0 { count / prev * 100.0 } else { 0.0 },
                "dropoff_count": (prev - count) as i64,
                "avg_time_to_convert_secs": avg,
                "median_time_to_convert_secs": med,
            });
            prev = count;
            row
        })
        .collect()
}

fn time_to_convert(q: &FunnelsQuery, group: &[&Outcome]) -> Value {
    let (from, to) = q.range();
    let mut secs: Vec<f64> = group
        .iter()
        .filter(|o| o.reached > to)
        .map(|o| o.times[to] - o.times[from])
        .collect();
    let bins = q.bins.unwrap_or(20);
    let avg = (!secs.is_empty()).then(|| secs.iter().sum::<f64>() / secs.len() as f64);
    let med = median(&mut secs);
    let max = secs.iter().cloned().fold(0.0_f64, f64::max);
    let width = if max <= 0.0 {
        1.0
    } else {
        (max / bins as f64).ceil().max(1.0)
    };
    let mut hist = vec![0_i64; bins];
    for s in &secs {
        let b = ((s / width) as usize).min(bins - 1);
        hist[b] += 1;
    }
    json!({
        "from_step": from,
        "to_step": to,
        "converted": secs.len(),
        "average_secs": avg,
        "median_secs": med,
        "bins": hist.iter().enumerate().map(|(i, c)| json!({
            "from_secs": i as f64 * width,
            "to_secs": (i + 1) as f64 * width,
            "count": c,
        })).collect::<Vec<_>>(),
    })
}

fn conversion_trend(q: &FunnelsQuery, group: &[&Outcome], days: &[String]) -> Value {
    let (from, to) = q.range();
    let mut entered: HashMap<&str, (i64, i64)> = HashMap::new();
    for o in group.iter().filter(|o| o.reached > from) {
        let e = entered.entry(o.entry_bucket.as_str()).or_default();
        e.0 += 1;
        if o.reached > to {
            e.1 += 1;
        }
    }
    let data: Vec<Value> = days
        .iter()
        .map(|d| {
            let (base, conv) = entered.get(d.as_str()).copied().unwrap_or((0, 0));
            json!({
                "day": d,
                "entered": base,
                "converted": conv,
                "conversion_rate": if base > 0 { conv as f64 / base as f64 * 100.0 } else { 0.0 },
            })
        })
        .collect();
    json!({ "from_step": from, "to_step": to, "data": data })
}

pub async fn run_funnel(db: &PgPool, project_id: Uuid, q: &FunnelsQuery) -> AppResult<Value> {
    let outcomes = load_outcomes(db, project_id, q).await?;
    let groups = group_by_breakdown(q, &outcomes);
    let days = if q.viz == FunnelViz::Trends {
        bucket_labels(db, &q.interval, &q.timezone, q.date_from, q.date_to).await?
    } else {
        Vec::new()
    };
    let render = |bd: &str, g: &[&Outcome]| {
        let mut r = json!({ "breakdown_value": breakdown_label(bd) });
        match q.viz {
            FunnelViz::Steps => r["steps"] = json!(steps_result(q, g)),
            FunnelViz::TimeToConvert => r["time_to_convert"] = time_to_convert(q, g),
            FunnelViz::Trends => r["trend"] = conversion_trend(q, g, &days),
        }
        r
    };
    let results: Vec<Value> = if groups.is_empty() && q.breakdown.is_none() {
        vec![render("", &[])]
    } else {
        groups.iter().map(|(bd, g)| render(bd, g)).collect()
    };
    Ok(json!({ "results": results, "order": q.order, "viz": q.viz }))
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunnelActorsQuery {
    pub query: FunnelsQuery,
    /// 0-based step.
    pub step: usize,
    /// `false`: visitors who reached `step`. `true`: visitors who reached
    /// `step - 1` but dropped before `step`.
    #[serde(default)]
    pub dropped: bool,
    #[serde(default)]
    pub breakdown_value: Option<String>,
    #[serde(default = "default_actor_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_actor_limit() -> usize {
    100
}

pub async fn run_funnel_actors(
    db: &PgPool,
    project_id: Uuid,
    a: &FunnelActorsQuery,
) -> AppResult<Value> {
    let q = &a.query;
    if a.step >= q.steps.len() || (a.dropped && a.step == 0) {
        return Err(bad_request("invalid step for actors"));
    }
    if !(1..=1000).contains(&a.limit) {
        return Err(bad_request("limit must be 1-1000"));
    }
    let outcomes = load_outcomes(db, project_id, q).await?;
    let wanted_bd = a.breakdown_value.as_deref();
    let groups = group_by_breakdown(q, &outcomes);
    let mut visitors: Vec<&Outcome> = groups
        .iter()
        .filter(|(bd, _)| q.breakdown.is_none() || breakdown_label(bd).as_str() == wanted_bd)
        .flat_map(|(_, g)| g.iter().copied())
        .filter(|o| {
            if a.dropped {
                o.reached == a.step
            } else {
                o.reached > a.step
            }
        })
        .collect();
    visitors.sort_by(|x, y| x.visitor_id.cmp(&y.visitor_id));
    let total = visitors.len();
    let page: Vec<&Outcome> = visitors.into_iter().skip(a.offset).take(a.limit).collect();
    let ids: Vec<String> = page.iter().map(|o| o.visitor_id.clone()).collect();
    let profiles = load_profiles(db, project_id, &ids).await?;
    let data: Vec<Value> = page
        .iter()
        .map(|o| {
            let (user_id, traits) = profiles
                .get(&o.visitor_id)
                .cloned()
                .unwrap_or((None, json!({})));
            json!({
                "visitor_id": o.visitor_id,
                "user_id": user_id,
                "traits": traits,
                "steps_reached": o.reached,
                "entered_at": DateTime::<Utc>::from_timestamp(o.times[0] as i64, 0),
            })
        })
        .collect();
    Ok(json!({ "data": data, "total": total, "limit": a.limit, "offset": a.offset }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(order: FunnelOrder, steps: usize, window_days: i64) -> FunnelsQuery {
        serde_json::from_value(json!({
            "steps": (0..steps).map(|i| json!({"event": format!("s{i}")})).collect::<Vec<_>>(),
            "date_from": "2026-11-01T00:00:00Z",
            "date_to": "2026-11-30T00:00:00Z",
            "funnel_window": window_days,
            "order": order,
        }))
        .unwrap()
    }

    /// Build events from (day offset, matched step) pairs; `None` = other event.
    fn evs(n: usize, items: &[(f64, Option<usize>)]) -> Vec<Ev> {
        items
            .iter()
            .map(|(d, s)| Ev {
                ts: d * 86_400.0,
                steps: (0..n).map(|i| Some(i) == *s).collect(),
                excl: vec![false],
                bd: String::new(),
                bucket: String::new(),
            })
            .collect()
    }

    fn best_attempt_t(e: &[Ev], q: &FunnelsQuery) -> Attempt {
        best_attempt(e, q, f64::MAX)
    }

    #[test]
    fn entries_after_date_to_are_ignored() {
        let q2 = q(FunnelOrder::Ordered, 2, 14);
        let e = evs(
            2,
            &[
                (0.0, Some(0)),
                (10.0, Some(0)),
                (11.0, Some(1)),
                (30.0, Some(1)),
            ],
        );
        // the day-10 entry is past the limit, but the day-0 entry converts at day 11 (within 14d)
        assert_eq!(best_attempt(&e, &q2, 5.0 * 86_400.0).reached, 2);
        let e = evs(2, &[(10.0, Some(0)), (11.0, Some(1))]);
        assert_eq!(best_attempt(&e, &q2, 5.0 * 86_400.0).reached, 0);
    }

    #[test]
    fn ordered_requires_order_and_window() {
        let q3 = q(FunnelOrder::Ordered, 3, 14);
        // step1 before step0 does not count
        assert_eq!(
            best_attempt_t(&evs(3, &[(0.0, Some(1)), (1.0, Some(0))]), &q3).reached,
            1
        );
        // in order within window
        assert_eq!(
            best_attempt_t(
                &evs(
                    3,
                    &[(0.0, Some(0)), (1.0, None), (2.0, Some(1)), (3.0, Some(2))]
                ),
                &q3
            )
            .reached,
            3
        );
        // step 2 outside 14 days of entry
        assert_eq!(
            best_attempt_t(
                &evs(3, &[(0.0, Some(0)), (1.0, Some(1)), (20.0, Some(2))]),
                &q3
            )
            .reached,
            2
        );
        // a later re-entry can complete the funnel
        assert_eq!(
            best_attempt_t(
                &evs(
                    3,
                    &[
                        (0.0, Some(0)),
                        (15.0, Some(0)),
                        (16.0, Some(1)),
                        (17.0, Some(2))
                    ]
                ),
                &q3
            )
            .reached,
            3
        );
    }

    #[test]
    fn strict_breaks_on_interruption() {
        let qs = q(FunnelOrder::Strict, 2, 14);
        assert_eq!(
            best_attempt_t(&evs(2, &[(0.0, Some(0)), (1.0, None), (2.0, Some(1))]), &qs).reached,
            1
        );
        assert_eq!(
            best_attempt_t(&evs(2, &[(0.0, Some(0)), (2.0, Some(1))]), &qs).reached,
            2
        );
    }

    #[test]
    fn unordered_counts_distinct_steps() {
        let qu = q(FunnelOrder::Unordered, 3, 14);
        assert_eq!(
            best_attempt_t(&evs(3, &[(0.0, Some(2)), (1.0, Some(0))]), &qu).reached,
            2
        );
        assert_eq!(
            best_attempt_t(&evs(3, &[(0.0, Some(2)), (20.0, Some(0))]), &qu).reached,
            1
        );
    }

    #[test]
    fn exclusion_between_steps_removes_attempt() {
        let mut qx = q(FunnelOrder::Ordered, 2, 14);
        qx.exclusions = vec![Exclusion {
            event: Some("x".into()),
            properties: vec![],
            from_step: 0,
            to_step: 1,
        }];
        let mut e = evs(2, &[(0.0, Some(0)), (1.0, None), (2.0, Some(1))]);
        e[1].excl = vec![true];
        assert_eq!(best_attempt_t(&e, &qx).reached, 0);
        // exclusion after conversion does not matter
        let mut e = evs(2, &[(0.0, Some(0)), (2.0, Some(1)), (3.0, None)]);
        e[2].excl = vec![true];
        assert_eq!(best_attempt_t(&e, &qx).reached, 2);
    }

    #[test]
    fn unordered_overlapping_predicates_use_distinct_events() {
        let qu = q(FunnelOrder::Unordered, 2, 14);
        // event 0 matches both steps, event 1 matches only step 0
        let mut e = evs(2, &[(0.0, Some(0)), (1.0, Some(0))]);
        e[0].steps = vec![true, true];
        assert_eq!(best_attempt_t(&e, &qu).reached, 2);
    }

    #[test]
    fn many_starts_stay_fast() {
        let q2 = q(FunnelOrder::Ordered, 2, 14);
        let e = evs(
            2,
            &(0..100_000)
                .map(|i| (i as f64 / 1_000.0, Some(0)))
                .collect::<Vec<_>>(),
        );
        let t = std::time::Instant::now();
        assert_eq!(best_attempt_t(&e, &q2).reached, 1);
        assert!(t.elapsed().as_secs() < 2, "took {:?}", t.elapsed());
    }

    #[test]
    fn validation() {
        let mut v = q(FunnelOrder::Ordered, 1, 14);
        assert!(validate(&v).is_err());
        v = q(FunnelOrder::Ordered, 3, 14);
        assert!(validate(&v).is_ok());
        v.funnel_window = 0;
        assert!(validate(&v).is_err());
        v = q(FunnelOrder::Ordered, 3, 14);
        v.exclusions = vec![Exclusion {
            event: None,
            properties: vec![],
            from_step: 2,
            to_step: 1,
        }];
        assert!(validate(&v).is_err());
    }
}
