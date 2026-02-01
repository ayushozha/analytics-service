use std::sync::Arc;
use std::time::Duration;

use redis::AsyncCommands;
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info};

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

async fn batch_insert_pageviews(db: &PgPool, pageviews: &[BufferedPageview]) -> Result<(), anyhow::Error> {
    // Build a bulk insert
    let mut query = String::from(
        "INSERT INTO pageviews (project_id, session_id, visitor_id, path, title, referrer, referrer_domain, query_params, duration_ms, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in pageviews.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            params_idx, params_idx + 1, params_idx + 2, params_idx + 3, params_idx + 4,
            params_idx + 5, params_idx + 6, params_idx + 7, params_idx + 8, params_idx + 9,
        ));
        params_idx += 10;
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
            .bind(pv.created_at);
    }

    q.execute(db).await?;
    Ok(())
}

async fn batch_insert_events(db: &PgPool, events: &[BufferedEvent]) -> Result<(), anyhow::Error> {
    let mut query = String::from(
        "INSERT INTO events (project_id, session_id, visitor_id, event_name, event_data, path, created_at) VALUES "
    );

    let mut params_idx = 1u32;
    for (i, _) in events.iter().enumerate() {
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
    for ev in events {
        q = q
            .bind(ev.project_id)
            .bind(ev.session_id)
            .bind(&ev.visitor_id)
            .bind(&ev.event_name)
            .bind(&ev.event_data)
            .bind(&ev.path)
            .bind(ev.created_at);
    }

    q.execute(db).await?;
    Ok(())
}
