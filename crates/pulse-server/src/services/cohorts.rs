use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortGroup {
    pub period_start: NaiveDate,
    pub size: i64,
    pub data: Vec<CohortDataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortDataPoint {
    pub period_offset: i32,
    pub value: f64,
}

pub async fn get_cohorts(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    group_by: &str,
    metric: &str,
) -> AppResult<Vec<CohortGroup>> {
    // Validate inputs
    match group_by {
        "week" | "month" => {}
        other => {
            return Err(AppError::BadRequest(format!(
                "Invalid group_by: {other}. Must be 'week' or 'month'"
            )));
        }
    }
    match metric {
        "pageviews" | "sessions" | "events" | "revenue" => {}
        other => {
            return Err(AppError::BadRequest(format!(
                "Invalid metric: {other}. Must be one of: pageviews, sessions, events, revenue"
            )));
        }
    }

    let start_date = start.date_naive();
    let end_date = end.date_naive();

    // Generate cohort period boundaries
    let periods = generate_periods(start_date, end_date, group_by)?;
    let mut cohort_groups = Vec::with_capacity(periods.len());

    for (i, &cohort_start) in periods.iter().enumerate() {
        let cohort_end = if i + 1 < periods.len() {
            periods[i + 1]
        } else {
            next_period_start(cohort_start, group_by)?
        };

        let cohort_start_ts = cohort_start.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let cohort_end_ts = cohort_end.and_hms_opt(0, 0, 0).unwrap().and_utc();

        // Get cohort size: distinct visitors who had their first session in this period
        let size: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT visitor_id) FROM ( \
                 SELECT visitor_id, MIN(first_at) AS first_visit \
                 FROM sessions WHERE project_id = $1 \
                 GROUP BY visitor_id \
             ) fv WHERE fv.first_visit >= $2 AND fv.first_visit < $3",
        )
        .bind(project_id)
        .bind(cohort_start_ts)
        .bind(cohort_end_ts)
        .fetch_one(db)
        .await?;

        if size.0 == 0 {
            cohort_groups.push(CohortGroup {
                period_start: cohort_start,
                size: 0,
                data: Vec::new(),
            });
            continue;
        }

        // For each subsequent period, track the chosen metric for cohort members
        let mut data_points = Vec::new();
        let mut offset = 0i32;
        let mut measure_start = cohort_start;

        while measure_start < end_date {
            let measure_end = next_period_start(measure_start, group_by)?;
            let ms_ts = measure_start.and_hms_opt(0, 0, 0).unwrap().and_utc();
            let me_ts = measure_end.and_hms_opt(0, 0, 0).unwrap().and_utc();

            let value: f64 = match metric {
                "pageviews" => {
                    let row: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*)::bigint FROM pageviews \
                         WHERE project_id = $1 AND created_at >= $2 AND created_at < $3 \
                         AND visitor_id IN ( \
                             SELECT visitor_id FROM ( \
                                 SELECT visitor_id, MIN(first_at) AS first_visit \
                                 FROM sessions WHERE project_id = $1 \
                                 GROUP BY visitor_id \
                             ) fv WHERE fv.first_visit >= $4 AND fv.first_visit < $5 \
                         )",
                    )
                    .bind(project_id)
                    .bind(ms_ts)
                    .bind(me_ts)
                    .bind(cohort_start_ts)
                    .bind(cohort_end_ts)
                    .fetch_one(db)
                    .await?;
                    row.0 as f64
                }
                "sessions" => {
                    let row: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*)::bigint FROM sessions \
                         WHERE project_id = $1 AND first_at >= $2 AND first_at < $3 \
                         AND visitor_id IN ( \
                             SELECT visitor_id FROM ( \
                                 SELECT visitor_id, MIN(first_at) AS first_visit \
                                 FROM sessions WHERE project_id = $1 \
                                 GROUP BY visitor_id \
                             ) fv WHERE fv.first_visit >= $4 AND fv.first_visit < $5 \
                         )",
                    )
                    .bind(project_id)
                    .bind(ms_ts)
                    .bind(me_ts)
                    .bind(cohort_start_ts)
                    .bind(cohort_end_ts)
                    .fetch_one(db)
                    .await?;
                    row.0 as f64
                }
                "events" => {
                    let row: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*)::bigint FROM events \
                         WHERE project_id = $1 AND created_at >= $2 AND created_at < $3 \
                         AND visitor_id IN ( \
                             SELECT visitor_id FROM ( \
                                 SELECT visitor_id, MIN(first_at) AS first_visit \
                                 FROM sessions WHERE project_id = $1 \
                                 GROUP BY visitor_id \
                             ) fv WHERE fv.first_visit >= $4 AND fv.first_visit < $5 \
                         )",
                    )
                    .bind(project_id)
                    .bind(ms_ts)
                    .bind(me_ts)
                    .bind(cohort_start_ts)
                    .bind(cohort_end_ts)
                    .fetch_one(db)
                    .await?;
                    row.0 as f64
                }
                "revenue" => {
                    let row: (f64,) = sqlx::query_as(
                        "SELECT COALESCE(SUM(revenue_amount), 0)::float8 FROM events \
                         WHERE project_id = $1 AND created_at >= $2 AND created_at < $3 \
                         AND revenue_amount IS NOT NULL \
                         AND visitor_id IN ( \
                             SELECT visitor_id FROM ( \
                                 SELECT visitor_id, MIN(first_at) AS first_visit \
                                 FROM sessions WHERE project_id = $1 \
                                 GROUP BY visitor_id \
                             ) fv WHERE fv.first_visit >= $4 AND fv.first_visit < $5 \
                         )",
                    )
                    .bind(project_id)
                    .bind(ms_ts)
                    .bind(me_ts)
                    .bind(cohort_start_ts)
                    .bind(cohort_end_ts)
                    .fetch_one(db)
                    .await?;
                    row.0
                }
                _ => 0.0,
            };

            data_points.push(CohortDataPoint {
                period_offset: offset,
                value,
            });

            offset += 1;
            measure_start = measure_end;
        }

        cohort_groups.push(CohortGroup {
            period_start: cohort_start,
            size: size.0,
            data: data_points,
        });
    }

    Ok(cohort_groups)
}

fn generate_periods(start: NaiveDate, end: NaiveDate, group_by: &str) -> AppResult<Vec<NaiveDate>> {
    let mut periods = Vec::new();
    let mut current = match group_by {
        "week" => {
            // Align to Monday
            let weekday = start.weekday().num_days_from_monday();
            start - Duration::days(weekday as i64)
        }
        "month" => NaiveDate::from_ymd_opt(start.year(), start.month(), 1).unwrap_or(start),
        _ => start,
    };

    while current <= end {
        periods.push(current);
        current = next_period_start(current, group_by)?;
    }

    Ok(periods)
}

fn next_period_start(date: NaiveDate, group_by: &str) -> AppResult<NaiveDate> {
    match group_by {
        "week" => Ok(date + Duration::days(7)),
        "month" => {
            let (y, m) = if date.month() == 12 {
                (date.year() + 1, 1)
            } else {
                (date.year(), date.month() + 1)
            };
            Ok(NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(date + Duration::days(28)))
        }
        other => Err(AppError::BadRequest(format!("Invalid group_by: {other}"))),
    }
}
