//! Stickiness (PostHog `StickinessQuery`): for each series, how many visitors
//! did the event on exactly N intervals of the range (or at least N with
//! `cumulative`), with filters and compare-to-previous.
//!
//! Lifecycle (PostHog `LifecycleQuery`): per interval, how many visitors were
//! new, returning, resurrecting, or went dormant for one series.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::trends::{bucket_labels, series_label, Ctx, TrendsQuery};
use super::{bad_request, load_profiles, map_query_error};
use crate::error::AppResult;

/// Constraints for stickiness queries (also enforced when saving).
pub(crate) fn validate_common(q: &TrendsQuery) -> AppResult<()> {
    super::trends::validate(q)?;
    if q.breakdown.is_some() || q.formula.is_some() {
        return Err(bad_request(
            "breakdowns and formulas are not supported for this insight",
        ));
    }
    if q.series.iter().any(|s| s.math != Default::default()) {
        return Err(bad_request("this insight counts visitors; omit math"));
    }
    Ok(())
}

/// visitor -> number of distinct active intervals, for one series.
async fn active_intervals(db: &PgPool, ctx: &Ctx<'_>) -> AppResult<Vec<(String, i64)>> {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT ev.visitor_id, COUNT(DISTINCT ");
    ctx.push_bucket(&mut qb);
    qb.push(")");
    ctx.push_from_where(&mut qb)?;
    qb.push(" GROUP BY 1");
    qb.build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)
}

async fn stickiness_period(
    db: &PgPool,
    project_id: Uuid,
    q: &TrendsQuery,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    label: Option<&'static str>,
) -> AppResult<Vec<Value>> {
    let n = bucket_labels(db, &q.interval, &q.timezone, from, to)
        .await?
        .len();
    let mut out = Vec::new();
    for (i, series) in q.series.iter().enumerate() {
        let ctx = Ctx::new(project_id, q, series, from, to);
        let mut hist = vec![0.0_f64; n];
        for (_, k) in active_intervals(db, &ctx).await? {
            let k = (k as usize).clamp(1, n);
            hist[k - 1] += 1.0;
        }
        if q.cumulative {
            for j in (0..n.saturating_sub(1)).rev() {
                hist[j] += hist[j + 1];
            }
        }
        out.push(json!({
            "label": series_label(q, i),
            "series_index": i,
            "event": series.event,
            "compare_label": label,
            "days": (1..=n).collect::<Vec<_>>(),
            "labels": (1..=n).map(|k| format!("{k} {}{}", q.interval.as_sql(), if k == 1 { "" } else { "s" })).collect::<Vec<_>>(),
            "data": hist,
            "count": hist.iter().sum::<f64>(),
        }));
    }
    Ok(out)
}

pub async fn run_stickiness(db: &PgPool, project_id: Uuid, q: &TrendsQuery) -> AppResult<Value> {
    validate_common(q)?;
    let mut results = stickiness_period(
        db,
        project_id,
        q,
        q.date_from,
        q.date_to,
        q.compare.then_some("current"),
    )
    .await?;
    if q.compare {
        let len = q.date_to - q.date_from;
        results.extend(
            stickiness_period(
                db,
                project_id,
                q,
                q.date_from - len,
                q.date_from,
                Some("previous"),
            )
            .await?,
        );
    }
    Ok(json!({ "results": results, "interval": q.interval, "cumulative": q.cumulative }))
}

pub const LIFECYCLE_STATUSES: [&str; 4] = ["new", "returning", "resurrecting", "dormant"];

/// Per visitor: active bucket labels (including the bucket before the range)
/// and the bucket of their first-ever matching event.
struct LifecycleData {
    labels: Vec<String>,
    /// Label of the bucket immediately before the first one.
    prev_label: String,
    active: HashMap<String, HashSet<String>>,
    first: HashMap<String, String>,
}

/// Constraints for lifecycle queries (also enforced when saving).
pub(crate) fn validate_lifecycle(q: &TrendsQuery) -> AppResult<()> {
    validate_common(q)?;
    if q.series.len() != 1 {
        return Err(bad_request("lifecycle takes exactly one series"));
    }
    Ok(())
}

async fn lifecycle_data(
    db: &PgPool,
    project_id: Uuid,
    q: &TrendsQuery,
) -> AppResult<LifecycleData> {
    validate_lifecycle(q)?;
    let labels = bucket_labels(db, &q.interval, &q.timezone, q.date_from, q.date_to).await?;
    // The bucket before the range is loaded separately so the first bucket can
    // be classified (returning/resurrecting/dormant) without counting events
    // that fall in the first bucket but before `date_from`.
    let (prev_start, first_start, prev_label): (DateTime<Utc>, DateTime<Utc>, String) =
        sqlx::query_as(
            "SELECT (date_trunc($1, $2 AT TIME ZONE $3) - ('1 ' || $1)::interval) AT TIME ZONE $3, \
             date_trunc($1, $2 AT TIME ZONE $3) AT TIME ZONE $3, \
             to_char(date_trunc($1, $2 AT TIME ZONE $3) - ('1 ' || $1)::interval, 'YYYY-MM-DD\"T\"HH24:MI:SS')",
        )
        .bind(q.interval.as_sql())
        .bind(q.date_from)
        .bind(&q.timezone)
        .fetch_one(db)
        .await
        .map_err(map_query_error)?;

    let series = &q.series[0];
    let mut active: HashMap<String, HashSet<String>> = HashMap::new();
    for (from, to) in [(prev_start, first_start), (q.date_from, q.date_to)] {
        let ctx = Ctx::new(project_id, q, series, from, to);
        let mut qb = QueryBuilder::<Postgres>::new("SELECT DISTINCT ev.visitor_id, ");
        ctx.push_bucket(&mut qb);
        ctx.push_from_where(&mut qb)?;
        let rows: Vec<(String, String)> = qb
            .build_query_as()
            .fetch_all(db)
            .await
            .map_err(map_query_error)?;
        for (v, b) in rows {
            active.entry(v).or_default().insert(b);
        }
    }

    // First-ever matching event per active visitor (all history up to date_to).
    let ids: Vec<String> = active.keys().cloned().collect();
    let ctx = Ctx::new(
        project_id,
        q,
        series,
        DateTime::<Utc>::UNIX_EPOCH,
        q.date_to,
    );
    let mut qb = QueryBuilder::<Postgres>::new("SELECT ev.visitor_id, MIN(");
    ctx.push_bucket(&mut qb);
    qb.push(")");
    ctx.push_from_where(&mut qb)?;
    qb.push(" AND ev.visitor_id = ANY(");
    qb.push_bind(ids);
    qb.push(") GROUP BY 1");
    let first: Vec<(String, String)> = qb
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(map_query_error)?;
    Ok(LifecycleData {
        labels,
        prev_label,
        active,
        first: first.into_iter().collect(),
    })
}

/// Status of a visitor at bucket `i`, or None if not counted there.
fn status(d: &LifecycleData, visitor: &str, i: usize) -> Option<&'static str> {
    let act = d.active.get(visitor)?;
    let cur = &d.labels[i];
    let prev = if i == 0 {
        &d.prev_label
    } else {
        &d.labels[i - 1]
    };
    let (now, before) = (act.contains(cur), act.contains(prev));
    match (now, before) {
        (true, _) if d.first.get(visitor) == Some(cur) => Some("new"),
        (true, true) => Some("returning"),
        (true, false) => Some("resurrecting"),
        (false, true) => Some("dormant"),
        (false, false) => None,
    }
}

pub async fn run_lifecycle(db: &PgPool, project_id: Uuid, q: &TrendsQuery) -> AppResult<Value> {
    let d = lifecycle_data(db, project_id, q).await?;
    let n = d.labels.len();
    let mut counts: HashMap<&str, Vec<f64>> = LIFECYCLE_STATUSES
        .iter()
        .map(|s| (*s, vec![0.0; n]))
        .collect();
    for v in d.active.keys() {
        for i in 0..n {
            if let Some(s) = status(&d, v, i) {
                counts.get_mut(s).expect("known status")[i] += 1.0;
            }
        }
    }
    let results: Vec<Value> = LIFECYCLE_STATUSES
        .iter()
        .map(|s| {
            let sign = if *s == "dormant" { -1.0 } else { 1.0 };
            let data: Vec<f64> = counts[s].iter().map(|c| c * sign).collect();
            json!({
                "status": s,
                "label": format!("{} - {s}", series_label(q, 0)),
                "days": d.labels,
                "count": data.iter().sum::<f64>(),
                "data": data,
            })
        })
        .collect();
    Ok(json!({ "results": results, "interval": q.interval }))
}

/// Visitors with a given lifecycle status in one bucket.
pub async fn run_lifecycle_actors(
    db: &PgPool,
    project_id: Uuid,
    q: &TrendsQuery,
    day: &str,
    wanted: &str,
    limit: usize,
    offset: usize,
) -> AppResult<Value> {
    if !LIFECYCLE_STATUSES.contains(&wanted) {
        return Err(bad_request(
            "status must be new, returning, resurrecting or dormant",
        ));
    }
    if !(1..=1000).contains(&limit) {
        return Err(bad_request("limit must be 1-1000"));
    }
    let d = lifecycle_data(db, project_id, q).await?;
    let i = d
        .labels
        .iter()
        .position(|l| l == day)
        .ok_or_else(|| bad_request("day is not a bucket of this query"))?;
    let mut ids: Vec<String> = d
        .active
        .keys()
        .filter(|v| status(&d, v, i) == Some(wanted))
        .cloned()
        .collect();
    ids.sort();
    let total = ids.len();
    let page: Vec<String> = ids.into_iter().skip(offset).take(limit).collect();
    let profiles = load_profiles(db, project_id, &page).await?;
    let data: Vec<Value> = page
        .iter()
        .map(|v| {
            let (user_id, traits) = profiles.get(v).cloned().unwrap_or((None, json!({})));
            json!({ "visitor_id": v, "user_id": user_id, "traits": traits })
        })
        .collect();
    Ok(json!({ "data": data, "total": total, "limit": limit, "offset": offset }))
}

/// Visitors active on exactly `intervals` intervals (stickiness bar).
pub async fn run_stickiness_actors(
    db: &PgPool,
    project_id: Uuid,
    q: &TrendsQuery,
    series: usize,
    intervals: i64,
    limit: usize,
    offset: usize,
) -> AppResult<Value> {
    validate_common(q)?;
    if !(1..=1000).contains(&limit) {
        return Err(bad_request("limit must be 1-1000"));
    }
    let s = q
        .series
        .get(series)
        .ok_or_else(|| bad_request("series index out of range"))?;
    let ctx = Ctx::new(project_id, q, s, q.date_from, q.date_to);
    let mut ids: Vec<String> = active_intervals(db, &ctx)
        .await?
        .into_iter()
        // Cumulative bars count visitors active on at least N intervals.
        .filter(|(_, k)| {
            if q.cumulative {
                *k >= intervals
            } else {
                *k == intervals
            }
        })
        .map(|(v, _)| v)
        .collect();
    ids.sort();
    let total = ids.len();
    let page: Vec<String> = ids.into_iter().skip(offset).take(limit).collect();
    let profiles = load_profiles(db, project_id, &page).await?;
    let data: Vec<Value> = page
        .iter()
        .map(|v| {
            let (user_id, traits) = profiles.get(v).cloned().unwrap_or((None, json!({})));
            json!({ "visitor_id": v, "user_id": user_id, "traits": traits })
        })
        .collect();
    Ok(json!({ "data": data, "total": total, "limit": limit, "offset": offset }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(active: &[(&str, &[&str])], first: &[(&str, &str)]) -> LifecycleData {
        LifecycleData {
            labels: vec!["d1".into(), "d2".into(), "d3".into()],
            prev_label: "d0".into(),
            active: active
                .iter()
                .map(|(v, bs)| (v.to_string(), bs.iter().map(|b| b.to_string()).collect()))
                .collect(),
            first: first
                .iter()
                .map(|(v, b)| (v.to_string(), b.to_string()))
                .collect(),
        }
    }

    #[test]
    fn lifecycle_statuses() {
        let d = data(
            &[
                ("a", &["d1", "d2"]),
                ("b", &["d0", "d1"]),
                ("c", &["d0", "d2"]),
            ],
            &[("a", "d1"), ("b", "d0"), ("c", "d0")],
        );
        assert_eq!(status(&d, "a", 0), Some("new"));
        assert_eq!(status(&d, "a", 1), Some("returning"));
        assert_eq!(status(&d, "a", 2), Some("dormant"));
        assert_eq!(status(&d, "b", 0), Some("returning"));
        assert_eq!(status(&d, "b", 1), Some("dormant"));
        assert_eq!(status(&d, "b", 2), None);
        assert_eq!(status(&d, "c", 0), Some("dormant"));
        assert_eq!(status(&d, "c", 1), Some("resurrecting"));
    }
}
