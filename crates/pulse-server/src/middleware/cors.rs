use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

use crate::config::Config;

pub fn build_cors_layer(config: &Config) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-pulse-key"),
        ]);

    if config.allowed_origins.is_empty() || !config.is_production() {
        cors = cors.allow_origin(tower_http::cors::Any);
    } else {
        let origins: Vec<HeaderValue> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors = cors.allow_origin(origins);
    }

    cors
}
