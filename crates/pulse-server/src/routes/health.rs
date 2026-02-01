use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use serde_json::json;

use crate::state::SharedState;

pub async fn health_check(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();

    let redis_ok = {
        let mut redis = state.redis.clone();
        let result: Result<String, _> = redis::cmd("PING").query_async(&mut redis).await;
        result.is_ok()
    };

    let status = if db_ok && redis_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        axum::Json(json!({
            "status": if db_ok && redis_ok { "healthy" } else { "unhealthy" },
            "database": db_ok,
            "redis": redis_ok,
        })),
    )
}
