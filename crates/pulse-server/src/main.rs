#![allow(clippy::too_many_arguments, clippy::type_complexity)]

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

    // Install the BI connection-string encryption key (if configured) before any
    // BI connection is read or written.
    services::bi::init_connection_kms(config.bi_connection_kms_key);
    if config.bi_connection_kms_key.is_some() {
        info!("BI connection-string encryption enabled");
    }

    // Database connection pool
    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;

    info!("Connected to PostgreSQL");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&db).await?;
    info!("Migrations applied");

    // Encrypt any legacy plaintext BI connection strings now that a key is configured.
    match services::bi::reencrypt_plaintext_connections(&db).await {
        Ok(0) => {}
        Ok(n) => info!("Encrypted {n} legacy BI connection string(s) at rest"),
        Err(e) => tracing::warn!("Failed to re-encrypt legacy BI connection strings: {e}"),
    }

    // Redis connection
    let redis_client = RedisClient::open(config.redis_url.as_str())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    info!("Connected to Redis");

    // GeoIP database (optional)
    let geoip = config
        .geoip_db_path
        .as_ref()
        .and_then(|path| match Reader::open_readfile(path) {
            Ok(reader) => {
                info!("Loaded GeoIP database from {path}");
                Some(reader)
            }
            Err(e) => {
                tracing::warn!("Failed to load GeoIP database: {e}");
                None
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

    services::destinations::start_destination_delivery_task(state.clone());
    info!("Destination delivery task started");

    services::email_reports::start_email_report_task(state.clone());
    info!("Email report scheduler started");

    // Build routes
    let cors = build_cors_layer(&config);

    // Public ingestion routes (API key auth)
    let ingest_routes = Router::new()
        .route("/api/collect", post(routes::ingest::collect))
        .route("/api/batch", post(routes::ingest::collect_batch))
        .layer(axum_mw::from_fn(
            middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(axum_mw::from_fn(middleware::auth::auth_middleware));

    // Source webhook ingestion routes (source token auth)
    let source_ingest_routes = Router::new()
        .route(
            "/api/source/{id}/collect",
            post(routes::features_ext::ingest_source_webhook),
        )
        .route(
            "/api/sources/{id}/collect",
            post(routes::features_ext::ingest_source_webhook),
        )
        .layer(axum_mw::from_fn(
            middleware::rate_limit::rate_limit_middleware,
        ));

    // Query routes (API key auth with query scope)
    let query_routes = Router::new()
        // Core analytics
        .route("/api/v1/stats", get(routes::query::get_stats))
        .route(
            "/api/v1/stats/timeseries",
            get(routes::query::get_timeseries),
        )
        .route("/api/v1/pages", get(routes::query::get_pages))
        .route("/api/v1/referrers", get(routes::query::get_referrers))
        .route("/api/v1/events", get(routes::query::get_events))
        .route("/api/v1/devices", get(routes::query::get_devices))
        .route("/api/v1/geo", get(routes::query::get_geo))
        .route("/api/v1/realtime", get(routes::query::get_realtime))
        // Funnels
        .route(
            "/api/v1/funnels",
            get(routes::features::list_funnels).post(routes::features::create_funnel),
        )
        .route(
            "/api/v1/funnels/{id}",
            get(routes::features::get_funnel)
                .put(routes::features::update_funnel)
                .delete(routes::features::delete_funnel),
        )
        .route(
            "/api/v1/funnels/{id}/analyze",
            get(routes::features::analyze_funnel),
        )
        // Goals
        .route(
            "/api/v1/goals",
            get(routes::features::list_goals).post(routes::features::create_goal),
        )
        .route(
            "/api/v1/goals/{id}",
            get(routes::features::get_goal)
                .put(routes::features::update_goal)
                .delete(routes::features::delete_goal),
        )
        .route(
            "/api/v1/goals/{id}/stats",
            get(routes::features::get_goal_stats),
        )
        // Retention & Cohorts & Paths
        .route("/api/v1/retention", get(routes::features::get_retention))
        .route("/api/v1/cohorts", get(routes::features::get_cohorts))
        .route("/api/v1/paths", get(routes::features::get_paths))
        // Campaigns
        .route("/api/v1/campaigns", get(routes::features::get_campaigns))
        .route(
            "/api/v1/campaigns/sources",
            get(routes::features::get_sources),
        )
        .route(
            "/api/v1/campaigns/mediums",
            get(routes::features::get_mediums),
        )
        .route(
            "/api/v1/campaigns/timeseries",
            get(routes::features::get_campaign_timeseries),
        )
        .route(
            "/api/v1/marketing/channels",
            get(routes::features_ext::get_marketing_channels),
        )
        .route(
            "/api/v1/marketing/attribution",
            get(routes::features_ext::get_marketing_attribution),
        )
        .route(
            "/api/v1/marketing/ecommerce",
            get(routes::features_ext::get_marketing_ecommerce),
        )
        .route(
            "/api/v1/marketing/ai-referrers",
            get(routes::features_ext::get_marketing_ai_referrers),
        )
        .route(
            "/api/v1/marketing/imports",
            get(routes::features_ext::list_marketing_imports)
                .post(routes::features_ext::create_marketing_import),
        )
        .route(
            "/api/v1/marketing/imports/summary",
            get(routes::features_ext::get_marketing_import_summary),
        )
        .route(
            "/api/v1/marketing/imports/{id}/rows",
            get(routes::features_ext::get_marketing_import_rows),
        )
        .route(
            "/api/v1/marketing/imports/{id}",
            axum::routing::delete(routes::features_ext::delete_marketing_import),
        )
        // CSV Exports
        .route(
            "/api/v1/exports/{report_type}",
            get(routes::features_ext::export_csv),
        )
        // Product Analytics Workspace
        .route(
            "/api/v1/dashboards",
            get(routes::features_ext::list_custom_dashboards)
                .post(routes::features_ext::create_custom_dashboard),
        )
        .route(
            "/api/v1/dashboards/{id}",
            get(routes::features_ext::get_custom_dashboard)
                .put(routes::features_ext::update_custom_dashboard)
                .delete(routes::features_ext::delete_custom_dashboard),
        )
        .route(
            "/api/v1/reports",
            get(routes::features_ext::list_saved_reports)
                .post(routes::features_ext::create_saved_report),
        )
        .route(
            "/api/v1/reports/{id}",
            get(routes::features_ext::get_saved_report)
                .put(routes::features_ext::update_saved_report)
                .delete(routes::features_ext::delete_saved_report),
        )
        .route(
            "/api/v1/reports/{id}/run",
            post(routes::features_ext::run_saved_report),
        )
        .route(
            "/api/v1/query-explorer",
            post(routes::features_ext::run_query_explorer),
        )
        .route(
            "/api/v1/query-explorer/history",
            get(routes::features_ext::list_query_explorer_runs),
        )
        .route(
            "/api/v1/product/stickiness",
            get(routes::features_ext::get_product_stickiness),
        )
        .route(
            "/api/v1/product/lifecycle",
            get(routes::features_ext::get_product_lifecycle),
        )
        .route(
            "/api/v1/product/activation",
            post(routes::features_ext::get_product_activation),
        )
        .route(
            "/api/v1/product/impact",
            post(routes::features_ext::get_product_impact),
        )
        // BI Layer
        .route(
            "/api/v1/bi/metrics",
            get(routes::features_ext::list_bi_metrics).post(routes::features_ext::create_bi_metric),
        )
        .route(
            "/api/v1/bi/metrics/{id}",
            get(routes::features_ext::get_bi_metric)
                .put(routes::features_ext::update_bi_metric)
                .delete(routes::features_ext::delete_bi_metric),
        )
        .route(
            "/api/v1/bi/row-policies",
            get(routes::features_ext::list_bi_row_policies)
                .post(routes::features_ext::create_bi_row_policy),
        )
        .route(
            "/api/v1/bi/row-policies/{id}",
            axum::routing::put(routes::features_ext::update_bi_row_policy)
                .delete(routes::features_ext::delete_bi_row_policy),
        )
        .route(
            "/api/v1/bi/connections",
            get(routes::features_ext::list_bi_database_connections)
                .post(routes::features_ext::create_bi_database_connection),
        )
        .route(
            "/api/v1/bi/connections/{id}",
            get(routes::features_ext::get_bi_database_connection)
                .put(routes::features_ext::update_bi_database_connection)
                .delete(routes::features_ext::delete_bi_database_connection),
        )
        .route(
            "/api/v1/bi/connections/{id}/test",
            post(routes::features_ext::test_bi_database_connection),
        )
        .route(
            "/api/v1/bi/connections/{id}/query",
            post(routes::features_ext::run_bi_external_sql),
        )
        .route(
            "/api/v1/bi/embeds",
            get(routes::features_ext::list_bi_embeds).post(routes::features_ext::create_bi_embed),
        )
        .route(
            "/api/v1/bi/embeds/{id}",
            get(routes::features_ext::get_bi_embed)
                .put(routes::features_ext::update_bi_embed)
                .delete(routes::features_ext::delete_bi_embed),
        )
        .route(
            "/api/v1/bi/embeds/{id}/rotate-token",
            post(routes::features_ext::rotate_bi_embed_token),
        )
        .route("/api/v1/bi/sql", post(routes::features_ext::run_bi_sql))
        .route(
            "/api/v1/bi/sql-queries",
            get(routes::features_ext::list_bi_saved_queries)
                .post(routes::features_ext::create_bi_saved_query),
        )
        .route(
            "/api/v1/bi/sql-queries/{id}",
            get(routes::features_ext::get_bi_saved_query)
                .put(routes::features_ext::update_bi_saved_query)
                .delete(routes::features_ext::delete_bi_saved_query),
        )
        .route(
            "/api/v1/bi/sql-queries/{id}/run",
            post(routes::features_ext::run_bi_saved_query),
        )
        .route(
            "/api/v1/bi/visual-query",
            post(routes::features_ext::run_bi_visual_query),
        )
        .route(
            "/api/v1/bi/drill-through",
            post(routes::features_ext::run_bi_drill_through),
        )
        .route(
            "/api/v1/bi/query-runs",
            get(routes::features_ext::list_bi_query_runs),
        )
        .route(
            "/api/v1/bi/csv-uploads",
            get(routes::features_ext::list_csv_uploads)
                .post(routes::features_ext::create_csv_upload),
        )
        .route(
            "/api/v1/bi/csv-uploads/{id}/rows",
            get(routes::features_ext::get_csv_upload_rows),
        )
        .route(
            "/api/v1/bi/csv-uploads/{id}",
            axum::routing::delete(routes::features_ext::delete_csv_upload),
        )
        // Integrations Marketplace
        .route(
            "/api/v1/integrations",
            get(routes::features_ext::list_integrations),
        )
        .route(
            "/api/v1/integrations/{key}",
            get(routes::features_ext::get_integration),
        )
        // Sources / CDP Ingestion
        .route(
            "/api/v1/sources",
            get(routes::features_ext::list_event_sources)
                .post(routes::features_ext::create_event_source),
        )
        .route(
            "/api/v1/sources/{id}",
            get(routes::features_ext::get_event_source)
                .put(routes::features_ext::update_event_source)
                .delete(routes::features_ext::delete_event_source),
        )
        .route(
            "/api/v1/sources/{id}/ingestions",
            get(routes::features_ext::list_source_ingestions),
        )
        // Destinations / Event Routing
        .route(
            "/api/v1/destinations",
            get(routes::features_ext::list_destinations)
                .post(routes::features_ext::create_destination),
        )
        .route(
            "/api/v1/destinations/{id}",
            get(routes::features_ext::get_destination)
                .put(routes::features_ext::update_destination)
                .delete(routes::features_ext::delete_destination),
        )
        .route(
            "/api/v1/destination-deliveries",
            get(routes::features_ext::list_destination_deliveries),
        )
        .route(
            "/api/v1/destination-deliveries/{id}/retry",
            post(routes::features_ext::retry_destination_delivery),
        )
        .route(
            "/api/v1/destination-health",
            get(routes::features_ext::get_destination_health),
        )
        // AI Analytics
        .route("/api/v1/ai/query", post(routes::features_ext::ask_ai_query))
        .route(
            "/api/v1/ai/insights",
            get(routes::features_ext::get_ai_insights),
        )
        .route(
            "/api/v1/ai/history",
            get(routes::features_ext::list_ai_query_history),
        )
        .route(
            "/api/v1/ai/llm/traces",
            get(routes::features_ext::list_llm_traces).post(routes::features_ext::record_llm_trace),
        )
        .route(
            "/api/v1/ai/llm/traces/{id}",
            get(routes::features_ext::get_llm_trace),
        )
        .route(
            "/api/v1/ai/llm/generations",
            get(routes::features_ext::list_llm_generations)
                .post(routes::features_ext::record_llm_generation),
        )
        .route(
            "/api/v1/ai/llm/evaluations",
            get(routes::features_ext::list_llm_evaluations)
                .post(routes::features_ext::record_llm_evaluation),
        )
        .route(
            "/api/v1/ai/llm/stats",
            get(routes::features_ext::get_llm_stats),
        )
        // Shared Dashboards
        .route(
            "/api/v1/sharing",
            get(routes::features_ext::list_shared_dashboards)
                .post(routes::features_ext::create_shared_dashboard),
        )
        .route(
            "/api/v1/sharing/{id}",
            axum::routing::delete(routes::features_ext::delete_shared_dashboard),
        )
        // Email Reports
        .route(
            "/api/v1/email-reports",
            get(routes::features_ext::list_email_reports)
                .post(routes::features_ext::create_email_report),
        )
        .route(
            "/api/v1/email-reports/{id}",
            axum::routing::put(routes::features_ext::update_email_report)
                .delete(routes::features_ext::delete_email_report),
        )
        .route(
            "/api/v1/email-reports/{id}/test",
            post(routes::features_ext::send_test_email_report),
        )
        // Identity
        .route(
            "/api/v1/identity/users",
            get(routes::features_ext::list_user_profiles),
        )
        .route(
            "/api/v1/identity/users/{visitor_id}",
            get(routes::features_ext::get_user_profile),
        )
        .route(
            "/api/v1/identity/aliases/{user_id}",
            get(routes::features_ext::list_user_aliases),
        )
        .route(
            "/api/v1/identity/graph",
            get(routes::features_ext::get_identity_graph),
        )
        .route(
            "/api/v1/identity/accounts",
            get(routes::features_ext::list_account_profiles),
        )
        .route(
            "/api/v1/identity/accounts/{account_id}",
            get(routes::features_ext::get_account_profile),
        )
        .route(
            "/api/v1/identity/accounts/{account_id}/members",
            get(routes::features_ext::list_account_members),
        )
        .route(
            "/api/v1/identity/accounts/{account_id}/analytics",
            get(routes::features_ext::get_account_analytics),
        )
        .route(
            "/api/v1/scim/users",
            get(routes::features_ext::list_scim_users).post(routes::features_ext::create_scim_user),
        )
        .route(
            "/api/v1/scim/users/{id}",
            get(routes::features_ext::get_scim_user)
                .put(routes::features_ext::update_scim_user)
                .delete(routes::features_ext::delete_scim_user),
        )
        .route(
            "/api/v1/scim/groups",
            get(routes::features_ext::list_scim_groups)
                .post(routes::features_ext::create_scim_group),
        )
        .route(
            "/api/v1/scim/groups/{id}",
            get(routes::features_ext::get_scim_group)
                .put(routes::features_ext::update_scim_group)
                .delete(routes::features_ext::delete_scim_group),
        )
        // Segments
        .route(
            "/api/v1/segments",
            get(routes::features_ext::list_segments).post(routes::features_ext::create_segment),
        )
        .route(
            "/api/v1/segments/compare",
            get(routes::features_ext::compare_segments),
        )
        .route(
            "/api/v1/segments/{id}",
            get(routes::features_ext::get_segment)
                .put(routes::features_ext::update_segment)
                .delete(routes::features_ext::delete_segment),
        )
        .route(
            "/api/v1/segments/{id}/evaluate",
            get(routes::features_ext::evaluate_segment),
        )
        .route(
            "/api/v1/segments/{id}/breakdown",
            get(routes::features_ext::breakdown_segment),
        )
        .route(
            "/api/v1/privacy/settings",
            get(routes::features_ext::get_privacy_settings)
                .put(routes::features_ext::update_privacy_settings),
        )
        // Governance
        .route(
            "/api/v1/governance/tracking-plans",
            get(routes::features_ext::list_tracking_plans)
                .post(routes::features_ext::create_tracking_plan),
        )
        .route(
            "/api/v1/governance/tracking-plans/{id}",
            get(routes::features_ext::get_tracking_plan)
                .put(routes::features_ext::update_tracking_plan)
                .delete(routes::features_ext::delete_tracking_plan),
        )
        .route(
            "/api/v1/governance/event-schemas",
            get(routes::features_ext::list_event_schemas)
                .post(routes::features_ext::create_event_schema),
        )
        .route(
            "/api/v1/governance/event-schemas/{id}",
            get(routes::features_ext::get_event_schema)
                .put(routes::features_ext::update_event_schema)
                .delete(routes::features_ext::delete_event_schema),
        )
        .route(
            "/api/v1/governance/event-schemas/{id}/status",
            axum::routing::put(routes::features_ext::update_event_schema_status),
        )
        .route(
            "/api/v1/governance/data-dictionary",
            get(routes::features_ext::list_data_dictionary_entries)
                .post(routes::features_ext::create_data_dictionary_entry),
        )
        .route(
            "/api/v1/governance/data-dictionary/{id}",
            axum::routing::put(routes::features_ext::update_data_dictionary_entry)
                .delete(routes::features_ext::delete_data_dictionary_entry),
        )
        .route(
            "/api/v1/governance/violations",
            get(routes::features_ext::list_quality_violations),
        )
        .route(
            "/api/v1/governance/health",
            get(routes::features_ext::get_governance_health),
        )
        // Privacy / Audit
        .route(
            "/api/v1/privacy/users/{visitor_id}/export",
            get(routes::features_ext::export_visitor_data),
        )
        .route(
            "/api/v1/privacy/users/{visitor_id}",
            axum::routing::delete(routes::features_ext::delete_visitor_data),
        )
        .route(
            "/api/v1/audit-logs",
            get(routes::features_ext::list_audit_logs),
        )
        // Alerts
        .route(
            "/api/v1/alerts",
            get(routes::features_ext::list_alerts).post(routes::features_ext::create_alert),
        )
        .route(
            "/api/v1/alerts/{id}",
            axum::routing::put(routes::features_ext::update_alert)
                .delete(routes::features_ext::delete_alert),
        )
        .route(
            "/api/v1/alerts/{id}/toggle",
            post(routes::features_ext::toggle_alert),
        )
        // Feature Flags / Remote Config
        .route(
            "/api/v1/feature-flags",
            get(routes::features_ext::list_feature_flags)
                .post(routes::features_ext::create_feature_flag),
        )
        .route(
            "/api/v1/feature-flags/{id}",
            get(routes::features_ext::get_feature_flag)
                .put(routes::features_ext::update_feature_flag)
                .delete(routes::features_ext::delete_feature_flag),
        )
        .route(
            "/api/v1/feature-flags/{key}/evaluate",
            post(routes::features_ext::evaluate_feature_flag),
        )
        .route(
            "/api/v1/feature-flags/{id}/evaluations",
            get(routes::features_ext::list_feature_flag_evaluations),
        )
        .route(
            "/api/v1/remote-config",
            get(routes::features_ext::list_remote_configs)
                .post(routes::features_ext::create_remote_config),
        )
        .route(
            "/api/v1/remote-config/{id}",
            get(routes::features_ext::get_remote_config)
                .put(routes::features_ext::update_remote_config)
                .delete(routes::features_ext::delete_remote_config),
        )
        .route(
            "/api/v1/remote-config/{key}/evaluate",
            post(routes::features_ext::evaluate_remote_config),
        )
        // Experiments
        .route(
            "/api/v1/experiments",
            get(routes::features_ext::list_experiments)
                .post(routes::features_ext::create_experiment),
        )
        .route(
            "/api/v1/experiments/{id}",
            get(routes::features_ext::get_experiment)
                .delete(routes::features_ext::delete_experiment),
        )
        .route(
            "/api/v1/experiments/{id}/status",
            axum::routing::put(routes::features_ext::update_experiment_status),
        )
        .route(
            "/api/v1/experiments/{id}/results",
            get(routes::features_ext::get_experiment_results),
        )
        .route(
            "/api/v1/experiments/{id}/assign",
            post(routes::features_ext::assign_experiment_visitor),
        )
        // Surveys
        .route(
            "/api/v1/surveys",
            get(routes::features_ext::list_surveys).post(routes::features_ext::create_survey),
        )
        .route(
            "/api/v1/surveys/active",
            get(routes::features_ext::get_active_surveys),
        )
        .route(
            "/api/v1/surveys/{id}",
            get(routes::features_ext::get_survey)
                .put(routes::features_ext::update_survey)
                .delete(routes::features_ext::delete_survey),
        )
        .route(
            "/api/v1/surveys/{id}/status",
            axum::routing::put(routes::features_ext::update_survey_status),
        )
        .route(
            "/api/v1/surveys/{id}/responses",
            get(routes::features_ext::get_survey_responses),
        )
        .route(
            "/api/v1/surveys/{id}/stats",
            get(routes::features_ext::get_survey_stats),
        )
        .route(
            "/api/v1/surveys/{id}/nps",
            get(routes::features_ext::get_survey_nps),
        )
        .route(
            "/api/v1/surveys/{id}/sentiment",
            get(routes::features_ext::get_survey_sentiment),
        )
        .route(
            "/api/v1/guides",
            get(routes::features_ext::list_guides).post(routes::features_ext::create_guide),
        )
        .route(
            "/api/v1/guides/active",
            get(routes::features_ext::get_active_guides),
        )
        .route(
            "/api/v1/guides/{id}",
            get(routes::features_ext::get_guide)
                .put(routes::features_ext::update_guide)
                .delete(routes::features_ext::delete_guide),
        )
        .route(
            "/api/v1/guides/{id}/status",
            axum::routing::put(routes::features_ext::update_guide_status),
        )
        .route(
            "/api/v1/guides/{id}/events",
            get(routes::features_ext::list_guide_events)
                .post(routes::features_ext::record_guide_event),
        )
        .route(
            "/api/v1/guides/{id}/stats",
            get(routes::features_ext::get_guide_stats),
        )
        // Session Replay
        .route(
            "/api/v1/session-replay",
            get(routes::features_ext::list_session_recordings),
        )
        .route(
            "/api/v1/session-replay/{id}",
            get(routes::features_ext::get_session_recording),
        )
        // Web Vitals
        .route(
            "/api/v1/webvitals",
            get(routes::features_ext::get_vitals_summary),
        )
        .route(
            "/api/v1/webvitals/pages",
            get(routes::features_ext::get_vitals_by_page),
        )
        .route(
            "/api/v1/webvitals/timeseries",
            get(routes::features_ext::get_vitals_timeseries),
        )
        // Error Tracking
        .route(
            "/api/v1/errors",
            get(routes::features_ext::get_error_groups),
        )
        .route(
            "/api/v1/errors/detail",
            get(routes::features_ext::get_error_detail),
        )
        .route(
            "/api/v1/errors/timeseries",
            get(routes::features_ext::get_error_timeseries),
        )
        .route(
            "/api/v1/errors/stats",
            get(routes::features_ext::get_error_stats),
        )
        .route(
            "/api/v1/releases",
            get(routes::features_ext::list_releases).post(routes::features_ext::create_release),
        )
        .route(
            "/api/v1/releases/{id}",
            axum::routing::delete(routes::features_ext::delete_release),
        )
        .route(
            "/api/v1/source-maps",
            get(routes::features_ext::list_source_maps)
                .post(routes::features_ext::register_source_map),
        )
        .route(
            "/api/v1/source-maps/{id}",
            axum::routing::delete(routes::features_ext::delete_source_map),
        )
        .route("/api/v1/logs", get(routes::features_ext::get_logs))
        .route(
            "/api/v1/logs/stats",
            get(routes::features_ext::get_log_stats),
        )
        // Heatmaps
        .route(
            "/api/v1/heatmaps",
            get(routes::features_ext::get_click_heatmap),
        )
        .route(
            "/api/v1/heatmaps/stats",
            get(routes::features_ext::get_click_stats),
        )
        .route(
            "/api/v1/heatmaps/labels",
            get(routes::features_ext::list_visual_event_labels)
                .post(routes::features_ext::create_visual_event_label),
        )
        .route(
            "/api/v1/heatmaps/labels/stats",
            get(routes::features_ext::list_visual_event_label_stats),
        )
        .route(
            "/api/v1/heatmaps/labels/{id}",
            axum::routing::put(routes::features_ext::update_visual_event_label)
                .delete(routes::features_ext::delete_visual_event_label),
        )
        .route(
            "/api/v1/heatmaps/labels/{id}/stats",
            get(routes::features_ext::get_visual_event_label_stats),
        )
        .route(
            "/api/v1/heatmaps/friction",
            get(routes::features_ext::get_friction_signals),
        )
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
            axum::routing::put(routes::admin::update_webhook).delete(routes::admin::delete_webhook),
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
        .route(
            "/dashboard/login",
            get(routes::dashboard::login_page).post(routes::dashboard::login_submit),
        )
        .route("/dashboard/logout", post(routes::dashboard::logout))
        .route("/dashboard/overview", get(routes::dashboard::overview_page))
        .route("/dashboard/pages", get(routes::dashboard::pages_page))
        .route(
            "/dashboard/referrers",
            get(routes::dashboard::referrers_page),
        )
        .route("/dashboard/events", get(routes::dashboard::events_page))
        .route("/dashboard/devices", get(routes::dashboard::devices_page))
        .route("/dashboard/geo", get(routes::dashboard::geo_page))
        .route("/dashboard/realtime", get(routes::dashboard::realtime_page))
        .route(
            "/dashboard/api/stats",
            get(routes::dashboard::htmx_stats_cards),
        )
        .route(
            "/dashboard/api/timeseries",
            get(routes::dashboard::htmx_timeseries),
        )
        .route(
            "/dashboard/api/pages",
            get(routes::dashboard::htmx_pages_table),
        )
        .route(
            "/dashboard/api/referrers",
            get(routes::dashboard::htmx_referrers_table),
        )
        .route(
            "/dashboard/api/events",
            get(routes::dashboard::htmx_events_table),
        )
        .route(
            "/dashboard/api/devices",
            get(routes::dashboard::htmx_devices_table),
        )
        .route("/dashboard/api/geo", get(routes::dashboard::htmx_geo_table))
        .route(
            "/dashboard/api/realtime",
            get(routes::dashboard::htmx_realtime),
        )
        // Visitor pages
        .route("/dashboard/visitors", get(routes::dashboard::visitors_page))
        .route(
            "/dashboard/visitors/{visitor_id}",
            get(routes::dashboard::visitor_detail_page),
        )
        .route("/dashboard/pricing", get(routes::dashboard::pricing_page))
        // Visitor HTMX API
        .route(
            "/dashboard/api/visitors/live-count",
            get(routes::dashboard::htmx_visitors_live_count),
        )
        .route(
            "/dashboard/api/visitors/activity-feed",
            get(routes::dashboard::htmx_visitors_activity_feed),
        )
        .route(
            "/dashboard/api/visitors/table",
            get(routes::dashboard::htmx_visitors_table),
        )
        .route(
            "/dashboard/api/visitor/{visitor_id}/summary",
            get(routes::dashboard::htmx_visitor_summary),
        )
        .route(
            "/dashboard/api/visitor/{visitor_id}/sessions",
            get(routes::dashboard::htmx_visitor_sessions),
        )
        .route(
            "/dashboard/api/visitor/{visitor_id}/session/{session_id}/detail",
            get(routes::dashboard::htmx_visitor_session_detail),
        )
        .route(
            "/dashboard/api/visitor/{visitor_id}/activity-chart",
            get(routes::dashboard::htmx_visitor_activity_chart),
        )
        .route(
            "/dashboard/api/visitor/{visitor_id}/events-breakdown",
            get(routes::dashboard::htmx_visitor_events_breakdown),
        )
        // Pricing HTMX API
        .route(
            "/dashboard/api/pricing/stats",
            get(routes::dashboard::htmx_pricing_stats),
        )
        .route(
            "/dashboard/api/pricing/timeseries",
            get(routes::dashboard::htmx_pricing_timeseries),
        )
        .route(
            "/dashboard/api/pricing/frequency",
            get(routes::dashboard::htmx_pricing_frequency),
        )
        .route(
            "/dashboard/api/pricing/referrers",
            get(routes::dashboard::htmx_pricing_referrers),
        )
        .route(
            "/dashboard/api/pricing/heatmap",
            get(routes::dashboard::htmx_pricing_heatmap),
        )
        .route(
            "/dashboard/api/pricing/funnel",
            get(routes::dashboard::htmx_pricing_funnel),
        );

    // Public routes (no auth)
    let public_routes = Router::new()
        .route("/", get(routes::docs::serve_home))
        .route("/health", get(routes::health::health_check))
        .route("/api/script.js", get(routes::script::serve_script))
        .route(
            "/api/embed/bi/{token}",
            get(routes::features_ext::resolve_bi_embed),
        )
        .route("/api/docs", get(routes::docs::serve_docs))
        .route("/docs", get(routes::docs::redirect_docs));

    let app = Router::new()
        .merge(ingest_routes)
        .merge(source_ingest_routes)
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
