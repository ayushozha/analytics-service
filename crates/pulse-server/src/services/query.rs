use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use redis::AsyncCommands;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::SharedState;

/// Stats tuple: (pageviews, visitors, sessions, bounces, total_duration_ms)
pub type StatsTuple = (i64, i64, i64, i64, i64);

/// Timeseries point
pub type TimeseriesPoint = serde_json::Value;

pub async fn get_umami_website_id(state: &SharedState, project_id: Uuid) -> Option<String> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT umami_website_id FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_optional(&state.db)
            .await
            .ok()?;
    row.and_then(|r| r.0)
}

pub async fn fetch_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
) -> Result<StatsTuple, sqlx::Error> {
    let rollup: StatsTuple = sqlx::query_as(
        "SELECT COALESCE(SUM(pageviews), 0)::bigint, COALESCE(SUM(visitors), 0)::bigint, \
         COALESCE(SUM(sessions), 0)::bigint, COALESCE(SUM(bounces), 0)::bigint, \
         COALESCE(SUM(total_duration_ms), 0)::bigint \
         FROM daily_stats WHERE project_id = $1 \
         AND date >= $2::date AND date <= $3::date AND date < $4::date",
    )
    .bind(project_id)
    .bind(start.naive_utc())
    .bind(end.naive_utc())
    .bind(today)
    .fetch_one(db)
    .await?;

    let end_date = end.date_naive();
    if end_date >= today {
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let raw_start = if start > today_start { start } else { today_start };

        let raw: (i64, i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), COUNT(DISTINCT visitor_id), COUNT(DISTINCT session_id) \
             FROM pageviews WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3",
        )
        .bind(project_id)
        .bind(raw_start)
        .bind(end)
        .fetch_one(db)
        .await?;

        let bounces: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sessions WHERE project_id = $1 \
             AND first_at >= $2 AND first_at <= $3 AND is_bounce = true",
        )
        .bind(project_id)
        .bind(raw_start)
        .bind(end)
        .fetch_one(db)
        .await?;

        let duration: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(duration_ms), 0)::bigint FROM sessions \
             WHERE project_id = $1 AND first_at >= $2 AND first_at <= $3",
        )
        .bind(project_id)
        .bind(raw_start)
        .bind(end)
        .fetch_one(db)
        .await?;

        Ok((
            rollup.0 + raw.0,
            rollup.1 + raw.1,
            rollup.2 + raw.2,
            rollup.3 + bounces.0,
            rollup.4 + duration.0,
        ))
    } else {
        Ok(rollup)
    }
}

pub async fn fetch_events_count(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
) -> Result<i64, sqlx::Error> {
    let rollup: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(count), 0)::bigint FROM daily_events WHERE project_id = $1 \
         AND date >= $2::date AND date <= $3::date AND date < $4::date",
    )
    .bind(project_id)
    .bind(start.naive_utc())
    .bind(end.naive_utc())
    .bind(today)
    .fetch_one(db)
    .await?;

    let end_date = end.date_naive();
    if end_date >= today {
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let raw_start = if start > today_start { start } else { today_start };

        let raw: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM events WHERE project_id = $1 \
             AND created_at >= $2 AND created_at <= $3",
        )
        .bind(project_id)
        .bind(raw_start)
        .bind(end)
        .fetch_one(db)
        .await?;

        Ok(rollup.0 + raw.0)
    } else {
        Ok(rollup.0)
    }
}

pub async fn fetch_timeseries(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
) -> Result<Vec<TimeseriesPoint>, sqlx::Error> {
    let rollup_rows: Vec<(NaiveDate, i64, i64, i64)> = sqlx::query_as(
        "SELECT date, pageviews, visitors, sessions FROM daily_stats \
         WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
         AND date < $4::date ORDER BY date",
    )
    .bind(project_id)
    .bind(start.naive_utc())
    .bind(end.naive_utc())
    .bind(today)
    .fetch_all(db)
    .await?;

    let mut data: Vec<serde_json::Value> = rollup_rows
        .iter()
        .map(|r| {
            json!({
                "date": r.0.to_string(),
                "pageviews": r.1,
                "visitors": r.2,
                "sessions": r.3,
            })
        })
        .collect();

    let end_date = end.date_naive();
    if end_date >= today {
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let raw: (i64, i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), COUNT(DISTINCT visitor_id), COUNT(DISTINCT session_id) \
             FROM pageviews WHERE project_id = $1 \
             AND created_at >= $2 AND created_at <= $3",
        )
        .bind(project_id)
        .bind(today_start)
        .bind(end)
        .fetch_one(db)
        .await?;

        data.push(json!({
            "date": today.to_string(),
            "pageviews": raw.0,
            "visitors": raw.1,
            "sessions": raw.2,
        }));
    }

    Ok(data)
}

pub async fn fetch_pages(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let end_date = end.date_naive();

    let rows: Vec<(String, i64, i64, i32)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT path, SUM(views)::bigint, SUM(uv)::bigint, AVG(avg_dur)::int FROM (
                SELECT path, views, unique_views as uv, avg_duration_ms as avg_dur
                FROM daily_pages WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT path, COUNT(*)::bigint as views, COUNT(DISTINCT visitor_id)::bigint as uv, COALESCE(AVG(duration_ms), 0)::int as avg_dur
                FROM pageviews WHERE project_id = $1 AND created_at >= $6::date AND created_at <= $3
                GROUP BY path
            ) combined GROUP BY path ORDER BY 2 DESC LIMIT $4 OFFSET $5"#,
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
            "SELECT path, COALESCE(SUM(views), 0)::bigint, COALESCE(SUM(unique_views), 0)::bigint, \
             COALESCE(AVG(avg_duration_ms), 0)::int FROM daily_pages \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY path ORDER BY 2 DESC LIMIT $4 OFFSET $5",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "path": r.0,
                "views": r.1,
                "unique_views": r.2,
                "avg_duration": r.3,
            })
        })
        .collect())
}

pub async fn fetch_referrers(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let end_date = end.date_naive();

    let rows: Vec<(String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT domain, SUM(visits)::bigint FROM (
                SELECT referrer_domain as domain, visits FROM daily_referrers
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT COALESCE(referrer_domain, 'Direct') as domain, COUNT(DISTINCT session_id)::bigint as visits
                FROM pageviews WHERE project_id = $1 AND created_at >= $6::date AND created_at <= $3
                GROUP BY referrer_domain
            ) combined GROUP BY domain ORDER BY 2 DESC LIMIT $4 OFFSET $5"#,
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
            "SELECT referrer_domain, COALESCE(SUM(visits), 0)::bigint FROM daily_referrers \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY referrer_domain ORDER BY 2 DESC LIMIT $4 OFFSET $5",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| json!({ "referrer_domain": r.0, "visits": r.1 }))
        .collect())
}

pub async fn fetch_events(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let end_date = end.date_naive();

    let rows: Vec<(String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT name, SUM(cnt)::bigint FROM (
                SELECT event_name as name, count as cnt FROM daily_events
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT event_name as name, COUNT(*)::bigint as cnt
                FROM events WHERE project_id = $1 AND created_at >= $6::date AND created_at <= $3
                GROUP BY event_name
            ) combined GROUP BY name ORDER BY 2 DESC LIMIT $4 OFFSET $5"#,
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
            "SELECT event_name, COALESCE(SUM(count), 0)::bigint FROM daily_events \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY event_name ORDER BY 2 DESC LIMIT $4 OFFSET $5",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| json!({ "event_name": r.0, "count": r.1 }))
        .collect())
}

pub async fn fetch_devices(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let end_date = end.date_naive();

    let rows: Vec<(String, String, String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT browser, os, device, SUM(visitors)::bigint FROM (
                SELECT browser, os, device, visitors FROM daily_devices
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT COALESCE(browser, 'Unknown'), COALESCE(os, 'Unknown'), COALESCE(device, 'desktop'), COUNT(DISTINCT visitor_id)::bigint
                FROM sessions WHERE project_id = $1 AND first_at >= $6::date AND first_at <= $3
                GROUP BY browser, os, device
            ) combined GROUP BY browser, os, device ORDER BY 4 DESC LIMIT $4 OFFSET $5"#,
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
            "SELECT browser, os, device, COALESCE(SUM(visitors), 0)::bigint FROM daily_devices \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY browser, os, device ORDER BY 4 DESC LIMIT $4 OFFSET $5",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "browser": r.0,
                "os": r.1,
                "device": r.2,
                "visitors": r.3,
            })
        })
        .collect())
}

pub async fn fetch_geo(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let end_date = end.date_naive();

    let rows: Vec<(String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT country, SUM(visitors)::bigint FROM (
                SELECT country, visitors FROM daily_geo
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $6::date
                UNION ALL
                SELECT COALESCE(country, 'XX'), COUNT(DISTINCT visitor_id)::bigint
                FROM sessions WHERE project_id = $1 AND first_at >= $6::date AND first_at <= $3
                GROUP BY country
            ) combined GROUP BY country ORDER BY 2 DESC LIMIT $4 OFFSET $5"#,
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
            "SELECT country, COALESCE(SUM(visitors), 0)::bigint FROM daily_geo \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY country ORDER BY 2 DESC LIMIT $4 OFFSET $5",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| json!({ "country": r.0, "visitors": r.1 }))
        .collect())
}

pub async fn fetch_realtime(
    state: &SharedState,
    project_id: Uuid,
) -> Result<i64, anyhow::Error> {
    let key = state.redis_key(&format!("realtime:{}", project_id));
    let mut redis = state.redis.clone();

    let five_min_ago = (Utc::now().timestamp() - 300) as f64;
    let now = Utc::now().timestamp() as f64;

    let active_visitors: i64 = redis.zcount(&key, five_min_ago, now).await.unwrap_or(0);
    let _: () = redis
        .zrembyscore(&key, f64::NEG_INFINITY, five_min_ago)
        .await
        .unwrap_or(());

    Ok(active_visitors)
}

// ── Visitor queries ──

pub async fn fetch_visitors_list(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
    offset: i64,
    search: Option<&str>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let search_pattern = search.map(|s| format!("{s}%"));

    let rows: Vec<(String, i64, i64, i64, DateTime<Utc>, DateTime<Utc>, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT s.visitor_id, COUNT(DISTINCT s.id)::bigint, \
             COALESCE(SUM(s.pageview_count), 0)::bigint, \
             COALESCE(SUM(s.event_count), 0)::bigint, \
             MAX(s.last_at), MIN(s.first_at), \
             (array_agg(s.country ORDER BY s.last_at DESC))[1], \
             (array_agg(s.browser ORDER BY s.last_at DESC))[1], \
             (array_agg(s.device ORDER BY s.last_at DESC))[1] \
             FROM sessions s \
             WHERE s.project_id = $1 AND s.first_at >= $2 AND s.first_at <= $3 \
             AND ($6::text IS NULL OR s.visitor_id LIKE $6) \
             GROUP BY s.visitor_id ORDER BY MAX(s.last_at) DESC \
             LIMIT $4 OFFSET $5",
        )
        .bind(project_id)
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
        .bind(&search_pattern)
        .fetch_all(db)
        .await?;

    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "visitor_id": r.0,
                "session_count": r.1,
                "total_pageviews": r.2,
                "total_events": r.3,
                "last_seen": r.4.to_rfc3339(),
                "first_seen": r.5.to_rfc3339(),
                "country": r.6,
                "browser": r.7,
                "device": r.8,
            })
        })
        .collect())
}

pub async fn fetch_recent_activity(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows: Vec<(String, String, Option<String>, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT activity_type, visitor_id, detail, event_name, created_at FROM (
            (SELECT 'pageview'::text as activity_type, visitor_id, path as detail,
             NULL::text as event_name, created_at
             FROM pageviews WHERE project_id = $1 AND created_at >= NOW() - interval '1 hour'
             ORDER BY created_at DESC LIMIT $2)
            UNION ALL
            (SELECT 'event'::text as activity_type, visitor_id, path as detail,
             event_name, created_at
             FROM events WHERE project_id = $1 AND created_at >= NOW() - interval '1 hour'
             ORDER BY created_at DESC LIMIT $2)
        ) combined ORDER BY created_at DESC LIMIT $2"#,
    )
    .bind(project_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "activity_type": r.0,
                "visitor_id": r.1,
                "detail": r.2,
                "event_name": r.3,
                "created_at": r.4.to_rfc3339(),
            })
        })
        .collect())
}

pub async fn fetch_visitor_summary(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<serde_json::Value, sqlx::Error> {
    let row: (i64, i64, i64, Option<DateTime<Utc>>, Option<DateTime<Utc>>, i64,
              Option<String>, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT COUNT(DISTINCT s.id)::bigint, \
         COALESCE(SUM(s.pageview_count), 0)::bigint, \
         COALESCE(SUM(s.event_count), 0)::bigint, \
         MIN(s.first_at), MAX(s.last_at), \
         COALESCE(SUM(s.duration_ms), 0)::bigint, \
         (array_agg(s.country ORDER BY s.last_at DESC))[1], \
         (array_agg(s.browser ORDER BY s.last_at DESC))[1], \
         (array_agg(s.os ORDER BY s.last_at DESC))[1], \
         (array_agg(s.device ORDER BY s.last_at DESC))[1] \
         FROM sessions s \
         WHERE s.project_id = $1 AND s.visitor_id = $2 \
         AND s.first_at >= $3 AND s.first_at <= $4",
    )
    .bind(project_id)
    .bind(visitor_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    let pricing: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM pageviews \
         WHERE project_id = $1 AND visitor_id = $2 \
         AND created_at >= $3 AND created_at <= $4 \
         AND (path LIKE '%/pricing%' OR path LIKE '%/plans%')",
    )
    .bind(project_id)
    .bind(visitor_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    Ok(json!({
        "session_count": row.0,
        "total_pageviews": row.1,
        "total_events": row.2,
        "first_seen": row.3.map(|d| d.to_rfc3339()),
        "last_seen": row.4.map(|d| d.to_rfc3339()),
        "total_duration_ms": row.5,
        "country": row.6,
        "browser": row.7,
        "os": row.8,
        "device": row.9,
        "pricing_views": pricing.0,
    }))
}

pub async fn fetch_visitor_sessions(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows: Vec<(Uuid, DateTime<Utc>, DateTime<Utc>, Option<String>, Option<String>,
                   i32, i32, i64, bool, Option<String>, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT id, first_at, last_at, entry_page, exit_page, pageview_count, \
             event_count, duration_ms, is_bounce, browser, os, device, country \
             FROM sessions \
             WHERE project_id = $1 AND visitor_id = $2 \
             AND first_at >= $3 AND first_at <= $4 \
             ORDER BY first_at DESC LIMIT 100",
        )
        .bind(project_id)
        .bind(visitor_id)
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await?;

    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "id": r.0.to_string(),
                "first_at": r.1.to_rfc3339(),
                "last_at": r.2.to_rfc3339(),
                "entry_page": r.3,
                "exit_page": r.4,
                "pageview_count": r.5,
                "event_count": r.6,
                "duration_ms": r.7,
                "is_bounce": r.8,
                "browser": r.9,
                "os": r.10,
                "device": r.11,
                "country": r.12,
            })
        })
        .collect())
}

pub async fn fetch_session_detail(
    db: &PgPool,
    project_id: Uuid,
    session_id: Uuid,
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), sqlx::Error> {
    let pv_rows: Vec<(String, Option<String>, Option<String>, Option<i32>, DateTime<Utc>)> =
        sqlx::query_as(
            "SELECT path, title, referrer, duration_ms, created_at \
             FROM pageviews WHERE project_id = $1 AND session_id = $2 \
             ORDER BY created_at ASC",
        )
        .bind(project_id)
        .bind(session_id)
        .fetch_all(db)
        .await?;

    let ev_rows: Vec<(String, Option<serde_json::Value>, Option<String>, DateTime<Utc>)> =
        sqlx::query_as(
            "SELECT event_name, event_data, path, created_at \
             FROM events WHERE project_id = $1 AND session_id = $2 \
             ORDER BY created_at ASC",
        )
        .bind(project_id)
        .bind(session_id)
        .fetch_all(db)
        .await?;

    let pageviews = pv_rows
        .iter()
        .map(|r| {
            json!({
                "path": r.0,
                "title": r.1,
                "referrer": r.2,
                "duration_ms": r.3,
                "created_at": r.4.to_rfc3339(),
            })
        })
        .collect();

    let events = ev_rows
        .iter()
        .map(|r| {
            json!({
                "event_name": r.0,
                "event_data": r.1,
                "path": r.2,
                "created_at": r.3.to_rfc3339(),
            })
        })
        .collect();

    Ok((pageviews, events))
}

pub async fn fetch_visitor_daily_activity(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        "SELECT created_at::date as date, COUNT(*)::bigint as pageviews \
         FROM pageviews WHERE project_id = $1 AND visitor_id = $2 \
         AND created_at >= $3 AND created_at <= $4 \
         GROUP BY created_at::date ORDER BY date",
    )
    .bind(project_id)
    .bind(visitor_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| json!({ "date": r.0.to_string(), "pageviews": r.1 }))
        .collect())
}

pub async fn fetch_visitor_event_breakdown(
    db: &PgPool,
    project_id: Uuid,
    visitor_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT event_name, COUNT(*)::bigint as count \
         FROM events WHERE project_id = $1 AND visitor_id = $2 \
         AND created_at >= $3 AND created_at <= $4 \
         GROUP BY event_name ORDER BY count DESC LIMIT 20",
    )
    .bind(project_id)
    .bind(visitor_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| json!({ "event_name": r.0, "count": r.1 }))
        .collect())
}

// ── Pricing queries ──

pub async fn fetch_pricing_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
) -> Result<(i64, i64, f64, f64), sqlx::Error> {
    let end_date = end.date_naive();

    // Rollup portion
    let rollup: (i64, i64, f64) = sqlx::query_as(
        "SELECT COALESCE(SUM(views), 0)::bigint, \
         COALESCE(SUM(unique_views), 0)::bigint, \
         COALESCE(AVG(avg_duration_ms), 0)::float8 \
         FROM daily_pages WHERE project_id = $1 \
         AND date >= $2::date AND date <= $3::date AND date < $4::date \
         AND (path LIKE '%/pricing%' OR path LIKE '%/plans%')",
    )
    .bind(project_id)
    .bind(start.naive_utc())
    .bind(end.naive_utc())
    .bind(today)
    .fetch_one(db)
    .await?;

    let (mut views, mut visitors, mut avg_dur) = (rollup.0, rollup.1, rollup.2);

    if end_date >= today {
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let raw_start = if start > today_start { start } else { today_start };

        let raw: (i64, i64, f64) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COUNT(DISTINCT visitor_id)::bigint, \
             COALESCE(AVG(duration_ms), 0)::float8 \
             FROM pageviews WHERE project_id = $1 \
             AND created_at >= $2 AND created_at <= $3 \
             AND (path LIKE '%/pricing%' OR path LIKE '%/plans%')",
        )
        .bind(project_id)
        .bind(raw_start)
        .bind(end)
        .fetch_one(db)
        .await?;

        let total = views + raw.0;
        if total > 0 {
            avg_dur = (avg_dur * views as f64 + raw.2 * raw.0 as f64) / total as f64;
        }
        views += raw.0;
        visitors += raw.1;
    }

    // Bounce rate from pricing entry pages
    let bounce_row: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*) FILTER (WHERE is_bounce = true)::bigint, COUNT(*)::bigint \
         FROM sessions WHERE project_id = $1 \
         AND first_at >= $2 AND first_at <= $3 \
         AND (entry_page LIKE '%/pricing%' OR entry_page LIKE '%/plans%')",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    let bounce_rate = if bounce_row.1 > 0 {
        bounce_row.0 as f64 / bounce_row.1 as f64 * 100.0
    } else {
        0.0
    };

    Ok((views, visitors, avg_dur, bounce_rate))
}

pub async fn fetch_pricing_timeseries(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    today: NaiveDate,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rollup_rows: Vec<(NaiveDate, i64, i64)> = sqlx::query_as(
        "SELECT date, COALESCE(SUM(views), 0)::bigint, \
         COALESCE(SUM(unique_views), 0)::bigint \
         FROM daily_pages WHERE project_id = $1 \
         AND date >= $2::date AND date <= $3::date AND date < $4::date \
         AND (path LIKE '%/pricing%' OR path LIKE '%/plans%') \
         GROUP BY date ORDER BY date",
    )
    .bind(project_id)
    .bind(start.naive_utc())
    .bind(end.naive_utc())
    .bind(today)
    .fetch_all(db)
    .await?;

    let mut data: Vec<serde_json::Value> = rollup_rows
        .iter()
        .map(|r| json!({ "date": r.0.to_string(), "views": r.1, "unique_views": r.2 }))
        .collect();

    let end_date = end.date_naive();
    if end_date >= today {
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let raw: (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COUNT(DISTINCT visitor_id)::bigint \
             FROM pageviews WHERE project_id = $1 \
             AND created_at >= $2 AND created_at <= $3 \
             AND (path LIKE '%/pricing%' OR path LIKE '%/plans%')",
        )
        .bind(project_id)
        .bind(today_start)
        .bind(end)
        .fetch_one(db)
        .await?;

        data.push(json!({ "date": today.to_string(), "views": raw.0, "unique_views": raw.1 }));
    }

    Ok(data)
}

pub async fn fetch_pricing_frequency(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        r#"SELECT visit_count::bigint, COUNT(*)::bigint as visitor_count FROM (
            SELECT visitor_id, COUNT(*) as visit_count
            FROM pageviews WHERE project_id = $1
            AND created_at >= $2 AND created_at <= $3
            AND (path LIKE '%/pricing%' OR path LIKE '%/plans%')
            GROUP BY visitor_id
        ) visitor_visits GROUP BY visit_count ORDER BY visit_count"#,
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    // Bucket into 1x, 2x, 3x, 4x, 5x+
    let mut buckets = vec![0i64; 5];
    for r in &rows {
        let idx = if r.0 >= 5 { 4 } else { (r.0 - 1) as usize };
        buckets[idx] += r.1;
    }

    Ok((0..5)
        .map(|i| {
            let label = if i == 4 {
                "5+".to_string()
            } else {
                format!("{}", i + 1)
            };
            json!({ "visits": label, "visitor_count": buckets[i] })
        })
        .collect())
}

pub async fn fetch_pricing_referrers(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT COALESCE(referrer_domain, 'Direct'), COUNT(DISTINCT session_id)::bigint \
         FROM pageviews WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         AND (path LIKE '%/pricing%' OR path LIKE '%/plans%') \
         GROUP BY referrer_domain ORDER BY 2 DESC LIMIT $4",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| json!({ "referrer_domain": r.0, "visits": r.1 }))
        .collect())
}

pub async fn fetch_pricing_heatmap(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows: Vec<(f64, f64, i64)> = sqlx::query_as(
        "SELECT EXTRACT(DOW FROM created_at)::float8, \
         EXTRACT(HOUR FROM created_at)::float8, \
         COUNT(*)::bigint \
         FROM pageviews WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         AND (path LIKE '%/pricing%' OR path LIKE '%/plans%') \
         GROUP BY 1, 2 ORDER BY 1, 2",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "day_of_week": r.0 as i32,
                "hour_of_day": r.1 as i32,
                "views": r.2,
            })
        })
        .collect())
}

pub async fn fetch_funnel(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    steps: &[String],
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let mut results = Vec::new();
    for step in steps {
        let pattern = format!("{step}%");
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT visitor_id)::bigint FROM pageviews \
             WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
             AND path LIKE $4",
        )
        .bind(project_id)
        .bind(start)
        .bind(end)
        .bind(&pattern)
        .fetch_one(db)
        .await?;

        results.push(json!({
            "step": step,
            "visitors": row.0,
        }));
    }
    Ok(results)
}

// ── Merge helpers ──

pub fn merge_page_data(
    pulse_pages: Vec<serde_json::Value>,
    umami_pages: Vec<crate::services::umami_client::UmamiPageview>,
) -> Vec<serde_json::Value> {
    let mut merged: HashMap<String, serde_json::Value> = HashMap::new();

    for page in &pulse_pages {
        let path = page["path"].as_str().unwrap_or("").to_string();
        merged.insert(path, page.clone());
    }

    for up in &umami_pages {
        let entry = merged.entry(up.x.clone()).or_insert_with(|| {
            json!({
                "path": up.x,
                "views": 0i64,
                "unique_views": 0i64,
                "avg_duration": 0,
            })
        });
        if let Some(obj) = entry.as_object_mut() {
            let existing = obj.get("views").and_then(|v| v.as_i64()).unwrap_or(0);
            obj.insert("views".to_string(), json!(existing + up.y));
            obj.insert("umami_views".to_string(), json!(up.y));
        }
    }

    let mut result: Vec<serde_json::Value> = merged.into_values().collect();
    result.sort_by(|a, b| {
        let va = a["views"].as_i64().unwrap_or(0);
        let vb = b["views"].as_i64().unwrap_or(0);
        vb.cmp(&va)
    });
    result
}

pub fn merge_kv_data(
    pulse_data: Vec<serde_json::Value>,
    key_field: &str,
    value_field: &str,
    umami_data: &[(String, i64)],
) -> Vec<serde_json::Value> {
    let mut merged: HashMap<String, i64> = HashMap::new();

    for item in &pulse_data {
        let k = item[key_field].as_str().unwrap_or("").to_string();
        let v = item[value_field].as_i64().unwrap_or(0);
        *merged.entry(k).or_insert(0) += v;
    }

    for (k, v) in umami_data {
        *merged.entry(k.clone()).or_insert(0) += v;
    }

    let mut result: Vec<(String, i64)> = merged.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));

    result
        .into_iter()
        .map(|(k, v)| json!({ key_field: k, value_field: v }))
        .collect()
}
