use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignStat {
    pub utm_source: String,
    pub utm_medium: String,
    pub utm_campaign: String,
    pub visitors: i64,
    pub sessions: i64,
    pub pageviews: i64,
    pub bounce_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesPoint {
    pub date: NaiveDate,
    pub visitors: i64,
    pub sessions: i64,
    pub pageviews: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStat {
    pub utm_source: String,
    pub visitors: i64,
    pub sessions: i64,
    pub pageviews: i64,
    pub bounce_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediumStat {
    pub utm_medium: String,
    pub visitors: i64,
    pub sessions: i64,
    pub pageviews: i64,
    pub bounce_rate: f64,
}

/// Aggregate campaign stats using the hybrid rollup + raw approach.
/// Uses daily_campaigns for historical data and raw pageviews for today.
pub async fn get_campaign_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<Vec<CampaignStat>> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, String, String, i64, i64, i64, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT
                utm_source, utm_medium, utm_campaign,
                SUM(visitors)::bigint, SUM(sessions)::bigint,
                SUM(pageviews)::bigint, SUM(bounces)::bigint
            FROM (
                SELECT utm_source, utm_medium, utm_campaign, visitors, sessions, pageviews, bounces
                FROM daily_campaigns
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT
                    COALESCE(utm_source, '')::varchar AS utm_source,
                    COALESCE(utm_medium, '')::varchar AS utm_medium,
                    COALESCE(utm_campaign, '')::varchar AS utm_campaign,
                    COUNT(DISTINCT visitor_id)::int AS visitors,
                    COUNT(DISTINCT session_id)::int AS sessions,
                    COUNT(*)::int AS pageviews,
                    0::int AS bounces
                FROM pageviews
                WHERE project_id = $1 AND created_at >= $6::date AND created_at <= $3
                AND (utm_source IS NOT NULL OR utm_medium IS NOT NULL OR utm_campaign IS NOT NULL)
                GROUP BY COALESCE(utm_source, ''), COALESCE(utm_medium, ''), COALESCE(utm_campaign, '')
            ) combined
            GROUP BY utm_source, utm_medium, utm_campaign
            ORDER BY 6 DESC
            LIMIT 100"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(start)      // $4 (unused in this query variant but kept for consistency)
        .bind(end)        // $5
        .bind(today)      // $6
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT
                utm_source, utm_medium, utm_campaign,
                COALESCE(SUM(visitors), 0)::bigint,
                COALESCE(SUM(sessions), 0)::bigint,
                COALESCE(SUM(pageviews), 0)::bigint,
                COALESCE(SUM(bounces), 0)::bigint
            FROM daily_campaigns
            WHERE project_id = $1 AND date >= $2::date AND date <= $3::date
            GROUP BY utm_source, utm_medium, utm_campaign
            ORDER BY 6 DESC
            LIMIT 100"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .fetch_all(db)
        .await?
    };

    let stats = rows
        .into_iter()
        .map(|(source, medium, campaign, visitors, sessions, pageviews, bounces)| {
            let bounce_rate = if sessions > 0 {
                (bounces as f64 / sessions as f64) * 100.0
            } else {
                0.0
            };
            CampaignStat {
                utm_source: source,
                utm_medium: medium,
                utm_campaign: campaign,
                visitors,
                sessions,
                pageviews,
                bounce_rate,
            }
        })
        .collect();

    Ok(stats)
}

/// Daily timeseries for a specific UTM source.
pub async fn get_campaign_timeseries(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    utm_source: &str,
) -> AppResult<Vec<TimeseriesPoint>> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let mut rows: Vec<(NaiveDate, i64, i64, i64)> = sqlx::query_as(
        "SELECT date, COALESCE(SUM(visitors), 0)::bigint, \
         COALESCE(SUM(sessions), 0)::bigint, COALESCE(SUM(pageviews), 0)::bigint \
         FROM daily_campaigns \
         WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
         AND date < $5::date AND utm_source = $4 \
         GROUP BY date ORDER BY date",
    )
    .bind(project_id)
    .bind(start.naive_utc())
    .bind(end.naive_utc())
    .bind(utm_source)
    .bind(today)
    .fetch_all(db)
    .await?;

    // Add today's raw data if the range includes today
    if end_date >= today {
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let raw: (i64, i64, i64) = sqlx::query_as(
            "SELECT COUNT(DISTINCT visitor_id)::bigint, \
             COUNT(DISTINCT session_id)::bigint, COUNT(*)::bigint \
             FROM pageviews \
             WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
             AND utm_source = $4",
        )
        .bind(project_id)
        .bind(today_start)
        .bind(end)
        .bind(utm_source)
        .fetch_one(db)
        .await?;

        if raw.0 > 0 || raw.1 > 0 || raw.2 > 0 {
            rows.push((today, raw.0, raw.1, raw.2));
        }
    }

    let points = rows
        .into_iter()
        .map(|(date, visitors, sessions, pageviews)| TimeseriesPoint {
            date,
            visitors,
            sessions,
            pageviews,
        })
        .collect();

    Ok(points)
}

/// Top traffic sources.
pub async fn get_sources(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<SourceStat>> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, i64, i64, i64, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT
                utm_source,
                SUM(visitors)::bigint, SUM(sessions)::bigint,
                SUM(pageviews)::bigint, SUM(bounces)::bigint
            FROM (
                SELECT utm_source, visitors, sessions, pageviews, bounces
                FROM daily_campaigns
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT
                    COALESCE(utm_source, '')::varchar AS utm_source,
                    COUNT(DISTINCT visitor_id)::int AS visitors,
                    COUNT(DISTINCT session_id)::int AS sessions,
                    COUNT(*)::int AS pageviews,
                    0::int AS bounces
                FROM pageviews
                WHERE project_id = $1 AND created_at >= $6::date AND created_at <= $3
                AND utm_source IS NOT NULL
                GROUP BY COALESCE(utm_source, '')
            ) combined
            WHERE utm_source != ''
            GROUP BY utm_source
            ORDER BY 4 DESC
            LIMIT $4 OFFSET $5"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT
                utm_source,
                COALESCE(SUM(visitors), 0)::bigint,
                COALESCE(SUM(sessions), 0)::bigint,
                COALESCE(SUM(pageviews), 0)::bigint,
                COALESCE(SUM(bounces), 0)::bigint
            FROM daily_campaigns
            WHERE project_id = $1 AND date >= $2::date AND date <= $3::date
            AND utm_source != ''
            GROUP BY utm_source
            ORDER BY 4 DESC
            LIMIT $4 OFFSET $5"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };

    let stats = rows
        .into_iter()
        .map(|(source, visitors, sessions, pageviews, bounces)| {
            let bounce_rate = if sessions > 0 {
                (bounces as f64 / sessions as f64) * 100.0
            } else {
                0.0
            };
            SourceStat {
                utm_source: source,
                visitors,
                sessions,
                pageviews,
                bounce_rate,
            }
        })
        .collect();

    Ok(stats)
}

/// Top mediums.
pub async fn get_mediums(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<MediumStat>> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, i64, i64, i64, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT
                utm_medium,
                SUM(visitors)::bigint, SUM(sessions)::bigint,
                SUM(pageviews)::bigint, SUM(bounces)::bigint
            FROM (
                SELECT utm_medium, visitors, sessions, pageviews, bounces
                FROM daily_campaigns
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT
                    COALESCE(utm_medium, '')::varchar AS utm_medium,
                    COUNT(DISTINCT visitor_id)::int AS visitors,
                    COUNT(DISTINCT session_id)::int AS sessions,
                    COUNT(*)::int AS pageviews,
                    0::int AS bounces
                FROM pageviews
                WHERE project_id = $1 AND created_at >= $6::date AND created_at <= $3
                AND utm_medium IS NOT NULL
                GROUP BY COALESCE(utm_medium, '')
            ) combined
            WHERE utm_medium != ''
            GROUP BY utm_medium
            ORDER BY 4 DESC
            LIMIT $4 OFFSET $5"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT
                utm_medium,
                COALESCE(SUM(visitors), 0)::bigint,
                COALESCE(SUM(sessions), 0)::bigint,
                COALESCE(SUM(pageviews), 0)::bigint,
                COALESCE(SUM(bounces), 0)::bigint
            FROM daily_campaigns
            WHERE project_id = $1 AND date >= $2::date AND date <= $3::date
            AND utm_medium != ''
            GROUP BY utm_medium
            ORDER BY 4 DESC
            LIMIT $4 OFFSET $5"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };

    let stats = rows
        .into_iter()
        .map(|(medium, visitors, sessions, pageviews, bounces)| {
            let bounce_rate = if sessions > 0 {
                (bounces as f64 / sessions as f64) * 100.0
            } else {
                0.0
            };
            MediumStat {
                utm_medium: medium,
                visitors,
                sessions,
                pageviews,
                bounce_rate,
            }
        })
        .collect();

    Ok(stats)
}
