use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

const DOCS_HTML: &str = include_str!("../../static/docs.html");

pub async fn serve_docs() -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        DOCS_HTML,
    )
}
