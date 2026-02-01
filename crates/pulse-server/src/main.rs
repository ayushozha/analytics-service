mod config;
mod error;
mod middleware;
mod models;
mod routes;
mod services;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::middleware as axum_mw;
use axum::routing::{get, post};
use axum::{Extension, Router};
use maxminddb::Reader;
use redis::Client as RedisClient;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::middleware::cors::build_cors_layer;
use crate::services::umami_client::UmamiClient;
use crate::state::{AppState, SharedState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env();
    info!("Starting Pulse Analytics on port {}", config.port);

    // Database connection pool
    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;

    info!("Connected to PostgreSQL");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&db).await?;
    info!("Migrations applied");

    // Redis connection
    let redis_client = RedisClient::open(config.redis_url.as_str())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    info!("Connected to Redis");

    // GeoIP database (optional)
    let geoip = config.geoip_db_path.as_ref().and_then(|path| {
        match Reader::open_readfile(path) {
            Ok(reader) => {
                info!("Loaded GeoIP database from {path}");
                Some(reader)
            }
            Err(e) => {
                tracing::warn!("Failed to load GeoIP database: {e}");
                None
            }
        }
    });

    // Umami client (optional — only if URL + credentials are configured)
    let umami = match (&config.umami_url, &config.umami_user, &config.umami_pass) {
        (Some(url), Some(user), Some(pass)) => {
            info!("Umami proxy enabled: {url}");
            Some(UmamiClient::new(url, user, pass))
        }
        _ => {
            info!("Umami proxy disabled (UMAMI_URL/USER/PASS not configured)");
            None
        }
    };

    let state: SharedState = Arc::new(AppState {
        config: config.clone(),
        db,
        redis,
        geoip,
        umami,
    });

    // Start background tasks
    services::ingestion::start_flush_task(state.clone());
    info!("Buffer flush task started");

    services::aggregation::start_rollup_task(state.clone());
    info!("Daily rollup task started");

    services::partition::start_partition_task(state.clone());
    info!("Partition management task started");

    services::retention::start_retention_task(state.clone());
    info!("Data retention task started");

    services::webhook::start_webhook_tasks(state.clone());
    info!("Webhook alert tasks started");

    // Build routes
    let cors = build_cors_layer(&config);

    // Public ingestion routes (API key auth)
    let ingest_routes = Router::new()
        .route("/api/collect", post(routes::ingest::collect))
        .layer(axum_mw::from_fn(middleware::rate_limit::rate_limit_middleware))
        .layer(axum_mw::from_fn(middleware::auth::auth_middleware));

    // Query routes (API key auth with query scope)
    let query_routes = Router::new()
        .route("/api/v1/stats", get(routes::query::get_stats))
        .route("/api/v1/stats/timeseries", get(routes::query::get_timeseries))
        .route("/api/v1/pages", get(routes::query::get_pages))
        .route("/api/v1/referrers", get(routes::query::get_referrers))
        .route("/api/v1/events", get(routes::query::get_events))
        .route("/api/v1/devices", get(routes::query::get_devices))
        .route("/api/v1/geo", get(routes::query::get_geo))
        .route("/api/v1/realtime", get(routes::query::get_realtime))
        .layer(axum_mw::from_fn(middleware::auth::auth_middleware));

    // Admin routes (admin token auth)
    let admin_routes = Router::new()
        .route("/api/admin/projects", post(routes::admin::create_project))
        .route("/api/admin/projects", get(routes::admin::list_projects))
        .route("/api/admin/projects/{id}", get(routes::admin::get_project))
        .route(
            "/api/admin/projects/{id}/keys",
            post(routes::admin::create_api_key),
        )
        .route(
            "/api/admin/projects/{id}/keys",
            get(routes::admin::list_api_keys),
        )
        .route(
            "/api/admin/projects/{id}/keys/{key_id}",
            axum::routing::delete(routes::admin::revoke_api_key),
        )
        .route(
            "/api/admin/projects/{id}/webhooks",
            post(routes::admin::create_webhook).get(routes::admin::list_webhooks),
        )
        .route(
            "/api/admin/projects/{id}/webhooks/{webhook_id}",
            axum::routing::put(routes::admin::update_webhook)
                .delete(routes::admin::delete_webhook),
        )
        .route(
            "/api/admin/projects/{id}/webhooks/{webhook_id}/test",
            post(routes::admin::test_webhook),
        )
        .layer(axum_mw::from_fn(middleware::auth::admin_auth_middleware));

    // Dashboard routes (cookie-based session auth)
    let dashboard_routes = Router::new()
        .route("/dashboard", get(routes::dashboard::dashboard_index))
        .route("/dashboard/login", get(routes::dashboard::login_page).post(routes::dashboard::login_submit))
        .route("/dashboard/logout", post(routes::dashboard::logout))
        .route("/dashboard/overview", get(routes::dashboard::overview_page))
        .route("/dashboard/pages", get(routes::dashboard::pages_page))
        .route("/dashboard/referrers", get(routes::dashboard::referrers_page))
        .route("/dashboard/events", get(routes::dashboard::events_page))
        .route("/dashboard/devices", get(routes::dashboard::devices_page))
        .route("/dashboard/geo", get(routes::dashboard::geo_page))
        .route("/dashboard/realtime", get(routes::dashboard::realtime_page))
        .route("/dashboard/api/stats", get(routes::dashboard::htmx_stats_cards))
        .route("/dashboard/api/timeseries", get(routes::dashboard::htmx_timeseries))
        .route("/dashboard/api/pages", get(routes::dashboard::htmx_pages_table))
        .route("/dashboard/api/referrers", get(routes::dashboard::htmx_referrers_table))
        .route("/dashboard/api/events", get(routes::dashboard::htmx_events_table))
        .route("/dashboard/api/devices", get(routes::dashboard::htmx_devices_table))
        .route("/dashboard/api/geo", get(routes::dashboard::htmx_geo_table))
        .route("/dashboard/api/realtime", get(routes::dashboard::htmx_realtime));

    // Public routes (no auth)
    let public_routes = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/api/script.js", get(routes::script::serve_script));

    let app = Router::new()
        .merge(ingest_routes)
        .merge(query_routes)
        .merge(admin_routes)
        .merge(dashboard_routes)
        .merge(public_routes)
        .layer(cors)
        .layer(Extension(state));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
