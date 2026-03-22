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
        // Core analytics
        .route("/api/v1/stats", get(routes::query::get_stats))
        .route("/api/v1/stats/timeseries", get(routes::query::get_timeseries))
        .route("/api/v1/pages", get(routes::query::get_pages))
        .route("/api/v1/referrers", get(routes::query::get_referrers))
        .route("/api/v1/events", get(routes::query::get_events))
        .route("/api/v1/devices", get(routes::query::get_devices))
        .route("/api/v1/geo", get(routes::query::get_geo))
        .route("/api/v1/realtime", get(routes::query::get_realtime))
        // Funnels
        .route("/api/v1/funnels", get(routes::features::list_funnels).post(routes::features::create_funnel))
        .route("/api/v1/funnels/{id}", get(routes::features::get_funnel).put(routes::features::update_funnel).delete(routes::features::delete_funnel))
        .route("/api/v1/funnels/{id}/analyze", get(routes::features::analyze_funnel))
        // Goals
        .route("/api/v1/goals", get(routes::features::list_goals).post(routes::features::create_goal))
        .route("/api/v1/goals/{id}", get(routes::features::get_goal).put(routes::features::update_goal).delete(routes::features::delete_goal))
        .route("/api/v1/goals/{id}/stats", get(routes::features::get_goal_stats))
        // Retention & Cohorts & Paths
        .route("/api/v1/retention", get(routes::features::get_retention))
        .route("/api/v1/cohorts", get(routes::features::get_cohorts))
        .route("/api/v1/paths", get(routes::features::get_paths))
        // Campaigns
        .route("/api/v1/campaigns", get(routes::features::get_campaigns))
        .route("/api/v1/campaigns/sources", get(routes::features::get_sources))
        .route("/api/v1/campaigns/mediums", get(routes::features::get_mediums))
        .route("/api/v1/campaigns/timeseries", get(routes::features::get_campaign_timeseries))
        // CSV Exports
        .route("/api/v1/exports/{report_type}", get(routes::features_ext::export_csv))
        // Shared Dashboards
        .route("/api/v1/sharing", get(routes::features_ext::list_shared_dashboards).post(routes::features_ext::create_shared_dashboard))
        .route("/api/v1/sharing/{id}", axum::routing::delete(routes::features_ext::delete_shared_dashboard))
        // Alerts
        .route("/api/v1/alerts", get(routes::features_ext::list_alerts).post(routes::features_ext::create_alert))
        .route("/api/v1/alerts/{id}", axum::routing::put(routes::features_ext::update_alert).delete(routes::features_ext::delete_alert))
        .route("/api/v1/alerts/{id}/toggle", post(routes::features_ext::toggle_alert))
        // Experiments
        .route("/api/v1/experiments", get(routes::features_ext::list_experiments).post(routes::features_ext::create_experiment))
        .route("/api/v1/experiments/{id}", get(routes::features_ext::get_experiment).delete(routes::features_ext::delete_experiment))
        .route("/api/v1/experiments/{id}/status", axum::routing::put(routes::features_ext::update_experiment_status))
        .route("/api/v1/experiments/{id}/results", get(routes::features_ext::get_experiment_results))
        // experiment assignment handled via POST /api/collect with type=experiment_assign
        // Surveys
        .route("/api/v1/surveys", get(routes::features_ext::list_surveys).post(routes::features_ext::create_survey))
        .route("/api/v1/surveys/active", get(routes::features_ext::get_active_surveys))
        .route("/api/v1/surveys/{id}", get(routes::features_ext::get_survey).put(routes::features_ext::update_survey).delete(routes::features_ext::delete_survey))
        .route("/api/v1/surveys/{id}/status", axum::routing::put(routes::features_ext::update_survey_status))
        .route("/api/v1/surveys/{id}/responses", get(routes::features_ext::get_survey_responses))
        .route("/api/v1/surveys/{id}/stats", get(routes::features_ext::get_survey_stats))
        // Web Vitals
        .route("/api/v1/webvitals", get(routes::features_ext::get_vitals_summary))
        .route("/api/v1/webvitals/pages", get(routes::features_ext::get_vitals_by_page))
        .route("/api/v1/webvitals/timeseries", get(routes::features_ext::get_vitals_timeseries))
        // Error Tracking
        .route("/api/v1/errors", get(routes::features_ext::get_error_groups))
        .route("/api/v1/errors/detail", get(routes::features_ext::get_error_detail))
        .route("/api/v1/errors/timeseries", get(routes::features_ext::get_error_timeseries))
        .route("/api/v1/errors/stats", get(routes::features_ext::get_error_stats))
        // Heatmaps
        .route("/api/v1/heatmaps", get(routes::features_ext::get_click_heatmap))
        .route("/api/v1/heatmaps/stats", get(routes::features_ext::get_click_stats))
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
        // Module management
        .route(
            "/api/admin/projects/{id}/modules",
            get(routes::admin::list_modules).put(routes::admin::update_modules),
        )
        .route(
            "/api/admin/projects/{id}/modules/{module_name}/enable",
            post(routes::admin::enable_module),
        )
        .route(
            "/api/admin/projects/{id}/modules/{module_name}/disable",
            post(routes::admin::disable_module),
        )
        .route(
            "/api/admin/projects/{id}/modules/{module_name}/access",
            axum::routing::put(routes::admin::update_module_access),
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
        .route("/dashboard/api/realtime", get(routes::dashboard::htmx_realtime))
        // Visitor pages
        .route("/dashboard/visitors", get(routes::dashboard::visitors_page))
        .route("/dashboard/visitors/{visitor_id}", get(routes::dashboard::visitor_detail_page))
        .route("/dashboard/pricing", get(routes::dashboard::pricing_page))
        // Visitor HTMX API
        .route("/dashboard/api/visitors/live-count", get(routes::dashboard::htmx_visitors_live_count))
        .route("/dashboard/api/visitors/activity-feed", get(routes::dashboard::htmx_visitors_activity_feed))
        .route("/dashboard/api/visitors/table", get(routes::dashboard::htmx_visitors_table))
        .route("/dashboard/api/visitor/{visitor_id}/summary", get(routes::dashboard::htmx_visitor_summary))
        .route("/dashboard/api/visitor/{visitor_id}/sessions", get(routes::dashboard::htmx_visitor_sessions))
        .route("/dashboard/api/visitor/{visitor_id}/session/{session_id}/detail", get(routes::dashboard::htmx_visitor_session_detail))
        .route("/dashboard/api/visitor/{visitor_id}/activity-chart", get(routes::dashboard::htmx_visitor_activity_chart))
        .route("/dashboard/api/visitor/{visitor_id}/events-breakdown", get(routes::dashboard::htmx_visitor_events_breakdown))
        // Pricing HTMX API
        .route("/dashboard/api/pricing/stats", get(routes::dashboard::htmx_pricing_stats))
        .route("/dashboard/api/pricing/timeseries", get(routes::dashboard::htmx_pricing_timeseries))
        .route("/dashboard/api/pricing/frequency", get(routes::dashboard::htmx_pricing_frequency))
        .route("/dashboard/api/pricing/referrers", get(routes::dashboard::htmx_pricing_referrers))
        .route("/dashboard/api/pricing/heatmap", get(routes::dashboard::htmx_pricing_heatmap))
        .route("/dashboard/api/pricing/funnel", get(routes::dashboard::htmx_pricing_funnel));

    // Public routes (no auth)
    let public_routes = Router::new()
        .route("/", get(routes::docs::serve_home))
        .route("/health", get(routes::health::health_check))
        .route("/api/script.js", get(routes::script::serve_script))
        .route("/api/docs", get(routes::docs::serve_docs))
        .route("/docs", get(routes::docs::redirect_docs));

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
