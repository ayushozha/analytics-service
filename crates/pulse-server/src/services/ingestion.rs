use std::sync::Arc;
use std::time::Duration;

use redis::AsyncCommands;
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info, warn};

use crate::models::buffered::{
    BufferedClickEvent, BufferedJsError, BufferedLogEntry, BufferedOutlink, BufferedScrollDepth,
    BufferedSearchQuery, BufferedWebVital,
};
use crate::models::event::BufferedEvent;
use crate::models::pageview::BufferedPageview;
use crate::state::AppState;

pub async fn push_pageview(
    state: &Arc<AppState>,
    pageview: &BufferedPageview,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:pageviews:{}", pageview.project_id));
    let serialized = serde_json::to_string(pageview)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_event(state: &Arc<AppState>, event: &BufferedEvent) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:events:{}", event.project_id));
    let serialized = serde_json::to_string(event)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_web_vital(
    state: &Arc<AppState>,
    vital: &BufferedWebVital,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:web_vitals:{}", vital.project_id));
    let serialized = serde_json::to_string(vital)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_scroll_depth(
    state: &Arc<AppState>,
    scroll: &BufferedScrollDepth,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:scroll_depths:{}", scroll.project_id));
    let serialized = serde_json::to_string(scroll)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_search_query(
    state: &Arc<AppState>,
    search: &BufferedSearchQuery,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:search_queries:{}", search.project_id));
    let serialized = serde_json::to_string(search)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_outlink(
    state: &Arc<AppState>,
    outlink: &BufferedOutlink,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:outlinks:{}", outlink.project_id));
    let serialized = serde_json::to_string(outlink)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_js_error(
    state: &Arc<AppState>,
    js_error: &BufferedJsError,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:js_errors:{}", js_error.project_id));
    let serialized = serde_json::to_string(js_error)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_log_entry(
    state: &Arc<AppState>,
    log_entry: &BufferedLogEntry,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:log_entries:{}", log_entry.project_id));
    let serialized = serde_json::to_string(log_entry)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_click_event(
    state: &Arc<AppState>,
    click: &BufferedClickEvent,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:click_events:{}", click.project_id));
    let serialized = serde_json::to_string(click)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn update_realtime(
    state: &Arc<AppState>,
    project_id: uuid::Uuid,
    visitor_id: &str,
) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("realtime:{project_id}"));
    let score = chrono::Utc::now().timestamp() as f64;
    let mut redis = state.redis.clone();
    let _: () = redis.zadd(&key, visitor_id, score).await?;
    // Set a TTL on the sorted set to auto-cleanup inactive projects
    let _: () = redis.expire(&key, 600).await.unwrap_or(());
    Ok(())
}

pub fn start_flush_task(state: Arc<AppState>) {
    let interval = Duration::from_secs(state.config.buffer_flush_interval_secs);
    let batch_size = state.config.buffer_batch_size;

    tokio::spawn(async move {
        let mut ticker = time::interval(interval);
        ticker.tick().await; // Skip first immediate tick

        loop {
            ticker.tick().await;
            if let Err(e) = flush_all_buffers(&state, batch_size).await {
                error!("Buffer flush error: {e}");
            }
        }
    });
}

async fn flush_all_buffers(state: &Arc<AppState>, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    // Get all pageview buffer keys
    let pv_pattern = state.redis_key("buffer:pageviews:*");
    let pv_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&pv_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &pv_keys {
        if let Err(e) = flush_pageviews(state, key, batch_size).await {
            error!("flush_pageviews failed for {key}: {e}");
        }
    }

    // Get all event buffer keys
    let ev_pattern = state.redis_key("buffer:events:*");
    let ev_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&ev_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &ev_keys {
        if let Err(e) = flush_events(state, key, batch_size).await {
            error!("flush_events failed for {key}: {e}");
        }
    }

    // Get all web_vitals buffer keys
    let wv_pattern = state.redis_key("buffer:web_vitals:*");
    let wv_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&wv_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &wv_keys {
        if let Err(e) = flush_web_vitals(state, key, batch_size).await {
            error!("flush_web_vitals failed for {key}: {e}");
        }
    }

    // Get all scroll_depths buffer keys
    let sd_pattern = state.redis_key("buffer:scroll_depths:*");
    let sd_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&sd_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &sd_keys {
        if let Err(e) = flush_scroll_depths(state, key, batch_size).await {
            error!("flush_scroll_depths failed for {key}: {e}");
        }
    }

    // Get all search_queries buffer keys
    let sq_pattern = state.redis_key("buffer:search_queries:*");
    let sq_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&sq_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &sq_keys {
        if let Err(e) = flush_search_queries(state, key, batch_size).await {
            error!("flush_search_queries failed for {key}: {e}");
        }
    }

    // Get all outlinks buffer keys
    let ol_pattern = state.redis_key("buffer:outlinks:*");
    let ol_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&ol_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &ol_keys {
        if let Err(e) = flush_outlinks(state, key, batch_size).await {
            error!("flush_outlinks failed for {key}: {e}");
        }
    }

    // Get all js_errors buffer keys
    let je_pattern = state.redis_key("buffer:js_errors:*");
    let je_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&je_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &je_keys {
        if let Err(e) = flush_js_errors(state, key, batch_size).await {
            error!("flush_js_errors failed for {key}: {e}");
        }
    }

    // Get all log_entries buffer keys
    let log_pattern = state.redis_key("buffer:log_entries:*");
    let log_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&log_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &log_keys {
        if let Err(e) = flush_log_entries(state, key, batch_size).await {
            error!("flush_log_entries failed for {key}: {e}");
        }
    }

    // Get all click_events buffer keys
    let ce_pattern = state.redis_key("buffer:click_events:*");
    let ce_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&ce_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &ce_keys {
        if let Err(e) = flush_click_events(state, key, batch_size).await {
            error!("flush_click_events failed for {key}: {e}");
        }
    }

    Ok(())
}

/// Flush one buffered event type from Redis to Postgres with durability guarantees:
/// malformed JSON is counted and logged (not silently discarded); and if the batch insert
/// fails, rows are retried individually so a single bad row cannot drop the whole batch —
/// rows that still fail are moved to a capped `<key>:dead` deadletter list rather than lost.
macro_rules! flush_buffer {
    ($state:expr, $key:expr, $batch_size:expr, $ty:ty, $insert:path, $label:expr) => {{
        let state = $state;
        let key = $key;
        let mut redis = state.redis.clone();

        let items: Vec<String> = redis
            .lpop(key, std::num::NonZero::new($batch_size))
            .await
            .unwrap_or_default();
        if items.is_empty() {
            return Ok(());
        }

        let mut rows: Vec<$ty> = Vec::with_capacity(items.len());
        let mut malformed = 0usize;
        for item in &items {
            match serde_json::from_str::<$ty>(item) {
                Ok(row) => rows.push(row),
                Err(_) => malformed += 1,
            }
        }
        if malformed > 0 {
            warn!(
                "{}: dropped {} malformed buffered item(s)",
                $label, malformed
            );
        }
        if rows.is_empty() {
            return Ok(());
        }

        match $insert(&state.db, &rows).await {
            Ok(()) => {
                info!("Flushed {} {} to PostgreSQL", rows.len(), $label);
            }
            Err(batch_err) => {
                warn!(
                    "{}: batch insert of {} row(s) failed ({batch_err}); retrying per-row",
                    $label,
                    rows.len()
                );
                let dead_key = format!("{}:dead", key);
                let mut recovered = 0usize;
                let mut dead = 0usize;
                for row in &rows {
                    if $insert(&state.db, std::slice::from_ref(row)).await.is_ok() {
                        recovered += 1;
                    } else {
                        dead += 1;
                        if let Ok(serialized) = serde_json::to_string(row) {
                            let _: Result<i64, _> = redis.rpush(&dead_key, serialized).await;
                        }
                    }
                }
                // Bound the deadletter list so a persistent failure can't exhaust Redis.
                let _: Result<(), _> = redis.ltrim(&dead_key, -10_000, -1).await;
                if dead > 0 {
                    error!(
                        "{}: {} row(s) recovered individually, {} moved to deadletter {}",
                        $label, recovered, dead, dead_key
                    );
                } else if recovered > 0 {
                    info!(
                        "{}: recovered all {} row(s) via per-row insert",
                        $label, recovered
                    );
                }
            }
        }
        Ok(())
    }};
}

async fn flush_pageviews(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedPageview,
        batch_insert_pageviews,
        "pageviews"
    )
}

async fn flush_events(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedEvent,
        batch_insert_events,
        "events"
    )
}

async fn flush_web_vitals(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedWebVital,
        batch_insert_web_vitals,
        "web vitals"
    )
}

async fn flush_scroll_depths(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedScrollDepth,
        batch_insert_scroll_depths,
        "scroll depths"
    )
}

async fn flush_search_queries(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedSearchQuery,
        batch_insert_search_queries,
        "search queries"
    )
}

async fn flush_outlinks(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedOutlink,
        batch_insert_outlinks,
        "outlinks"
    )
}

async fn flush_js_errors(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedJsError,
        batch_insert_js_errors,
        "JS errors"
    )
}

async fn flush_log_entries(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedLogEntry,
        batch_insert_log_entries,
        "log entries"
    )
}

async fn flush_click_events(
    state: &Arc<AppState>,
    key: &str,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    flush_buffer!(
        state,
        key,
        batch_size,
        BufferedClickEvent,
        batch_insert_click_events,
        "click events"
    )
}

async fn batch_insert_pageviews(
    db: &PgPool,
    pageviews: &[BufferedPageview],
) -> Result<(), anyhow::Error> {
    // Build a bulk insert
    let mut query = String::from(
        "INSERT INTO pageviews (project_id, session_id, visitor_id, path, title, referrer, referrer_domain, query_params, duration_ms, utm_source, utm_medium, utm_campaign, utm_content, utm_term, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in pageviews.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
            params_idx + 7,
            params_idx + 8,
            params_idx + 9,
            params_idx + 10,
            params_idx + 11,
            params_idx + 12,
            params_idx + 13,
            params_idx + 14,
        ));
        params_idx += 15;
    }

    let mut q = sqlx::query(&query);
    for pv in pageviews {
        q = q
            .bind(pv.project_id)
            .bind(pv.session_id)
            .bind(&pv.visitor_id)
            .bind(&pv.path)
            .bind(&pv.title)
            .bind(&pv.referrer)
            .bind(&pv.referrer_domain)
            .bind(&pv.query_params)
            .bind(pv.duration_ms)
            .bind(&pv.utm_source)
            .bind(&pv.utm_medium)
            .bind(&pv.utm_campaign)
            .bind(&pv.utm_content)
            .bind(&pv.utm_term)
            .bind(pv.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_events(db: &PgPool, events: &[BufferedEvent]) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO events (project_id, session_id, visitor_id, event_name, event_data, path, revenue_amount, revenue_currency, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in events.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
            params_idx + 7,
            params_idx + 8,
        ));
        params_idx += 9;
    }

    let mut q = sqlx::query(&query);
    for ev in events {
        q = q
            .bind(ev.project_id)
            .bind(ev.session_id)
            .bind(&ev.visitor_id)
            .bind(&ev.event_name)
            .bind(&ev.event_data)
            .bind(&ev.path)
            .bind(ev.revenue_amount)
            .bind(&ev.revenue_currency)
            .bind(ev.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_web_vitals(
    db: &PgPool,
    vitals: &[BufferedWebVital],
) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO web_vitals (project_id, visitor_id, session_id, path, metric_name, metric_value, rating, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in vitals.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
            params_idx + 7,
        ));
        params_idx += 8;
    }

    let mut q = sqlx::query(&query);
    for v in vitals {
        q = q
            .bind(v.project_id)
            .bind(&v.visitor_id)
            .bind(v.session_id)
            .bind(&v.path)
            .bind(&v.metric_name)
            .bind(v.metric_value)
            .bind(&v.rating)
            .bind(v.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_scroll_depths(
    db: &PgPool,
    scrolls: &[BufferedScrollDepth],
) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO scroll_depths (project_id, visitor_id, session_id, path, max_depth, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in scrolls.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
        ));
        params_idx += 6;
    }

    let mut q = sqlx::query(&query);
    for s in scrolls {
        q = q
            .bind(s.project_id)
            .bind(&s.visitor_id)
            .bind(s.session_id)
            .bind(&s.path)
            .bind(s.max_depth)
            .bind(s.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_search_queries(
    db: &PgPool,
    searches: &[BufferedSearchQuery],
) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO search_queries (project_id, visitor_id, session_id, query, results_count, path, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in searches.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
        ));
        params_idx += 7;
    }

    let mut q = sqlx::query(&query);
    for s in searches {
        q = q
            .bind(s.project_id)
            .bind(&s.visitor_id)
            .bind(s.session_id)
            .bind(&s.query)
            .bind(s.results_count)
            .bind(&s.path)
            .bind(s.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_outlinks(
    db: &PgPool,
    outlinks: &[BufferedOutlink],
) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO outlinks (project_id, visitor_id, session_id, url, link_type, path, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in outlinks.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
        ));
        params_idx += 7;
    }

    let mut q = sqlx::query(&query);
    for o in outlinks {
        q = q
            .bind(o.project_id)
            .bind(&o.visitor_id)
            .bind(o.session_id)
            .bind(&o.url)
            .bind(&o.link_type)
            .bind(&o.path)
            .bind(o.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_js_errors(
    db: &PgPool,
    errors: &[BufferedJsError],
) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO js_errors (project_id, visitor_id, session_id, message, stack, filename, lineno, colno, path, browser, os, release, environment, fingerprint, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in errors.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
            params_idx + 7,
            params_idx + 8,
            params_idx + 9,
            params_idx + 10,
            params_idx + 11,
            params_idx + 12,
            params_idx + 13,
            params_idx + 14,
        ));
        params_idx += 15;
    }

    let mut q = sqlx::query(&query);
    for e in errors {
        q = q
            .bind(e.project_id)
            .bind(&e.visitor_id)
            .bind(e.session_id)
            .bind(&e.message)
            .bind(&e.stack)
            .bind(&e.filename)
            .bind(e.lineno)
            .bind(e.colno)
            .bind(&e.path)
            .bind(&e.browser)
            .bind(&e.os)
            .bind(&e.release)
            .bind(&e.environment)
            .bind(&e.fingerprint)
            .bind(e.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_log_entries(
    db: &PgPool,
    logs: &[BufferedLogEntry],
) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO log_entries (project_id, visitor_id, session_id, level, message, body, path, browser, os, release, environment, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in logs.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
            params_idx + 7,
            params_idx + 8,
            params_idx + 9,
            params_idx + 10,
            params_idx + 11,
        ));
        params_idx += 12;
    }

    let mut q = sqlx::query(&query);
    for log in logs {
        q = q
            .bind(log.project_id)
            .bind(&log.visitor_id)
            .bind(log.session_id)
            .bind(&log.level)
            .bind(&log.message)
            .bind(&log.body)
            .bind(&log.path)
            .bind(&log.browser)
            .bind(&log.os)
            .bind(&log.release)
            .bind(&log.environment)
            .bind(log.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_click_events(
    db: &PgPool,
    clicks: &[BufferedClickEvent],
) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO click_events (project_id, visitor_id, session_id, path, x, y, element_selector, viewport_width, viewport_height, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in clicks.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx,
            params_idx + 1,
            params_idx + 2,
            params_idx + 3,
            params_idx + 4,
            params_idx + 5,
            params_idx + 6,
            params_idx + 7,
            params_idx + 8,
            params_idx + 9,
        ));
        params_idx += 10;
    }

    let mut q = sqlx::query(&query);
    for c in clicks {
        q = q
            .bind(c.project_id)
            .bind(&c.visitor_id)
            .bind(c.session_id)
            .bind(&c.path)
            .bind(c.x)
            .bind(c.y)
            .bind(&c.element_selector)
            .bind(c.viewport_width)
            .bind(c.viewport_height)
            .bind(c.created_at);
    }

    q.execute(db).await?;
    Ok(())
}
