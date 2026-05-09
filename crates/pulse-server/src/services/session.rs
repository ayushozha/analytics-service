use chrono::Utc;
use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::session::SessionCache;
use crate::services::geo::GeoResult;
use crate::services::ua::ParsedUA;
use crate::state::SharedState;

const SESSION_TTL: u64 = 1800; // 30 minutes

pub async fn resolve_session(
    state: &SharedState,
    project_id: Uuid,
    visitor_id: &str,
    ua: &ParsedUA,
    geo: &GeoResult,
    screen: Option<&str>,
    language: Option<&str>,
    hostname: Option<&str>,
    entry_page: Option<&str>,
) -> Result<Uuid, anyhow::Error> {
    let cache_key = state.redis_key(&format!("session:{project_id}:{visitor_id}"));
    let mut redis = state.redis.clone();

    // Check if session exists in Redis
    let cached: Option<String> = redis.get(&cache_key).await.unwrap_or(None);

    if let Some(cached) = cached {
        if let Ok(session_cache) = serde_json::from_str::<SessionCache>(&cached) {
            // Extend session TTL
            let _: () = redis
                .expire(&cache_key, SESSION_TTL as i64)
                .await
                .unwrap_or(());
            return Ok(session_cache.session_id);
        }
    }

    // Create new session in PostgreSQL
    let session_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO sessions (id, project_id, visitor_id, hostname, browser, os, device, screen, language, country, region, city, first_at, last_at, is_bounce, entry_page, exit_page, pageview_count, event_count, duration_ms) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, true, $15, $15, 0, 0, 0)"
    )
    .bind(session_id)
    .bind(project_id)
    .bind(visitor_id)
    .bind(hostname)
    .bind(ua.browser.as_deref())
    .bind(ua.os.as_deref())
    .bind(ua.device.as_deref())
    .bind(screen)
    .bind(language)
    .bind(geo.country.as_deref())
    .bind(geo.region.as_deref())
    .bind(geo.city.as_deref())
    .bind(now)
    .bind(now)
    .bind(entry_page)
    .execute(&state.db)
    .await?;

    // Cache in Redis
    let cache = SessionCache {
        session_id,
        pageview_count: 0,
        event_count: 0,
    };
    let serialized = serde_json::to_string(&cache)?;
    let _: () = redis
        .set_ex(&cache_key, &serialized, SESSION_TTL)
        .await
        .unwrap_or(());

    Ok(session_id)
}

pub async fn update_session_counts(
    db: &PgPool,
    session_id: Uuid,
    is_pageview: bool,
    exit_page: Option<&str>,
) -> Result<(), sqlx::Error> {
    if is_pageview {
        sqlx::query(
            "UPDATE sessions SET last_at = NOW(), pageview_count = pageview_count + 1, is_bounce = (pageview_count + 1) <= 1, exit_page = COALESCE($2, exit_page), duration_ms = EXTRACT(EPOCH FROM (NOW() - first_at))::bigint * 1000 WHERE id = $1"
        )
        .bind(session_id)
        .bind(exit_page)
        .execute(db)
        .await?;
    } else {
        sqlx::query(
            "UPDATE sessions SET last_at = NOW(), event_count = event_count + 1, duration_ms = EXTRACT(EPOCH FROM (NOW() - first_at))::bigint * 1000 WHERE id = $1"
        )
        .bind(session_id)
        .execute(db)
        .await?;
    }
    Ok(())
}
