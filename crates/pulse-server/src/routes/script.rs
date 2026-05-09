use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

const PULSE_SCRIPT: &str = include_str!("../../static/pulse.min.js");

pub async fn serve_script() -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        PULSE_SCRIPT,
    )
}
