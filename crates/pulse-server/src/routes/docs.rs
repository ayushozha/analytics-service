use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect};

const DOCS_HTML: &str = include_str!("../../static/docs.html");
const HOME_HTML: &str = include_str!("../../static/home.html");

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

pub async fn serve_home() -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        HOME_HTML,
    )
}

/// Redirect old /docs path to /api/docs for backwards compatibility.
pub async fn redirect_docs() -> Redirect {
    Redirect::permanent("/api/docs")
}
