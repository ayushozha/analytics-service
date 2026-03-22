use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Export overview stats as CSV.
pub async fn export_stats_csv(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let today = Utc::now().date_naive();

    let rows: Vec<(NaiveDate, i64, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT date, pageviews, visitors, sessions, bounces, total_duration_ms \
         FROM daily_stats WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
         AND date < $4::date ORDER BY date",
    )
    .bind(project_id)
    .bind(start.naive_utc())
    .bind(end.naive_utc())
    .bind(today)
    .fetch_all(db)
    .await?;

    let mut csv = String::from("date,pageviews,visitors,sessions,bounces,total_duration_ms\n");
    for r in &rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            r.0, r.1, r.2, r.3, r.4, r.5
        ));
    }

    // Add today's raw data if in range
    let end_date = end.date_naive();
    if end_date >= today {
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let raw_start = if start > today_start {
            start
        } else {
            today_start
        };

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

        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            today, raw.0, raw.1, raw.2, bounces.0, duration.0
        ));
    }

    Ok(csv)
}

/// Export top pages as CSV.
pub async fn export_pages_csv(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, i64, i64, i32)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT path, SUM(views)::bigint, SUM(uv)::bigint, AVG(avg_dur)::int FROM (
                SELECT path, views, unique_views as uv, avg_duration_ms as avg_dur
                FROM daily_pages WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $4::date
                UNION ALL
                SELECT path, COUNT(*)::bigint as views, COUNT(DISTINCT visitor_id)::bigint as uv, COALESCE(AVG(duration_ms), 0)::int as avg_dur
                FROM pageviews WHERE project_id = $1 AND created_at >= $4::date AND created_at <= $3
                GROUP BY path
            ) combined GROUP BY path ORDER BY 2 DESC"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT path, COALESCE(SUM(views), 0)::bigint, COALESCE(SUM(unique_views), 0)::bigint, \
             COALESCE(AVG(avg_duration_ms), 0)::int FROM daily_pages \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY path ORDER BY 2 DESC",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .fetch_all(db)
        .await?
    };

    let mut csv = String::from("path,views,unique_views,avg_duration_ms\n");
    for r in &rows {
        csv.push_str(&format!(
            "\"{}\",{},{},{}\n",
            r.0.replace('"', "\"\""),
            r.1,
            r.2,
            r.3
        ));
    }
    Ok(csv)
}

/// Export referrers as CSV.
pub async fn export_referrers_csv(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT domain, SUM(visits)::bigint FROM (
                SELECT referrer_domain as domain, visits FROM daily_referrers
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $4::date
                UNION ALL
                SELECT COALESCE(referrer_domain, 'Direct') as domain, COUNT(DISTINCT session_id)::bigint as visits
                FROM pageviews WHERE project_id = $1 AND created_at >= $4::date AND created_at <= $3
                GROUP BY referrer_domain
            ) combined GROUP BY domain ORDER BY 2 DESC"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT referrer_domain, COALESCE(SUM(visits), 0)::bigint FROM daily_referrers \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY referrer_domain ORDER BY 2 DESC",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .fetch_all(db)
        .await?
    };

    let mut csv = String::from("referrer_domain,visits\n");
    for r in &rows {
        csv.push_str(&format!(
            "\"{}\",{}\n",
            r.0.replace('"', "\"\""),
            r.1
        ));
    }
    Ok(csv)
}

/// Export events as CSV.
pub async fn export_events_csv(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT name, SUM(cnt)::bigint FROM (
                SELECT event_name as name, count as cnt FROM daily_events
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $4::date
                UNION ALL
                SELECT event_name as name, COUNT(*)::bigint as cnt
                FROM events WHERE project_id = $1 AND created_at >= $4::date AND created_at <= $3
                GROUP BY event_name
            ) combined GROUP BY name ORDER BY 2 DESC"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT event_name, COALESCE(SUM(count), 0)::bigint FROM daily_events \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY event_name ORDER BY 2 DESC",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .fetch_all(db)
        .await?
    };

    let mut csv = String::from("event_name,count\n");
    for r in &rows {
        csv.push_str(&format!(
            "\"{}\",{}\n",
            r.0.replace('"', "\"\""),
            r.1
        ));
    }
    Ok(csv)
}

/// Export devices as CSV.
pub async fn export_devices_csv(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, String, String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT browser, os, device, SUM(visitors)::bigint FROM (
                SELECT browser, os, device, visitors FROM daily_devices
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $4::date
                UNION ALL
                SELECT COALESCE(browser, 'Unknown'), COALESCE(os, 'Unknown'), COALESCE(device, 'desktop'), COUNT(DISTINCT visitor_id)::bigint
                FROM sessions WHERE project_id = $1 AND first_at >= $4::date AND first_at <= $3
                GROUP BY browser, os, device
            ) combined GROUP BY browser, os, device ORDER BY 4 DESC"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT browser, os, device, COALESCE(SUM(visitors), 0)::bigint FROM daily_devices \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY browser, os, device ORDER BY 4 DESC",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .fetch_all(db)
        .await?
    };

    let mut csv = String::from("browser,os,device,visitors\n");
    for r in &rows {
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",{}\n",
            r.0.replace('"', "\"\""),
            r.1.replace('"', "\"\""),
            r.2.replace('"', "\"\""),
            r.3
        ));
    }
    Ok(csv)
}

/// Export geo data as CSV.
pub async fn export_geo_csv(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT country, SUM(visitors)::bigint FROM (
                SELECT country, visitors FROM daily_geo
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $4::date
                UNION ALL
                SELECT COALESCE(country, 'XX'), COUNT(DISTINCT visitor_id)::bigint
                FROM sessions WHERE project_id = $1 AND first_at >= $4::date AND first_at <= $3
                GROUP BY country
            ) combined GROUP BY country ORDER BY 2 DESC"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT country, COALESCE(SUM(visitors), 0)::bigint FROM daily_geo \
             WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY country ORDER BY 2 DESC",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .fetch_all(db)
        .await?
    };

    let mut csv = String::from("country,visitors\n");
    for r in &rows {
        csv.push_str(&format!("{},{}\n", r.0, r.1));
    }
    Ok(csv)
}

/// Export campaign data as CSV.
pub async fn export_campaigns_csv(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let today = Utc::now().date_naive();
    let end_date = end.date_naive();

    let rows: Vec<(String, String, String, i64, i64, i64, i64)> = if end_date >= today {
        sqlx::query_as(
            r#"SELECT utm_source, utm_medium, utm_campaign, SUM(visitors)::bigint, SUM(sessions)::bigint, SUM(pageviews)::bigint, SUM(bounces)::bigint FROM (
                SELECT utm_source, utm_medium, utm_campaign, visitors, sessions, pageviews, bounces
                FROM daily_campaigns
                WHERE project_id = $1 AND date >= $2::date AND date <= $3::date AND date < $4::date
                UNION ALL
                SELECT COALESCE(utm_source, ''), COALESCE(utm_medium, ''), COALESCE(utm_campaign, ''),
                       COUNT(DISTINCT visitor_id)::bigint, COUNT(DISTINCT session_id)::bigint, COUNT(*)::bigint, 0::bigint
                FROM pageviews WHERE project_id = $1 AND created_at >= $4::date AND created_at <= $3
                AND utm_source IS NOT NULL
                GROUP BY utm_source, utm_medium, utm_campaign
            ) combined GROUP BY utm_source, utm_medium, utm_campaign ORDER BY 4 DESC"#,
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .bind(today)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT utm_source, utm_medium, utm_campaign, COALESCE(SUM(visitors), 0)::bigint, \
             COALESCE(SUM(sessions), 0)::bigint, COALESCE(SUM(pageviews), 0)::bigint, \
             COALESCE(SUM(bounces), 0)::bigint \
             FROM daily_campaigns WHERE project_id = $1 AND date >= $2::date AND date <= $3::date \
             GROUP BY utm_source, utm_medium, utm_campaign ORDER BY 4 DESC",
        )
        .bind(project_id)
        .bind(start.naive_utc())
        .bind(end.naive_utc())
        .fetch_all(db)
        .await?
    };

    let mut csv =
        String::from("utm_source,utm_medium,utm_campaign,visitors,sessions,pageviews,bounces\n");
    for r in &rows {
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",{},{},{},{}\n",
            r.0.replace('"', "\"\""),
            r.1.replace('"', "\"\""),
            r.2.replace('"', "\"\""),
            r.3,
            r.4,
            r.5,
            r.6
        ));
    }
    Ok(csv)
}
