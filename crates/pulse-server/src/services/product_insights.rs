use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct StickinessPeriod {
    pub date: NaiveDate,
    pub active_visitors: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StickinessReport {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub dau: f64,
    pub wau: f64,
    pub mau: f64,
    pub dau_wau: f64,
    pub wau_mau: f64,
    pub dau_mau: f64,
    pub periods: Vec<StickinessPeriod>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecycleReport {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub previous_start_at: DateTime<Utc>,
    pub previous_end_at: DateTime<Utc>,
    pub active_visitors: i64,
    pub new_visitors: i64,
    pub returning_visitors: i64,
    pub resurrected_visitors: i64,
    pub dormant_visitors: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActivationRequest {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(default)]
    pub event_names: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivationReport {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub cohort_visitors: i64,
    pub activated_visitors: i64,
    pub activation_rate: f64,
    pub required_events: Vec<String>,
    pub required_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImpactAnalysisRequest {
    pub metric: String,
    pub split_at: DateTime<Utc>,
    pub window_days: Option<i64>,
    pub event_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactAnalysisReport {
    pub metric: String,
    pub event_name: Option<String>,
    pub split_at: DateTime<Utc>,
    pub before_start_at: DateTime<Utc>,
    pub before_end_at: DateTime<Utc>,
    pub after_start_at: DateTime<Utc>,
    pub after_end_at: DateTime<Utc>,
    pub before_value: i64,
    pub after_value: i64,
    pub absolute_change: i64,
    pub percent_change: f64,
    pub direction: String,
    pub summary: String,
}

pub async fn get_stickiness(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> AppResult<StickinessReport> {
    validate_range(start_at, end_at)?;

    let periods: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"
        WITH activity AS (
            SELECT visitor_id, created_at FROM pageviews
            WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3
            UNION ALL
            SELECT visitor_id, created_at FROM events
            WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3
        )
        SELECT date_trunc('day', created_at)::date AS date,
               COUNT(DISTINCT visitor_id)::bigint AS active_visitors
        FROM activity
        GROUP BY 1
        ORDER BY 1
        "#,
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_all(db)
    .await?;

    let dau = average_active(db, project_id, start_at, end_at, "day").await?;
    let wau = average_active(db, project_id, start_at, end_at, "week").await?;
    let mau = average_active(db, project_id, start_at, end_at, "month").await?;

    Ok(StickinessReport {
        start_at,
        end_at,
        dau,
        wau,
        mau,
        dau_wau: percent_float(dau, wau),
        wau_mau: percent_float(wau, mau),
        dau_mau: percent_float(dau, mau),
        periods: periods
            .into_iter()
            .map(|(date, active_visitors)| StickinessPeriod {
                date,
                active_visitors,
            })
            .collect(),
    })
}

pub async fn get_lifecycle(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> AppResult<LifecycleReport> {
    validate_range(start_at, end_at)?;
    let previous_start_at = start_at - (end_at - start_at);
    let previous_end_at = start_at;

    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        WITH activity AS (
            SELECT visitor_id, created_at FROM pageviews
            WHERE project_id = $1 AND created_at <= $3
            UNION ALL
            SELECT visitor_id, created_at FROM events
            WHERE project_id = $1 AND created_at <= $3
        ),
        current_visitors AS (
            SELECT DISTINCT visitor_id FROM activity
            WHERE created_at >= $2 AND created_at <= $3
        ),
        previous_visitors AS (
            SELECT DISTINCT visitor_id FROM activity
            WHERE created_at >= $4 AND created_at < $5
        ),
        first_seen AS (
            SELECT visitor_id, MIN(created_at) AS first_seen_at
            FROM activity
            GROUP BY visitor_id
        )
        SELECT
            (SELECT COUNT(*)::bigint FROM current_visitors),
            (
                SELECT COUNT(*)::bigint
                FROM current_visitors cv
                JOIN first_seen fs USING (visitor_id)
                WHERE fs.first_seen_at >= $2
            ),
            (
                SELECT COUNT(*)::bigint
                FROM current_visitors cv
                JOIN previous_visitors pv USING (visitor_id)
            ),
            (
                SELECT COUNT(*)::bigint
                FROM current_visitors cv
                LEFT JOIN previous_visitors pv USING (visitor_id)
                JOIN first_seen fs USING (visitor_id)
                WHERE pv.visitor_id IS NULL AND fs.first_seen_at < $2
            ),
            (
                SELECT COUNT(*)::bigint
                FROM previous_visitors pv
                LEFT JOIN current_visitors cv USING (visitor_id)
                WHERE cv.visitor_id IS NULL
            )
        "#,
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .bind(previous_start_at)
    .bind(previous_end_at)
    .fetch_one(db)
    .await?;

    Ok(LifecycleReport {
        start_at,
        end_at,
        previous_start_at,
        previous_end_at,
        active_visitors: counts.0,
        new_visitors: counts.1,
        returning_visitors: counts.2,
        resurrected_visitors: counts.3,
        dormant_visitors: counts.4,
    })
}

pub async fn get_activation(
    db: &PgPool,
    project_id: Uuid,
    request: ActivationRequest,
) -> AppResult<ActivationReport> {
    validate_range(request.start_at, request.end_at)?;
    let event_names = normalize_criteria(request.event_names, "event_names")?;
    let paths = normalize_criteria(request.paths, "paths")?;
    if event_names.is_empty() && paths.is_empty() {
        return Err(AppError::BadRequest(
            "At least one activation event or path is required".to_string(),
        ));
    }

    let required_events = event_names.len() as i64;
    let required_paths = paths.len() as i64;
    let counts: (i64, i64) = sqlx::query_as(
        r#"
        WITH cohort AS (
            SELECT DISTINCT visitor_id FROM (
                SELECT visitor_id FROM pageviews
                WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3
                UNION
                SELECT visitor_id FROM events
                WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3
            ) activity
        ),
        event_hits AS (
            SELECT visitor_id, COUNT(DISTINCT event_name)::bigint AS matched
            FROM events
            WHERE project_id = $1
              AND created_at >= $2
              AND created_at <= $3
              AND event_name = ANY($4)
            GROUP BY visitor_id
        ),
        path_hits AS (
            SELECT visitor_id, COUNT(DISTINCT path)::bigint AS matched
            FROM pageviews
            WHERE project_id = $1
              AND created_at >= $2
              AND created_at <= $3
              AND path = ANY($5)
            GROUP BY visitor_id
        )
        SELECT
            COUNT(*)::bigint AS cohort_visitors,
            COUNT(*) FILTER (
                WHERE COALESCE(event_hits.matched, 0) >= $6
                  AND COALESCE(path_hits.matched, 0) >= $7
            )::bigint AS activated_visitors
        FROM cohort
        LEFT JOIN event_hits USING (visitor_id)
        LEFT JOIN path_hits USING (visitor_id)
        "#,
    )
    .bind(project_id)
    .bind(request.start_at)
    .bind(request.end_at)
    .bind(&event_names)
    .bind(&paths)
    .bind(required_events)
    .bind(required_paths)
    .fetch_one(db)
    .await?;

    Ok(ActivationReport {
        start_at: request.start_at,
        end_at: request.end_at,
        cohort_visitors: counts.0,
        activated_visitors: counts.1,
        activation_rate: percent_i64(counts.1, counts.0),
        required_events: event_names,
        required_paths: paths,
    })
}

pub async fn get_impact_analysis(
    db: &PgPool,
    project_id: Uuid,
    request: ImpactAnalysisRequest,
) -> AppResult<ImpactAnalysisReport> {
    let metric = validate_metric(&request.metric)?;
    let window_days = request.window_days.unwrap_or(7).clamp(1, 90);
    let window = Duration::days(window_days);
    let before_start_at = request.split_at - window;
    let before_end_at = request.split_at;
    let after_start_at = request.split_at;
    let after_end_at = request.split_at + window;
    let event_name = request
        .event_name
        .as_ref()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());

    if metric != "events" && event_name.is_some() {
        return Err(AppError::BadRequest(
            "event_name can only be used with metric=events".to_string(),
        ));
    }

    let before_value = metric_value(
        db,
        project_id,
        metric,
        before_start_at,
        before_end_at,
        event_name.as_deref(),
    )
    .await?;
    let after_value = metric_value(
        db,
        project_id,
        metric,
        after_start_at,
        after_end_at,
        event_name.as_deref(),
    )
    .await?;
    let absolute_change = after_value - before_value;
    let percent_change = if before_value > 0 {
        (absolute_change as f64) / (before_value as f64) * 100.0
    } else if after_value > 0 {
        100.0
    } else {
        0.0
    };
    let direction = if absolute_change > 0 {
        "up"
    } else if absolute_change < 0 {
        "down"
    } else {
        "flat"
    }
    .to_string();
    let summary = format!(
        "{metric} moved from {before_value} to {after_value} across {window_days}-day windows."
    );

    Ok(ImpactAnalysisReport {
        metric: metric.to_string(),
        event_name,
        split_at: request.split_at,
        before_start_at,
        before_end_at,
        after_start_at,
        after_end_at,
        before_value,
        after_value,
        absolute_change,
        percent_change,
        direction,
        summary,
    })
}

async fn average_active(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    bucket: &str,
) -> AppResult<f64> {
    let bucket = match bucket {
        "day" | "week" | "month" => bucket,
        _ => {
            return Err(AppError::BadRequest(
                "Unsupported stickiness bucket".to_string(),
            ))
        }
    };
    let sql = format!(
        r#"
        WITH activity AS (
            SELECT visitor_id, created_at FROM pageviews
            WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3
            UNION ALL
            SELECT visitor_id, created_at FROM events
            WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3
        ),
        bucketed AS (
            SELECT date_trunc('{bucket}', created_at) AS period,
                   COUNT(DISTINCT visitor_id)::bigint AS active_visitors
            FROM activity
            GROUP BY 1
        )
        SELECT COALESCE(AVG(active_visitors), 0)::double precision FROM bucketed
        "#
    );
    let value = sqlx::query_scalar(&sql)
        .bind(project_id)
        .bind(start_at)
        .bind(end_at)
        .fetch_one(db)
        .await?;
    Ok(value)
}

async fn metric_value(
    db: &PgPool,
    project_id: Uuid,
    metric: &str,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    event_name: Option<&str>,
) -> AppResult<i64> {
    let value: i64 = match metric {
        "pageviews" => {
            sqlx::query_scalar(
                "SELECT COUNT(*)::bigint FROM pageviews \
                 WHERE project_id = $1 AND created_at >= $2 AND created_at < $3",
            )
            .bind(project_id)
            .bind(start_at)
            .bind(end_at)
            .fetch_one(db)
            .await?
        }
        "visitors" => {
            sqlx::query_scalar(
                r#"
                WITH activity AS (
                    SELECT visitor_id FROM pageviews
                    WHERE project_id = $1 AND created_at >= $2 AND created_at < $3
                    UNION
                    SELECT visitor_id FROM events
                    WHERE project_id = $1 AND created_at >= $2 AND created_at < $3
                )
                SELECT COUNT(*)::bigint FROM activity
                "#,
            )
            .bind(project_id)
            .bind(start_at)
            .bind(end_at)
            .fetch_one(db)
            .await?
        }
        "sessions" => {
            sqlx::query_scalar(
                "SELECT COUNT(*)::bigint FROM sessions \
                 WHERE project_id = $1 AND first_at >= $2 AND first_at < $3",
            )
            .bind(project_id)
            .bind(start_at)
            .bind(end_at)
            .fetch_one(db)
            .await?
        }
        "events" => {
            if let Some(event_name) = event_name {
                sqlx::query_scalar(
                    "SELECT COUNT(*)::bigint FROM events \
                     WHERE project_id = $1 AND created_at >= $2 AND created_at < $3 \
                       AND event_name = $4",
                )
                .bind(project_id)
                .bind(start_at)
                .bind(end_at)
                .bind(event_name)
                .fetch_one(db)
                .await?
            } else {
                sqlx::query_scalar(
                    "SELECT COUNT(*)::bigint FROM events \
                     WHERE project_id = $1 AND created_at >= $2 AND created_at < $3",
                )
                .bind(project_id)
                .bind(start_at)
                .bind(end_at)
                .fetch_one(db)
                .await?
            }
        }
        "errors" => {
            sqlx::query_scalar(
                "SELECT COUNT(*)::bigint FROM js_errors \
                 WHERE project_id = $1 AND created_at >= $2 AND created_at < $3",
            )
            .bind(project_id)
            .bind(start_at)
            .bind(end_at)
            .fetch_one(db)
            .await?
        }
        _ => unreachable!("validated metric"),
    };
    Ok(value)
}

fn validate_range(start_at: DateTime<Utc>, end_at: DateTime<Utc>) -> AppResult<()> {
    if start_at >= end_at {
        return Err(AppError::BadRequest(
            "start_at must be before end_at".to_string(),
        ));
    }
    Ok(())
}

fn validate_metric(metric: &str) -> AppResult<&'static str> {
    match metric.trim() {
        "pageviews" => Ok("pageviews"),
        "visitors" => Ok("visitors"),
        "sessions" => Ok("sessions"),
        "events" => Ok("events"),
        "errors" => Ok("errors"),
        other => Err(AppError::BadRequest(format!("Unsupported metric: {other}"))),
    }
}

fn normalize_criteria(values: Vec<String>, field: &str) -> AppResult<Vec<String>> {
    if values.len() > 25 {
        return Err(AppError::BadRequest(format!(
            "{field} supports at most 25 entries"
        )));
    }
    let normalized: Vec<String> = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    Ok(normalized)
}

fn percent_i64(numerator: i64, denominator: i64) -> f64 {
    if denominator > 0 {
        (numerator as f64) / (denominator as f64) * 100.0
    } else {
        0.0
    }
}

fn percent_float(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        numerator / denominator * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_criteria, validate_metric, validate_range};
    use chrono::{Duration, Utc};

    #[test]
    fn validates_supported_impact_metrics() {
        assert!(validate_metric("pageviews").is_ok());
        assert!(validate_metric("events").is_ok());
        assert!(validate_metric("raw_sql").is_err());
    }

    #[test]
    fn normalizes_activation_criteria() {
        let criteria = normalize_criteria(
            vec![
                " signup ".to_string(),
                "".to_string(),
                "purchase".to_string(),
            ],
            "event_names",
        )
        .unwrap();
        assert_eq!(criteria, vec!["signup", "purchase"]);
    }

    #[test]
    fn rejects_invalid_date_ranges() {
        let end = Utc::now();
        let start = end + Duration::days(1);
        assert!(validate_range(start, end).is_err());
    }
}
