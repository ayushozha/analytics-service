//! Trends insight: any event as a series, math, property filters, breakdowns,
//! intervals, timezone, compare-to-previous, formulas, cumulative and
//! smoothing — modelled on PostHog's `TrendsQuery`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::filters::{
    push_and_filters, push_group, push_numeric_expr, push_property_expr, validate_property,
    DataNeeds, FilterGroup, Joins, PropertyFilter,
};
use super::{
    bad_request, formula, map_query_error, push_event_source, EventSource, PropertyType,
    BREAKDOWN_OTHER, DEFAULT_BREAKDOWN_LIMIT, MAX_BREAKDOWN_LIMIT, MAX_BUCKETS, MAX_SERIES,
};
use crate::error::AppResult;
use crate::services::groups::MAX_GROUP_TYPES;

/// Label used for events whose breakdown property is missing.
const BREAKDOWN_NONE: &str = "$$_none";
/// Distinct breakdown values considered per series when ranking.
const MAX_BREAKDOWN_CANDIDATES: i64 = 10_000;

/// Top breakdown values shared by every series of the query, ranked by total
/// events across all series, plus whether an "other" bucket is needed. One
/// shared set keeps formula operands and the "other" bucket comparable.
pub(crate) async fn shared_breakdown(
    db: &PgPool,
    project_id: Uuid,
    q: &TrendsQuery,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<(Vec<String>, bool)> {
    let Some(b) = &q.breakdown else {
        return Ok((Vec::new(), false));
    };
    let limit = b.limit.unwrap_or(DEFAULT_BREAKDOWN_LIMIT) as usize;
    let mut totals: HashMap<String, i64> = HashMap::new();
    for series in &q.series {
        let ctx = Ctx::new(project_id, q, series, from, to);
        for (bd, n) in ctx.breakdown_counts(db).await? {
            *totals.entry(bd).or_default() += n;
        }
    }
    let mut ranked: Vec<(String, i64)> = totals.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let more = ranked.len() > limit;
    Ok((ranked.into_iter().take(limit).map(|r| r.0).collect(), more))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Interval {
    Minute,
    Hour,
    #[default]
    Day,
    Week,
    Month,
}

impl Interval {
    pub(crate) fn as_sql(self) -> &'static str {
        match self {
            Interval::Minute => "minute",
            Interval::Hour => "hour",
            Interval::Day => "day",
            Interval::Week => "week",
            Interval::Month => "month",
        }
    }

    /// Shortest possible length, used to bound the bucket count.
    pub(crate) fn min_seconds(self) -> i64 {
        match self {
            Interval::Minute => 60,
            Interval::Hour => 3_600,
            Interval::Day => 86_400,
            Interval::Week => 7 * 86_400,
            Interval::Month => 28 * 86_400,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Math {
    /// Number of events.
    #[default]
    Total,
    /// Unique visitors.
    Dau,
    UniqueSessions,
    /// Unique groups of type `math_group_type_index` (e.g. "unique companies").
    UniqueGroup,
    Sum,
    Avg,
    Min,
    Max,
    Median,
    P75,
    P90,
    P95,
    P99,
    /// Events per visitor, aggregated across visitors.
    AvgCountPerActor,
    MinCountPerActor,
    MaxCountPerActor,
    MedianCountPerActor,
    P90CountPerActor,
    P95CountPerActor,
    P99CountPerActor,
}

impl Math {
    fn needs_property(self) -> bool {
        matches!(
            self,
            Math::Sum
                | Math::Avg
                | Math::Min
                | Math::Max
                | Math::Median
                | Math::P75
                | Math::P90
                | Math::P95
                | Math::P99
        )
    }

    fn per_actor(self) -> bool {
        matches!(
            self,
            Math::AvgCountPerActor
                | Math::MinCountPerActor
                | Math::MaxCountPerActor
                | Math::MedianCountPerActor
                | Math::P90CountPerActor
                | Math::P95CountPerActor
                | Math::P99CountPerActor
        )
    }

    /// Aggregate applied to a numeric value (a property, or a per-actor count).
    fn value_agg(self) -> &'static str {
        match self {
            Math::Sum => "SUM",
            Math::Avg | Math::AvgCountPerActor => "AVG",
            Math::Min | Math::MinCountPerActor => "MIN",
            Math::Max | Math::MaxCountPerActor => "MAX",
            Math::Median | Math::MedianCountPerActor => "percentile_cont(0.5) WITHIN GROUP",
            Math::P75 => "percentile_cont(0.75) WITHIN GROUP",
            Math::P90 | Math::P90CountPerActor => "percentile_cont(0.9) WITHIN GROUP",
            Math::P95 | Math::P95CountPerActor => "percentile_cont(0.95) WITHIN GROUP",
            Math::P99 | Math::P99CountPerActor => "percentile_cont(0.99) WITHIN GROUP",
            Math::Total | Math::Dau | Math::UniqueSessions | Math::UniqueGroup => "COUNT",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Series {
    /// Event name; `$pageview` for pageviews; `null` for all events.
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub custom_name: Option<String>,
    #[serde(default)]
    pub math: Math,
    #[serde(default)]
    pub math_property: Option<String>,
    #[serde(default)]
    pub math_property_type: PropertyType,
    /// Group type for `math: unique_group`, or for a group `math_property`.
    #[serde(default)]
    pub math_group_type_index: Option<i16>,
    /// Filters that apply to this series only.
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Breakdown {
    pub property: String,
    #[serde(rename = "type", default)]
    pub property_type: PropertyType,
    /// Group type for `type: "group"` breakdowns.
    #[serde(default)]
    pub group_type_index: Option<i16>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrendsQuery {
    pub series: Vec<Series>,
    pub date_from: DateTime<Utc>,
    pub date_to: DateTime<Utc>,
    #[serde(default)]
    pub interval: Interval,
    /// IANA timezone used for bucketing (e.g. `America/New_York`).
    #[serde(default = "default_tz")]
    pub timezone: String,
    /// Flat AND filters applied to every series.
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    /// Nested AND/OR filters applied to every series.
    #[serde(default)]
    pub filter_group: Option<FilterGroup>,
    #[serde(default)]
    pub breakdown: Option<Breakdown>,
    /// Also return the previous period of equal length.
    #[serde(default)]
    pub compare: bool,
    /// e.g. `A / B * 100`; when set, only formula results are returned.
    #[serde(default)]
    pub formula: Option<String>,
    #[serde(default)]
    pub cumulative: bool,
    /// Moving average over this many intervals (1 = off).
    #[serde(default)]
    pub smoothing_intervals: Option<usize>,
}

impl TrendsQuery {
    /// Protected data the query reads (person properties, cohorts).
    pub fn data_needs(&self) -> DataNeeds {
        let mut j = Joins::default();
        j.add_filters(&self.properties);
        if let Some(g) = &self.filter_group {
            j.add_group(g);
        }
        if let Some(b) = &self.breakdown {
            j.add(b.property_type, b.group_type_index);
        }
        for s in &self.series {
            j.add_filters(&s.properties);
            if s.math.needs_property() {
                j.add(s.math_property_type, s.math_group_type_index);
            }
        }
        j.into()
    }
}

fn default_tz() -> String {
    "UTC".to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendResult {
    pub label: String,
    pub series_index: Option<usize>,
    pub event: Option<String>,
    pub math: Option<Math>,
    pub breakdown_value: Option<String>,
    pub compare_label: Option<&'static str>,
    /// Bucket starts in the query timezone (`YYYY-MM-DDTHH:MM:SS`).
    pub days: Vec<String>,
    pub data: Vec<f64>,
    /// Sum of `data` (as PostHog reports it).
    pub count: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendsResponse {
    pub results: Vec<TrendResult>,
    pub interval: Interval,
    pub timezone: String,
}

/// Validate the query shape up front so SQL is only built for sane input.
pub fn validate(q: &TrendsQuery) -> AppResult<()> {
    if q.series.is_empty() || q.series.len() > MAX_SERIES {
        return Err(bad_request(format!(
            "series must contain 1-{MAX_SERIES} items"
        )));
    }
    if q.date_to <= q.date_from {
        return Err(bad_request("date_to must be after date_from"));
    }
    let buckets = (q.date_to - q.date_from).num_seconds() / q.interval.min_seconds() + 1;
    if buckets > MAX_BUCKETS {
        return Err(bad_request(format!(
            "{buckets} {} buckets requested; use a larger interval (max {MAX_BUCKETS})",
            q.interval.as_sql()
        )));
    }
    if q.timezone.is_empty() || q.timezone.len() > 64 {
        return Err(bad_request("invalid timezone"));
    }
    for s in &q.series {
        if let Some(e) = &s.event {
            if e.is_empty() || e.len() > 255 {
                return Err(bad_request("event name must be 1-255 characters"));
            }
        }
        if s.math.needs_property() {
            let key = s
                .math_property
                .as_deref()
                .ok_or_else(|| bad_request("this math needs math_property"))?;
            validate_property(key, s.math_property_type, s.math_group_type_index)?;
        }
        if s.math == Math::UniqueGroup
            && !s
                .math_group_type_index
                .is_some_and(|i| (0..MAX_GROUP_TYPES).contains(&i))
        {
            return Err(bad_request(format!(
                "unique_group math needs math_group_type_index 0-{}",
                MAX_GROUP_TYPES - 1
            )));
        }
    }
    if let Some(b) = &q.breakdown {
        validate_property(&b.property, b.property_type, b.group_type_index)?;
        let limit = b.limit.unwrap_or(DEFAULT_BREAKDOWN_LIMIT);
        if !(1..=MAX_BREAKDOWN_LIMIT).contains(&limit) {
            return Err(bad_request(format!(
                "breakdown limit must be 1-{MAX_BREAKDOWN_LIMIT}"
            )));
        }
    }
    if let Some(n) = q.smoothing_intervals {
        if n == 0 || n > 100 {
            return Err(bad_request("smoothing_intervals must be 1-100"));
        }
    }
    if let Some(f) = &q.formula {
        formula::parse(f, q.series.len())?;
    }
    Ok(())
}

/// Shared context for building one series' SQL.
pub(crate) struct Ctx<'a> {
    project_id: Uuid,
    q: &'a TrendsQuery,
    series: &'a Series,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    joins: Joins,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(
        project_id: Uuid,
        q: &'a TrendsQuery,
        series: &'a Series,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Self {
        let mut joins = Joins::default();
        joins.add_filters(&q.properties);
        joins.add_filters(&series.properties);
        if let Some(g) = &q.filter_group {
            joins.add_group(g);
        }
        if let Some(b) = &q.breakdown {
            joins.add(b.property_type, b.group_type_index);
        }
        if series.math.needs_property() {
            joins.add(series.math_property_type, series.math_group_type_index);
        }
        Self {
            project_id,
            q,
            series,
            from,
            to,
            joins,
        }
    }

    /// `FROM (<virtual events>) ev [joins] WHERE <filters>`
    pub(crate) fn push_from_where(&self, qb: &mut QueryBuilder<'_, Postgres>) -> AppResult<()> {
        let events: Vec<String> = self.series.event.iter().cloned().collect();
        push_event_source(
            qb,
            &EventSource {
                project_id: self.project_id,
                from: self.from,
                to: self.to,
                events: (!events.is_empty()).then_some(events.as_slice()),
                joins: self.joins,
            },
        );
        qb.push(" WHERE TRUE");
        push_and_filters(qb, &self.q.properties)?;
        push_and_filters(qb, &self.series.properties)?;
        if let Some(g) = &self.q.filter_group {
            push_group(qb, g)?;
        }
        Ok(())
    }

    pub(crate) fn push_bucket(&self, qb: &mut QueryBuilder<'_, Postgres>) {
        qb.push("to_char(date_trunc(");
        qb.push_bind(self.q.interval.as_sql());
        qb.push(", ev.created_at AT TIME ZONE ");
        qb.push_bind(self.q.timezone.clone());
        qb.push("), 'YYYY-MM-DD\"T\"HH24:MI:SS')");
    }

    /// Raw breakdown value with missing values mapped to `$$_none`.
    pub(crate) fn push_breakdown_raw(&self, qb: &mut QueryBuilder<'_, Postgres>) {
        match &self.q.breakdown {
            Some(b) => {
                qb.push("COALESCE(");
                push_property_expr(qb, &b.property, b.property_type, b.group_type_index);
                qb.push(", '$$_none')");
            }
            None => {
                qb.push("NULL::text");
            }
        }
    }

    /// The aggregate for event-level math (not per-actor).
    fn push_event_agg(&self, qb: &mut QueryBuilder<'_, Postgres>) {
        let m = self.series.math;
        match m {
            Math::Total => {
                qb.push("COUNT(*)::double precision");
            }
            Math::Dau => {
                qb.push("COUNT(DISTINCT ev.visitor_id)::double precision");
            }
            Math::UniqueSessions => {
                qb.push("COUNT(DISTINCT ev.session_id)::double precision");
            }
            Math::UniqueGroup => {
                // Validated to 0..MAX_GROUP_TYPES; NULL keys are not counted.
                let i = self.series.math_group_type_index.unwrap_or_default();
                qb.push(format!(
                    "COUNT(DISTINCT ev.properties ->> '$group_{i}')::double precision"
                ));
            }
            _ => {
                let key = self.series.math_property.as_deref().unwrap_or_default();
                let t = self.series.math_property_type;
                let g = self.series.math_group_type_index;
                let agg = m.value_agg();
                if agg.ends_with("WITHIN GROUP") {
                    qb.push(format!("{agg} (ORDER BY "));
                    push_numeric_expr(qb, key, t, g);
                    qb.push(")");
                } else {
                    qb.push(format!("{agg}("));
                    push_numeric_expr(qb, key, t, g);
                    qb.push(")");
                }
            }
        }
    }

    /// Event counts per breakdown value for this series (capped).
    async fn breakdown_counts(&self, db: &PgPool) -> AppResult<Vec<(String, i64)>> {
        let mut qb = QueryBuilder::<Postgres>::new("SELECT ");
        self.push_breakdown_raw(&mut qb);
        qb.push(" AS bd, COUNT(*) AS n");
        self.push_from_where(&mut qb)?;
        qb.push(" GROUP BY 1 ORDER BY 2 DESC, 1 LIMIT ");
        qb.push_bind(MAX_BREAKDOWN_CANDIDATES);
        qb.build_query_as()
            .fetch_all(db)
            .await
            .map_err(map_query_error)
    }

    /// Rows of `(bucket, breakdown_value, value)` for this series.
    async fn rows(&self, db: &PgPool, top: &[String]) -> AppResult<Vec<(String, String, f64)>> {
        let mut qb = QueryBuilder::<Postgres>::new("SELECT ");
        let push_bd = |qb: &mut QueryBuilder<'_, Postgres>| {
            if self.q.breakdown.is_some() {
                qb.push("CASE WHEN ");
                self.push_breakdown_raw(qb);
                qb.push(" = ANY(");
                qb.push_bind(top.to_vec());
                qb.push(") THEN ");
                self.push_breakdown_raw(qb);
                qb.push(format!(" ELSE '{BREAKDOWN_OTHER}' END"));
            } else {
                qb.push("''::text");
            }
        };
        if self.series.math.per_actor() {
            let agg = self.series.math.value_agg();
            qb.push("bucket, bd, ");
            if agg.ends_with("WITHIN GROUP") {
                qb.push(format!("({agg} (ORDER BY c))::double precision"));
            } else {
                qb.push(format!("{agg}(c)::double precision"));
            }
            qb.push(" FROM (SELECT ");
            self.push_bucket(&mut qb);
            qb.push(" AS bucket, ");
            push_bd(&mut qb);
            qb.push(" AS bd, ev.visitor_id, COUNT(*) AS c");
            self.push_from_where(&mut qb)?;
            qb.push(" GROUP BY 1, 2, 3) per_actor GROUP BY 1, 2");
        } else {
            self.push_bucket(&mut qb);
            qb.push(" AS bucket, ");
            push_bd(&mut qb);
            qb.push(" AS bd, ");
            self.push_event_agg(&mut qb);
            self.push_from_where(&mut qb)?;
            qb.push(" GROUP BY 1, 2");
        }
        let rows: Vec<(String, String, Option<f64>)> = qb
            .build_query_as()
            .fetch_all(db)
            .await
            .map_err(map_query_error)?;
        Ok(rows
            .into_iter()
            .map(|(b, bd, v)| (b, bd, v.unwrap_or(0.0)))
            .collect())
    }
}

/// Bucket starts covering the half-open range `[from, to)` in `timezone`.
pub(crate) async fn bucket_labels(
    db: &PgPool,
    interval: &Interval,
    timezone: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT to_char(g, 'YYYY-MM-DD\"T\"HH24:MI:SS') FROM generate_series(\
            date_trunc($1, $2 AT TIME ZONE $4), \
            date_trunc($1, ($3 - interval '1 microsecond') AT TIME ZONE $4), \
            ('1 ' || $1)::interval) g",
    )
    .bind(interval.as_sql())
    .bind(from)
    .bind(to)
    .bind(timezone)
    .fetch_all(db)
    .await
    .map_err(map_query_error)?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// One period's results: per series, per breakdown value, a value per bucket.
struct PeriodResult {
    days: Vec<String>,
    /// series index -> (breakdown value -> data aligned with `days`)
    series: Vec<Vec<(String, Vec<f64>)>>,
}

async fn run_period(
    db: &PgPool,
    project_id: Uuid,
    q: &TrendsQuery,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<PeriodResult> {
    let days = bucket_labels(db, &q.interval, &q.timezone, from, to).await?;
    let index: HashMap<&str, usize> = days
        .iter()
        .enumerate()
        .map(|(i, d)| (d.as_str(), i))
        .collect();
    let (top, more) = shared_breakdown(db, project_id, q, from, to).await?;
    let mut series_out = Vec::with_capacity(q.series.len());
    for series in &q.series {
        let ctx = Ctx::new(project_id, q, series, from, to);
        let rows = ctx.rows(db, &top).await?;
        let mut by_bd: HashMap<String, Vec<f64>> = HashMap::new();
        for (bucket, bd, value) in rows {
            if let Some(&i) = index.get(bucket.as_str()) {
                by_bd.entry(bd).or_insert_with(|| vec![0.0; days.len()])[i] = value;
            }
        }
        // Stable order: no breakdown, else top values by rank, then "other".
        let order: Vec<String> = if q.breakdown.is_none() {
            vec![String::new()]
        } else {
            let mut o = top.clone();
            if more {
                o.push(BREAKDOWN_OTHER.to_string());
            }
            o
        };
        series_out.push(
            order
                .into_iter()
                .map(|bd| {
                    let data = by_bd.remove(&bd).unwrap_or_else(|| vec![0.0; days.len()]);
                    (bd, data)
                })
                .collect(),
        );
    }
    Ok(PeriodResult {
        days,
        series: series_out,
    })
}

pub(crate) fn series_label(q: &TrendsQuery, i: usize) -> String {
    let s = &q.series[i];
    s.custom_name
        .clone()
        .or_else(|| s.event.clone())
        .unwrap_or_else(|| "All events".to_string())
}

pub(crate) fn breakdown_output(bd: &str) -> Option<String> {
    match bd {
        "" | BREAKDOWN_NONE => None,
        other => Some(other.to_string()),
    }
}

fn post_process(data: &mut [f64], q: &TrendsQuery) {
    if let Some(n) = q.smoothing_intervals.filter(|&n| n > 1) {
        let raw = data.to_vec();
        for i in 0..data.len() {
            let start = i.saturating_sub(n - 1);
            let window = &raw[start..=i];
            data[i] = window.iter().sum::<f64>() / window.len() as f64;
        }
    }
    if q.cumulative {
        let mut acc = 0.0;
        for v in data.iter_mut() {
            acc += *v;
            *v = acc;
        }
    }
}

fn period_results(
    q: &TrendsQuery,
    period: PeriodResult,
    compare_label: Option<&'static str>,
    formula: Option<&formula::Expr>,
) -> Vec<TrendResult> {
    let mut out = Vec::new();
    if let Some(expr) = formula {
        // Evaluate per breakdown value, aligning series by breakdown value.
        let mut bds: Vec<String> = Vec::new();
        for s in &period.series {
            for (bd, _) in s {
                if !bds.contains(bd) {
                    bds.push(bd.clone());
                }
            }
        }
        for bd in bds {
            let per_series: Vec<Option<&Vec<f64>>> = period
                .series
                .iter()
                .map(|s| s.iter().find(|(b, _)| *b == bd).map(|(_, d)| d))
                .collect();
            let mut data: Vec<f64> = (0..period.days.len())
                .map(|i| {
                    let values: Vec<f64> = per_series
                        .iter()
                        .map(|d| d.map(|d| d[i]).unwrap_or(0.0))
                        .collect();
                    formula::eval(expr, &values)
                })
                .collect();
            post_process(&mut data, q);
            out.push(TrendResult {
                label: format!("Formula ({})", q.formula.as_deref().unwrap_or_default()),
                series_index: None,
                event: None,
                math: None,
                breakdown_value: breakdown_output(&bd),
                compare_label,
                days: period.days.clone(),
                count: data.iter().sum(),
                data,
            });
        }
        return out;
    }
    for (i, series) in period.series.into_iter().enumerate() {
        for (bd, mut data) in series {
            post_process(&mut data, q);
            out.push(TrendResult {
                label: series_label(q, i),
                series_index: Some(i),
                event: q.series[i].event.clone(),
                math: Some(q.series[i].math),
                breakdown_value: breakdown_output(&bd),
                compare_label,
                days: period.days.clone(),
                count: data.iter().sum(),
                data,
            });
        }
    }
    out
}

pub async fn run_trends(
    db: &PgPool,
    project_id: Uuid,
    q: &TrendsQuery,
) -> AppResult<TrendsResponse> {
    validate(q)?;
    let expr = q
        .formula
        .as_deref()
        .map(|f| formula::parse(f, q.series.len()))
        .transpose()?;

    let current = run_period(db, project_id, q, q.date_from, q.date_to).await?;
    let mut results = period_results(q, current, q.compare.then_some("current"), expr.as_ref());
    if q.compare {
        let len = q.date_to - q.date_from;
        // Half-open windows: previous = [from - len, from), same bucket count.
        let prev_to = q.date_from;
        let prev_from = q.date_from - len;
        let previous = run_period(db, project_id, q, prev_from, prev_to).await?;
        results.extend(period_results(q, previous, Some("previous"), expr.as_ref()));
    }
    Ok(TrendsResponse {
        results,
        interval: q.interval,
        timezone: q.timezone.clone(),
    })
}

/// Who is behind one data point (PostHog "persons on this day").
#[derive(Debug, Clone, Deserialize)]
pub struct TrendsActorsQuery {
    pub query: TrendsQuery,
    #[serde(default)]
    pub series: usize,
    /// Bucket start as returned in `days`.
    pub day: String,
    /// Breakdown value of the clicked line (omit / null for none).
    #[serde(default)]
    pub breakdown_value: Option<String>,
    #[serde(default = "default_actor_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_actor_limit() -> i64 {
    100
}

pub async fn run_trends_actors(
    db: &PgPool,
    project_id: Uuid,
    a: &TrendsActorsQuery,
) -> AppResult<Value> {
    validate(&a.query)?;
    let series = a
        .query
        .series
        .get(a.series)
        .ok_or_else(|| bad_request("series index out of range"))?;
    if !(1..=1000).contains(&a.limit) || a.offset < 0 {
        return Err(bad_request("limit must be 1-1000 and offset >= 0"));
    }
    let mut ctx = Ctx::new(
        project_id,
        &a.query,
        series,
        a.query.date_from,
        a.query.date_to,
    );
    // `unique_group` counts groups, so its actors are groups rather than people.
    let group = (series.math == Math::UniqueGroup)
        .then_some(series.math_group_type_index)
        .flatten();
    let actor = match group {
        Some(i) => {
            ctx.joins.groups |= 1 << i;
            format!("(ev.properties ->> '$group_{i}')")
        }
        None => {
            ctx.joins.person = true;
            "ev.visitor_id".to_string()
        }
    };
    let profile = match group {
        Some(i) => format!("NULL::text AS user_id, (array_agg(g{i}.properties))[1] AS traits"),
        None => "MAX(up.user_id) AS user_id, (array_agg(up.traits))[1] AS traits".to_string(),
    };
    let mut qb = QueryBuilder::<Postgres>::new(format!(
        "SELECT {actor} AS actor, {profile}, COUNT(*) AS event_count, \
         MIN(ev.created_at) AS first_at, MAX(ev.created_at) AS last_at"
    ));
    ctx.push_from_where(&mut qb)?;
    qb.push(format!(" AND {actor} IS NOT NULL"));
    qb.push(" AND ");
    ctx.push_bucket(&mut qb);
    qb.push(" = ");
    qb.push_bind(a.day.clone());
    if a.query.breakdown.is_some() {
        let value = a
            .breakdown_value
            .clone()
            .unwrap_or_else(|| BREAKDOWN_NONE.to_string());
        if value == BREAKDOWN_OTHER {
            let (top, _) =
                shared_breakdown(db, project_id, &a.query, a.query.date_from, a.query.date_to)
                    .await?;
            qb.push(" AND NOT (");
            ctx.push_breakdown_raw(&mut qb);
            qb.push(" = ANY(");
            qb.push_bind(top);
            qb.push("))");
        } else {
            qb.push(" AND ");
            ctx.push_breakdown_raw(&mut qb);
            qb.push(" = ");
            qb.push_bind(value);
        }
    }
    qb.push(format!(
        " GROUP BY {actor} ORDER BY event_count DESC, {actor} LIMIT "
    ));
    qb.push_bind(a.limit);
    qb.push(" OFFSET ");
    qb.push_bind(a.offset);

    type Row = (
        String,
        Option<String>,
        Option<Value>,
        i64,
        DateTime<Utc>,
        DateTime<Utc>,
    );
    let rows: Vec<Row> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    let actors: Vec<Value> = rows
        .into_iter()
        .map(
            |(actor, user_id, traits, event_count, first_at, last_at)| match group {
                Some(i) => json!({
                    "group_type_index": i,
                    "group_key": actor,
                    "properties": traits.unwrap_or_else(|| json!({})),
                    "event_count": event_count,
                    "first_at": first_at,
                    "last_at": last_at,
                }),
                None => json!({
                    "visitor_id": actor,
                    "user_id": user_id,
                    "traits": traits.unwrap_or_else(|| json!({})),
                    "event_count": event_count,
                    "first_at": first_at,
                    "last_at": last_at,
                }),
            },
        )
        .collect();
    Ok(json!({ "data": actors, "limit": a.limit, "offset": a.offset }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(v: Value) -> TrendsQuery {
        serde_json::from_value(v).unwrap()
    }

    fn base() -> Value {
        json!({
            "series": [{"event": "signup"}],
            "date_from": "2026-11-01T00:00:00Z",
            "date_to": "2026-11-08T00:00:00Z"
        })
    }

    #[test]
    fn validates_ranges_and_buckets() {
        assert!(validate(&query(base())).is_ok());
        let mut v = base();
        v["interval"] = json!("minute");
        v["date_to"] = json!("2026-12-31T00:00:00Z");
        assert!(validate(&query(v)).is_err());
        let mut v = base();
        v["date_to"] = json!("2026-10-01T00:00:00Z");
        assert!(validate(&query(v)).is_err());
    }

    #[test]
    fn property_math_requires_property() {
        let mut v = base();
        v["series"] = json!([{"event": "purchase", "math": "sum"}]);
        assert!(validate(&query(v)).is_err());
        let mut v = base();
        v["series"] = json!([{"event": "purchase", "math": "p95", "math_property": "revenue"}]);
        assert!(validate(&query(v)).is_ok());
    }

    #[test]
    fn source_only_scans_needed_tables() {
        let q = query(base());
        let ctx = Ctx::new(Uuid::nil(), &q, &q.series[0], q.date_from, q.date_to);
        let mut qb = QueryBuilder::<Postgres>::new("SELECT 1");
        ctx.push_from_where(&mut qb).unwrap();
        let sql = qb.sql().to_string();
        assert!(sql.contains("FROM events e"));
        assert!(!sql.contains("FROM pageviews p"));
        assert!(!sql.contains("LEFT JOIN"));

        let mut v = base();
        v["series"] = json!([{"event": "$pageview"}]);
        v["breakdown"] = json!({"property": "country", "type": "session"});
        let q = query(v);
        let ctx = Ctx::new(Uuid::nil(), &q, &q.series[0], q.date_from, q.date_to);
        let mut qb = QueryBuilder::<Postgres>::new("SELECT 1");
        ctx.push_from_where(&mut qb).unwrap();
        let sql = qb.sql().to_string();
        assert!(sql.contains("FROM pageviews p") && !sql.contains("FROM events e"));
        assert!(sql.contains("LEFT JOIN sessions s"));
    }

    #[test]
    fn smoothing_then_cumulative() {
        let mut q = query(base());
        q.smoothing_intervals = Some(2);
        let mut d = vec![2.0, 4.0, 6.0];
        post_process(&mut d, &q);
        assert_eq!(d, vec![2.0, 3.0, 5.0]);
        q.smoothing_intervals = None;
        q.cumulative = true;
        let mut d = vec![1.0, 2.0, 3.0];
        post_process(&mut d, &q);
        assert_eq!(d, vec![1.0, 3.0, 6.0]);
    }
}
