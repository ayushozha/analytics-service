//! The React web app (`web/`): its static files and the session endpoint it boots from.

use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedProject;
use crate::state::SharedState;

/// Serves the built web app. Unknown paths fall back to `index.html` so the
/// client-side router can handle deep links such as `/app/persons/abc`.
pub fn app_service(dist_dir: &str) -> ServeDir<ServeFile> {
    ServeDir::new(dist_dir).fallback(ServeFile::new(format!("{dist_dir}/index.html")))
}

/// `GET /api/v2/me`: the project the caller is signed in to.
pub async fn me(
    Extension(state): Extension<SharedState>,
    auth: AuthenticatedProject,
) -> AppResult<impl IntoResponse> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM projects WHERE id = $1")
        .bind(auth.project_id)
        .fetch_optional(&state.db)
        .await?;
    let name = name.ok_or(AppError::Unauthorized)?;
    Ok(Json(
        json!({ "project_id": auth.project_id, "project_name": name }),
    ))
}
