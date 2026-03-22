use std::sync::Arc;
use std::time::Duration;

use redis::AsyncCommands;
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info};

use crate::models::buffered::{
    BufferedClickEvent, BufferedJsError, BufferedOutlink, BufferedScrollDepth, BufferedSearchQuery,
    BufferedWebVital,
};
use crate::models::event::BufferedEvent;
use crate::models::pageview::BufferedPageview;
use crate::state::AppState;

pub async fn push_pageview(state: &Arc<AppState>, pageview: &BufferedPageview) -> Result<(), anyhow::Error> {
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

pub async fn push_web_vital(state: &Arc<AppState>, vital: &BufferedWebVital) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:web_vitals:{}", vital.project_id));
    let serialized = serde_json::to_string(vital)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_scroll_depth(state: &Arc<AppState>, scroll: &BufferedScrollDepth) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:scroll_depths:{}", scroll.project_id));
    let serialized = serde_json::to_string(scroll)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_search_query(state: &Arc<AppState>, search: &BufferedSearchQuery) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:search_queries:{}", search.project_id));
    let serialized = serde_json::to_string(search)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_outlink(state: &Arc<AppState>, outlink: &BufferedOutlink) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:outlinks:{}", outlink.project_id));
    let serialized = serde_json::to_string(outlink)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_js_error(state: &Arc<AppState>, js_error: &BufferedJsError) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:js_errors:{}", js_error.project_id));
    let serialized = serde_json::to_string(js_error)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn push_click_event(state: &Arc<AppState>, click: &BufferedClickEvent) -> Result<(), anyhow::Error> {
    let key = state.redis_key(&format!("buffer:click_events:{}", click.project_id));
    let serialized = serde_json::to_string(click)?;
    let mut redis = state.redis.clone();
    let _: () = redis.rpush(&key, &serialized).await?;
    Ok(())
}

pub async fn update_realtime(state: &Arc<AppState>, project_id: uuid::Uuid, visitor_id: &str) -> Result<(), anyhow::Error> {
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
        flush_pageviews(state, key, batch_size).await?;
    }

    // Get all event buffer keys
    let ev_pattern = state.redis_key("buffer:events:*");
    let ev_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&ev_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &ev_keys {
        flush_events(state, key, batch_size).await?;
    }

    // Get all web_vitals buffer keys
    let wv_pattern = state.redis_key("buffer:web_vitals:*");
    let wv_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&wv_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &wv_keys {
        flush_web_vitals(state, key, batch_size).await?;
    }

    // Get all scroll_depths buffer keys
    let sd_pattern = state.redis_key("buffer:scroll_depths:*");
    let sd_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&sd_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &sd_keys {
        flush_scroll_depths(state, key, batch_size).await?;
    }

    // Get all search_queries buffer keys
    let sq_pattern = state.redis_key("buffer:search_queries:*");
    let sq_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&sq_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &sq_keys {
        flush_search_queries(state, key, batch_size).await?;
    }

    // Get all outlinks buffer keys
    let ol_pattern = state.redis_key("buffer:outlinks:*");
    let ol_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&ol_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &ol_keys {
        flush_outlinks(state, key, batch_size).await?;
    }

    // Get all js_errors buffer keys
    let je_pattern = state.redis_key("buffer:js_errors:*");
    let je_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&je_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &je_keys {
        flush_js_errors(state, key, batch_size).await?;
    }

    // Get all click_events buffer keys
    let ce_pattern = state.redis_key("buffer:click_events:*");
    let ce_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&ce_pattern)
        .query_async(&mut redis)
        .await
        .unwrap_or_default();

    for key in &ce_keys {
        flush_click_events(state, key, batch_size).await?;
    }

    Ok(())
}

async fn flush_pageviews(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    // Atomically get and remove items
    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let pageviews: Vec<BufferedPageview> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if pageviews.is_empty() {
        return Ok(());
    }

    info!("Flushing {} pageviews to PostgreSQL", pageviews.len());
    batch_insert_pageviews(&state.db, &pageviews).await?;

    Ok(())
}

async fn flush_events(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let events: Vec<BufferedEvent> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if events.is_empty() {
        return Ok(());
    }

    info!("Flushing {} events to PostgreSQL", events.len());
    batch_insert_events(&state.db, &events).await?;

    Ok(())
}

async fn flush_web_vitals(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let vitals: Vec<BufferedWebVital> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if vitals.is_empty() {
        return Ok(());
    }

    info!("Flushing {} web vitals to PostgreSQL", vitals.len());
    batch_insert_web_vitals(&state.db, &vitals).await?;

    Ok(())
}

async fn flush_scroll_depths(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let scrolls: Vec<BufferedScrollDepth> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if scrolls.is_empty() {
        return Ok(());
    }

    info!("Flushing {} scroll depths to PostgreSQL", scrolls.len());
    batch_insert_scroll_depths(&state.db, &scrolls).await?;

    Ok(())
}

async fn flush_search_queries(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let searches: Vec<BufferedSearchQuery> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if searches.is_empty() {
        return Ok(());
    }

    info!("Flushing {} search queries to PostgreSQL", searches.len());
    batch_insert_search_queries(&state.db, &searches).await?;

    Ok(())
}

async fn flush_outlinks(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let outlinks: Vec<BufferedOutlink> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if outlinks.is_empty() {
        return Ok(());
    }

    info!("Flushing {} outlinks to PostgreSQL", outlinks.len());
    batch_insert_outlinks(&state.db, &outlinks).await?;

    Ok(())
}

async fn flush_js_errors(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let errors: Vec<BufferedJsError> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if errors.is_empty() {
        return Ok(());
    }

    info!("Flushing {} JS errors to PostgreSQL", errors.len());
    batch_insert_js_errors(&state.db, &errors).await?;

    Ok(())
}

async fn flush_click_events(state: &Arc<AppState>, key: &str, batch_size: usize) -> Result<(), anyhow::Error> {
    let mut redis = state.redis.clone();

    let items: Vec<String> = redis.lpop(key, std::num::NonZero::new(batch_size)).await.unwrap_or_default();

    if items.is_empty() {
        return Ok(());
    }

    let clicks: Vec<BufferedClickEvent> = items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    if clicks.is_empty() {
        return Ok(());
    }

    info!("Flushing {} click events to PostgreSQL", clicks.len());
    batch_insert_click_events(&state.db, &clicks).await?;

    Ok(())
}

async fn batch_insert_pageviews(db: &PgPool, pageviews: &[BufferedPageview]) -> Result<(), anyhow::Error> {
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
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3, params_idx + 4,
            params_idx + 5, params_idx + 6, params_idx + 7, params_idx + 8, params_idx + 9,
            params_idx + 10, params_idx + 11, params_idx + 12, params_idx + 13, params_idx + 14,
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
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3,
            params_idx + 4, params_idx + 5, params_idx + 6, params_idx + 7, params_idx + 8,
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

async fn batch_insert_web_vitals(db: &PgPool, vitals: &[BufferedWebVital]) -> Result<(), anyhow::Error> {
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
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3,
            params_idx + 4, params_idx + 5, params_idx + 6, params_idx + 7,
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

async fn batch_insert_scroll_depths(db: &PgPool, scrolls: &[BufferedScrollDepth]) -> Result<(), anyhow::Error> {
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
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3,
            params_idx + 4, params_idx + 5,
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

async fn batch_insert_search_queries(db: &PgPool, searches: &[BufferedSearchQuery]) -> Result<(), anyhow::Error> {
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
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3,
            params_idx + 4, params_idx + 5, params_idx + 6,
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

async fn batch_insert_outlinks(db: &PgPool, outlinks: &[BufferedOutlink]) -> Result<(), anyhow::Error> {
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
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3,
            params_idx + 4, params_idx + 5, params_idx + 6,
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

async fn batch_insert_js_errors(db: &PgPool, errors: &[BufferedJsError]) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO js_errors (project_id, visitor_id, session_id, message, stack, filename, lineno, colno, path, browser, os, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in errors.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3,
            params_idx + 4, params_idx + 5, params_idx + 6, params_idx + 7,
            params_idx + 8, params_idx + 9, params_idx + 10, params_idx + 11,
        ));
        params_idx += 12;
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
            .bind(e.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_click_events(db: &PgPool, clicks: &[BufferedClickEvent]) -> Result<(), anyhow::Error> {
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
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3,
            params_idx + 4, params_idx + 5, params_idx + 6, params_idx + 7,
            params_idx + 8, params_idx + 9,
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
