use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionData {
    pub cohorts: Vec<RetentionCohort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionCohort {
    pub date: NaiveDate,
    pub total_visitors: i64,
    pub returning: Vec<RetentionPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPoint {
    pub period: i32,
    pub visitors: i64,
    pub percentage: f64,
}

const RETENTION_PERIODS: &[i32] = &[1, 7, 14, 30, 60, 90];

pub async fn get_retention(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    period: &str,
) -> AppResult<RetentionData> {
    let start_date = start.date_naive();
    let end_date = end.date_naive();

    let cohort_dates = generate_cohort_dates(start_date, end_date, period)?;
    let mut cohorts = Vec::with_capacity(cohort_dates.len());

    for cohort_date in &cohort_dates {
        let (cohort_start, cohort_end) = cohort_window(*cohort_date, period)?;

        // Find all distinct visitors whose first session falls within this cohort window
        let total_visitors: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT s.visitor_id) FROM sessions s \
             WHERE s.project_id = $1 \
             AND s.first_at >= $2 AND s.first_at < $3 \
             AND s.visitor_id IN ( \
                 SELECT visitor_id FROM ( \
                     SELECT visitor_id, MIN(first_at) AS first_visit \
                     FROM sessions WHERE project_id = $1 \
                     GROUP BY visitor_id \
                 ) fv WHERE fv.first_visit >= $2 AND fv.first_visit < $3 \
             )",
        )
        .bind(project_id)
        .bind(cohort_start.and_hms_opt(0, 0, 0).unwrap().and_utc())
        .bind(cohort_end.and_hms_opt(0, 0, 0).unwrap().and_utc())
        .fetch_one(db)
        .await?;

        let mut returning = Vec::with_capacity(RETENTION_PERIODS.len());

        for &ret_period in RETENTION_PERIODS {
            let ret_start = cohort_start + Duration::days(ret_period as i64);
            let ret_end = ret_start + Duration::days(1);

            if ret_start > Utc::now().date_naive() {
                // Future date, no data
                returning.push(RetentionPoint {
                    period: ret_period,
                    visitors: 0,
                    percentage: 0.0,
                });
                continue;
            }

            let ret_visitors: (i64,) = sqlx::query_as(
                "SELECT COUNT(DISTINCT s2.visitor_id) FROM sessions s2 \
                 WHERE s2.project_id = $1 \
                 AND s2.first_at >= $4 AND s2.first_at < $5 \
                 AND s2.visitor_id IN ( \
                     SELECT visitor_id FROM ( \
                         SELECT visitor_id, MIN(first_at) AS first_visit \
                         FROM sessions WHERE project_id = $1 \
                         GROUP BY visitor_id \
                     ) fv WHERE fv.first_visit >= $2 AND fv.first_visit < $3 \
                 )",
            )
            .bind(project_id)
            .bind(cohort_start.and_hms_opt(0, 0, 0).unwrap().and_utc())
            .bind(cohort_end.and_hms_opt(0, 0, 0).unwrap().and_utc())
            .bind(ret_start.and_hms_opt(0, 0, 0).unwrap().and_utc())
            .bind(ret_end.and_hms_opt(0, 0, 0).unwrap().and_utc())
            .fetch_one(db)
            .await?;

            let percentage = if total_visitors.0 > 0 {
                (ret_visitors.0 as f64 / total_visitors.0 as f64) * 100.0
            } else {
                0.0
            };

            returning.push(RetentionPoint {
                period: ret_period,
                visitors: ret_visitors.0,
                percentage,
            });
        }

        cohorts.push(RetentionCohort {
            date: *cohort_date,
            total_visitors: total_visitors.0,
            returning,
        });
    }

    Ok(RetentionData { cohorts })
}

fn generate_cohort_dates(
    start: NaiveDate,
    end: NaiveDate,
    period: &str,
) -> AppResult<Vec<NaiveDate>> {
    let mut dates = Vec::new();
    let mut current = start;

    match period {
        "daily" => {
            while current <= end {
                dates.push(current);
                current += Duration::days(1);
            }
        }
        "weekly" => {
            // Align to Monday
            let weekday = current.weekday().num_days_from_monday();
            current -= Duration::days(weekday as i64);
            while current <= end {
                dates.push(current);
                current += Duration::days(7);
            }
        }
        "monthly" => {
            // Align to first of month
            current =
                NaiveDate::from_ymd_opt(current.year(), current.month(), 1).unwrap_or(current);
            while current <= end {
                dates.push(current);
                // Move to first of next month
                let (y, m) = if current.month() == 12 {
                    (current.year() + 1, 1)
                } else {
                    (current.year(), current.month() + 1)
                };
                current = NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(current + Duration::days(28));
            }
        }
        other => {
            return Err(AppError::BadRequest(format!(
                "Invalid period: {other}. Must be 'daily', 'weekly', or 'monthly'"
            )));
        }
    }

    Ok(dates)
}

fn cohort_window(date: NaiveDate, period: &str) -> AppResult<(NaiveDate, NaiveDate)> {
    match period {
        "daily" => Ok((date, date + Duration::days(1))),
        "weekly" => Ok((date, date + Duration::days(7))),
        "monthly" => {
            let (y, m) = if date.month() == 12 {
                (date.year() + 1, 1)
            } else {
                (date.year(), date.month() + 1)
            };
            let next_month = NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(date + Duration::days(28));
            Ok((date, next_month))
        }
        other => Err(AppError::BadRequest(format!("Invalid period: {other}"))),
    }
}
