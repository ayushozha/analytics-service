use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct VitalsSummary {
    pub metrics: Vec<MetricSummary>,
}

#[derive(Debug, Serialize)]
pub struct MetricSummary {
    pub name: String,
    pub p50: f64,
    pub p75: f64,
    pub p99: f64,
    pub good: i64,
    pub needs_improvement: i64,
    pub poor: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct PageVitals {
    pub path: String,
    pub lcp_p75: Option<f64>,
    pub fid_p75: Option<f64>,
    pub inp_p75: Option<f64>,
    pub cls_p75: Option<f64>,
    pub fcp_p75: Option<f64>,
    pub ttfb_p75: Option<f64>,
    pub sample_count: i64,
}

#[derive(Debug, Serialize)]
pub struct VitalsTimeseriesPoint {
    pub date: NaiveDate,
    pub p75: f64,
}

const VITAL_METRICS: &[&str] = &["LCP", "FID", "INP", "CLS", "FCP", "TTFB"];

/// Get vitals summary with p50/p75/p99 and rating distribution for each metric.
pub async fn get_vitals_summary(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<VitalsSummary, sqlx::Error> {
    let mut metrics = Vec::new();

    for &metric_name in VITAL_METRICS {
        // Fetch percentiles using percentile_cont
        let percentiles: Option<(Option<f64>, Option<f64>, Option<f64>)> = sqlx::query_as(
            "SELECT \
             percentile_cont(0.5) WITHIN GROUP (ORDER BY metric_value)::double precision, \
             percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value)::double precision, \
             percentile_cont(0.99) WITHIN GROUP (ORDER BY metric_value)::double precision \
             FROM web_vitals WHERE project_id = $1 AND metric_name = $2 \
             AND created_at >= $3 AND created_at <= $4",
        )
        .bind(project_id)
        .bind(metric_name)
        .bind(start)
        .bind(end)
        .fetch_optional(db)
        .await?;

        let (p50, p75, p99) = percentiles
            .map(|(a, b, c)| {
                (a.unwrap_or(0.0), b.unwrap_or(0.0), c.unwrap_or(0.0))
            })
            .unwrap_or((0.0, 0.0, 0.0));

        // Count rating distribution
        let ratings: Vec<(Option<String>, i64)> = sqlx::query_as(
            "SELECT rating, COUNT(*)::bigint FROM web_vitals \
             WHERE project_id = $1 AND metric_name = $2 \
             AND created_at >= $3 AND created_at <= $4 \
             GROUP BY rating",
        )
        .bind(project_id)
        .bind(metric_name)
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await?;

        let mut good = 0i64;
        let mut needs_improvement = 0i64;
        let mut poor = 0i64;

        for (rating, count) in &ratings {
            match rating.as_deref() {
                Some("good") => good = *count,
                Some("needs-improvement") => needs_improvement = *count,
                Some("poor") => poor = *count,
                _ => {}
            }
        }

        let total = good + needs_improvement + poor;

        metrics.push(MetricSummary {
            name: metric_name.to_string(),
            p50,
            p75,
            p99,
            good,
            needs_improvement,
            poor,
            total,
        });
    }

    Ok(VitalsSummary { metrics })
}

/// Get aggregate vitals per page path.
pub async fn get_vitals_by_page(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<PageVitals>, sqlx::Error> {
    // Get all distinct paths with their vital metric p75 values using conditional aggregation
    let rows: Vec<(String, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, i64)> = sqlx::query_as(
        "SELECT path, \
         percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value) FILTER (WHERE metric_name = 'LCP')::double precision, \
         percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value) FILTER (WHERE metric_name = 'FID')::double precision, \
         percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value) FILTER (WHERE metric_name = 'INP')::double precision, \
         percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value) FILTER (WHERE metric_name = 'CLS')::double precision, \
         percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value) FILTER (WHERE metric_name = 'FCP')::double precision, \
         percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value) FILTER (WHERE metric_name = 'TTFB')::double precision, \
         COUNT(*)::bigint \
         FROM web_vitals WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
         AND path IS NOT NULL \
         GROUP BY path ORDER BY 8 DESC LIMIT $4",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| PageVitals {
            path: r.0,
            lcp_p75: r.1,
            fid_p75: r.2,
            inp_p75: r.3,
            cls_p75: r.4,
            fcp_p75: r.5,
            ttfb_p75: r.6,
            sample_count: r.7,
        })
        .collect();

    Ok(results)
}

/// Get daily p75 timeseries for a specific metric.
pub async fn get_vitals_timeseries(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    metric_name: &str,
) -> Result<Vec<VitalsTimeseriesPoint>, sqlx::Error> {
    let rows: Vec<(NaiveDate, Option<f64>)> = sqlx::query_as(
        "SELECT created_at::date as day, \
         percentile_cont(0.75) WITHIN GROUP (ORDER BY metric_value)::double precision \
         FROM web_vitals WHERE project_id = $1 AND metric_name = $2 \
         AND created_at >= $3 AND created_at <= $4 \
         GROUP BY day ORDER BY day",
    )
    .bind(project_id)
    .bind(metric_name)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|(date, p75)| VitalsTimeseriesPoint {
            date,
            p75: p75.unwrap_or(0.0),
        })
        .collect();

    Ok(results)
}
