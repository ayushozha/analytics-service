use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::Extension;
use redis::AsyncCommands;

use crate::error::AppError;
use crate::middleware::auth::AuthenticatedProject;
use crate::state::SharedState;

pub async fn rate_limit_middleware(
    Extension(state): Extension<SharedState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Rate limit is per-project
    if let Some(auth) = request.extensions().get::<AuthenticatedProject>() {
        let key = state.redis_key(&format!("ratelimit:{}", auth.project_id));
        let mut redis = state.redis.clone();
        let limit = state.config.rate_limit_per_second as i64;

        let count: i64 = redis.incr(&key, 1i64).await.unwrap_or(1);

        if count == 1 {
            // First request in this window — set TTL
            let _: () = redis.expire(&key, 1).await.unwrap_or(());
        }

        if count > limit {
            return Err(AppError::RateLimited);
        }
    }

    Ok(next.run(request).await)
}
