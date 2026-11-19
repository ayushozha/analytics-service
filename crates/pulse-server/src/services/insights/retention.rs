//! Retention insight (PostHog `RetentionQuery`): pick a start event and a
//! returning event, first-time or recurring retention, any period, cumulative
//! ("on or after") mode, property filters, breakdowns, and the visitors behind
//! each cell.

use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::filters::{FilterGroup, PropertyFilter};
use super::trends::{
    breakdown_output, bucket_labels, shared_breakdown, Breakdown, Ctx, Interval, Math, Series,
    TrendsQuery,
};
use super::{bad_request, load_actors, map_query_error, BREAKDOWN_OTHER};
use crate::error::AppResult;

pub const MAX_INTERVALS: usize = 120;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionEntity {
    /// Event name; `$pageview`; `null` for any event.
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetentionType {
    /// Cohort = the period of a visitor's first-ever start event.
    #[default]
    FirstTime,
    /// Every period with a start event starts a cohort entry.
    Recurring,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionQuery {
    pub target: RetentionEntity,
    /// Defaults to the target event.
    #[serde(default)]
    pub returning: Option<RetentionEntity>,
    #[serde(default)]
    pub retention_type: RetentionType,
    #[serde(default)]
    pub period: Interval,
    #[serde(default = "default_intervals")]
    pub total_intervals: usize,
    /// Cohorts start in `[date_from, date_to)`.
    pub date_from: DateTime<Utc>,
    pub date_to: DateTime<Utc>,
    #[serde(default = "default_tz")]
    pub timezone: String,
    #[serde(default)]
    pub properties: Vec<PropertyFilter>,
    #[serde(default)]
    pub filter_group: Option<FilterGroup>,
    #[serde(default)]
    pub breakdown: Option<Breakdown>,
    /// Count a visitor in interval N if they returned in N or any later interval.
    #[serde(default)]
    pub cumulative: bool,
    /// Retain groups of this type instead of people.
    #[serde(default)]
    pub aggregation_group_type_index: Option<i16>,
}

fn default_intervals() -> usize {
    8
}

fn default_tz() -> String {
    "UTC".to_string()
}

impl RetentionQuery {
    /// A TrendsQuery carrying the shared filters/breakdown, with series 0 =
    /// target and series 1 = returning, so the trends SQL builder is reused.
    fn as_trends(&self, date_to: DateTime<Utc>) -> TrendsQuery {
        let series = |e: &RetentionEntity| Series {
            event: e.event.clone(),
            custom_name: None,
            math: Math::Total,
            math_property: None,
            math_property_type: Default::default(),
            math_group_type_index: None,
            properties: e.properties.clone(),
        };
        TrendsQuery {
            series: vec![
                series(&self.target),
                series(self.returning.as_ref().unwrap_or(&self.target)),
            ],
            date_from: self.date_from,
            date_to,
            interval: self.period,
            timezone: self.timezone.clone(),
            properties: self.properties.clone(),
            filter_group: self.filter_group.clone(),
            breakdown: self.breakdown.clone(),
            compare: false,
            formula: None,
            cumulative: false,
            smoothing_intervals: None,
            aggregation_group_type_index: self.aggregation_group_type_index,
        }
    }

    /// End of the period in which the last cohort's last interval falls.
    fn horizon(&self) -> AppResult<DateTime<Utc>> {
        let secs = self.period.min_seconds() * self.total_intervals as i64;
        let extra = if self.period == Interval::Month {
            // Months are at most 31 days; the extra bucket is trimmed by labels.
            31 * 86_400 * self.total_intervals as i64
        } else {
            secs
        };
        self.date_to
            .checked_add_signed(chrono::Duration::seconds(extra))
            .ok_or_else(|| bad_request("date range is out of bounds"))
    }

    pub fn data_needs(&self) -> super::filters::DataNeeds {
        self.as_trends(self.date_to).data_needs()
    }
}

pub fn validate(q: &RetentionQuery) -> AppResult<()> {
    if q.date_to <= q.date_from {
        return Err(bad_request("date_to must be after date_from"));
    }
    if !(1..=MAX_INTERVALS).contains(&q.total_intervals) {
        return Err(bad_request(format!(
            "total_intervals must be 1-{MAX_INTERVALS}"
        )));
    }
    if q.period == Interval::Minute {
        return Err(bad_request(
            "retention period must be hour, day, week or month",
        ));
    }
    let t = q.as_trends(q.horizon()?);
    super::trends::validate(&t)
}

/// Per visitor: periods with a start event, periods with a return event, and
/// the breakdown value at each start period.
#[derive(Default)]
struct Activity {
    starts: BTreeSet<usize>,
    returns: HashSet<usize>,
    bd_at: HashMap<usize, String>,
}

struct Cell {
    cohort: usize,
    bd: String,
    visitor: String,
    /// Intervals (0..) in which this cohort entry counts as retained.
    retained: Vec<usize>,
}

async fn load(
    db: &PgPool,
    project_id: Uuid,
    q: &RetentionQuery,
) -> AppResult<(Vec<String>, usize, Vec<Cell>, Vec<String>, bool)> {
    validate(q)?;
    let horizon = q.horizon()?;
    let t = q.as_trends(horizon);
    let labels = bucket_labels(db, &q.period, &q.timezone, q.date_from, horizon).await?;
    let cohort_labels = bucket_labels(db, &q.period, &q.timezone, q.date_from, q.date_to).await?;
    let index: HashMap<&str, usize> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| (l.as_str(), i))
        .collect();

    let mut act: HashMap<String, Activity> = HashMap::new();
    for (si, (from, to)) in [(q.date_from, q.date_to), (q.date_from, horizon)]
        .into_iter()
        .enumerate()
    {
        let ctx = Ctx::new(project_id, &t, &t.series[si], from, to);
        let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {}, ", ctx.actor()));
        ctx.push_bucket(&mut qb);
        qb.push(" AS bucket, MIN(");
        ctx.push_breakdown_raw(&mut qb);
        qb.push(") AS bd");
        ctx.push_from_where(&mut qb)?;
        qb.push(" GROUP BY 1, 2");
        let rows: Vec<(String, String, Option<String>)> = qb
            .build_query_as()
            .fetch_all(db)
            .await
            .map_err(map_query_error)?;
        for (visitor, bucket, bd) in rows {
            let Some(&i) = index.get(bucket.as_str()) else {
                continue;
            };
            let a = act.entry(visitor).or_default();
            if si == 0 {
                a.starts.insert(i);
                a.bd_at.insert(i, bd.unwrap_or_default());
            } else {
                a.returns.insert(i);
            }
        }
    }

    // First-time retention: drop visitors whose first start event predates the range.
    if q.retention_type == RetentionType::FirstTime {
        let epoch = DateTime::<Utc>::UNIX_EPOCH;
        let ctx = Ctx::new(project_id, &t, &t.series[0], epoch, q.date_from);
        let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT DISTINCT {}", ctx.actor()));
        ctx.push_from_where(&mut qb)?;
        let earlier: Vec<(String,)> = qb
            .build_query_as()
            .fetch_all(db)
            .await
            .map_err(map_query_error)?;
        for (v,) in earlier {
            act.remove(&v);
        }
    }

    let (top, more) = if q.breakdown.is_some() {
        shared_breakdown(db, project_id, &t, q.date_from, q.date_to).await?
    } else {
        (Vec::new(), false)
    };
    let bucket_bd = |bd: &str| -> String {
        if q.breakdown.is_none() || top.iter().any(|t| t == bd) {
            bd.to_string()
        } else {
            BREAKDOWN_OTHER.to_string()
        }
    };

    let n_labels = labels.len();
    let mut cells = Vec::new();
    for (visitor, a) in act {
        let cohorts: Vec<usize> = match q.retention_type {
            RetentionType::FirstTime => a.starts.iter().next().copied().into_iter().collect(),
            RetentionType::Recurring => a.starts.iter().copied().collect(),
        };
        for c in cohorts {
            let max_i = q.total_intervals.min(n_labels - c);
            let returned: Vec<bool> = (0..max_i)
                .map(|i| i == 0 || a.returns.contains(&(c + i)))
                .collect();
            let retained: Vec<usize> = (0..max_i)
                .filter(|&i| {
                    if q.cumulative {
                        returned[i..].iter().any(|&r| r)
                    } else {
                        returned[i]
                    }
                })
                .collect();
            cells.push(Cell {
                cohort: c,
                bd: bucket_bd(a.bd_at.get(&c).map(String::as_str).unwrap_or_default()),
                visitor: visitor.clone(),
                retained,
            });
        }
    }
    Ok((labels, cohort_labels.len(), cells, top, more))
}

pub async fn run_retention(db: &PgPool, project_id: Uuid, q: &RetentionQuery) -> AppResult<Value> {
    let (labels, n_cohorts, cells, top, more) = load(db, project_id, q).await?;
    let mut groups: Vec<String> = if q.breakdown.is_some() {
        top
    } else {
        vec![String::new()]
    };
    if more {
        groups.push(BREAKDOWN_OTHER.to_string());
    }
    let mut results = Vec::new();
    for bd in &groups {
        for c in 0..n_cohorts {
            let max_i = q.total_intervals.min(labels.len() - c);
            let mut counts = vec![0_i64; max_i];
            for cell in cells.iter().filter(|x| x.cohort == c && &x.bd == bd) {
                for &i in &cell.retained {
                    counts[i] += 1;
                }
            }
            let size = counts.first().copied().unwrap_or(0);
            results.push(json!({
                "date": labels[c],
                "label": format!("{} {}", q.period.as_sql(), c),
                "breakdown_value": breakdown_output(bd),
                "values": counts.iter().enumerate().map(|(i, &n)| json!({
                    "interval": i,
                    "count": n,
                    "percentage": if size > 0 { n as f64 / size as f64 * 100.0 } else { 0.0 },
                })).collect::<Vec<_>>(),
            }));
        }
    }
    Ok(json!({
        "results": results,
        "period": q.period,
        "total_intervals": q.total_intervals,
        "retention_type": q.retention_type,
        "cumulative": q.cumulative,
    }))
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetentionActorsQuery {
    pub query: RetentionQuery,
    /// Index of the cohort row (0 = first period of the range).
    pub cohort: usize,
    /// Interval column (0 = cohort itself).
    #[serde(default)]
    pub interval: usize,
    #[serde(default)]
    pub breakdown_value: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

pub async fn run_retention_actors(
    db: &PgPool,
    project_id: Uuid,
    a: &RetentionActorsQuery,
) -> AppResult<Value> {
    if !(1..=1000).contains(&a.limit) {
        return Err(bad_request("limit must be 1-1000"));
    }
    let (_, _, cells, _, _) = load(db, project_id, &a.query).await?;
    let mut ids: Vec<String> = cells
        .into_iter()
        .filter(|c| {
            c.cohort == a.cohort
                && c.retained.contains(&a.interval)
                && (a.query.breakdown.is_none()
                    || breakdown_output(&c.bd).as_deref() == a.breakdown_value.as_deref())
        })
        .map(|c| c.visitor)
        .collect();
    ids.sort();
    ids.dedup();
    let total = ids.len();
    let page: Vec<String> = ids.into_iter().skip(a.offset).take(a.limit).collect();
    let data = load_actors(db, project_id, a.query.aggregation_group_type_index, &page).await?;
    Ok(json!({ "data": data, "total": total, "limit": a.limit, "offset": a.offset }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: Value) -> RetentionQuery {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn validation() {
        let base = json!({"target": {"event": "signup"}, "date_from": "2026-11-01T00:00:00Z", "date_to": "2026-11-08T00:00:00Z"});
        assert!(validate(&q(base.clone())).is_ok());
        let mut v = base.clone();
        v["total_intervals"] = json!(0);
        assert!(validate(&q(v)).is_err());
        let mut v = base.clone();
        v["period"] = json!("minute");
        assert!(validate(&q(v)).is_err());
        let mut v = base;
        v["date_to"] = json!("2026-10-01T00:00:00Z");
        assert!(validate(&q(v)).is_err());
    }

    #[test]
    fn returning_defaults_to_target() {
        let r = q(
            json!({"target": {"event": "signup", "properties": [{"key": "plan", "value": "pro"}]},
            "date_from": "2026-11-01T00:00:00Z", "date_to": "2026-11-08T00:00:00Z"}),
        );
        let t = r.as_trends(r.date_to);
        assert_eq!(t.series[1].event.as_deref(), Some("signup"));
        assert_eq!(t.series[1].properties.len(), 1);
    }
}
