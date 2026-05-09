use axum::extract::Request;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use axum::Extension;
use redis::AsyncCommands;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::models::project::ResolvedKey;
use crate::state::SharedState;

const API_KEY_CACHE_TTL: u64 = 300; // 5 minutes

#[derive(Debug, Clone)]
pub struct AuthenticatedProject {
    pub project_id: uuid::Uuid,
    pub scopes: Vec<String>,
    /// If set, restricts which modules this API key can access.
    /// None or empty means all enabled modules are accessible.
    pub allowed_modules: Option<Vec<String>>,
}

impl AuthenticatedProject {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope || s == "admin")
    }

    pub fn require_scope(&self, scope: &str) -> AppResult<()> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "Missing required scope: {scope}"
            )))
        }
    }
}

fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn auth_middleware(
    Extension(state): Extension<SharedState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract API key from header or query param
    let key = request
        .headers()
        .get("x-pulse-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            request.uri().query().and_then(|q| {
                q.split('&').find_map(|pair| {
                    let (k, v) = pair.split_once('=')?;
                    if k == "key" {
                        Some(v.to_string())
                    } else {
                        None
                    }
                })
            })
        })
        .ok_or(AppError::Unauthorized)?;

    let key_hash = hash_key(&key);
    let cache_key = state.redis_key(&format!("apikey:{key_hash}"));

    // Check Redis cache
    let mut redis = state.redis.clone();
    let cached: Option<String> = redis.get(&cache_key).await.unwrap_or(None);

    let resolved = if let Some(cached) = cached {
        serde_json::from_str::<ResolvedKey>(&cached)
            .map_err(|_| AppError::Internal("corrupt key cache".to_string()))?
    } else {
        // Query database
        let row: Option<(uuid::Uuid, Vec<String>, Option<Vec<String>>)> = sqlx::query_as(
            "SELECT project_id, scopes, allowed_modules FROM api_keys WHERE key_hash = $1 AND is_active = true AND (expires_at IS NULL OR expires_at > NOW())"
        )
        .bind(&key_hash)
        .fetch_optional(&state.db)
        .await?;

        let (project_id, scopes, allowed_modules) = row.ok_or(AppError::Unauthorized)?;
        let resolved = ResolvedKey {
            project_id,
            scopes,
            allowed_modules,
        };

        // Cache the result
        if let Ok(serialized) = serde_json::to_string(&resolved) {
            let _: () = redis
                .set_ex(&cache_key, &serialized, API_KEY_CACHE_TTL)
                .await
                .unwrap_or(());
        }

        // Update last_used_at (fire-and-forget)
        let db = state.db.clone();
        let hash = key_hash.clone();
        tokio::spawn(async move {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE key_hash = $1")
                .bind(&hash)
                .execute(&db)
                .await;
        });

        resolved
    };

    request.extensions_mut().insert(AuthenticatedProject {
        project_id: resolved.project_id,
        scopes: resolved.scopes,
        allowed_modules: resolved.allowed_modules,
    });

    Ok(next.run(request).await)
}

pub async fn admin_auth_middleware(
    Extension(state): Extension<SharedState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    if token != state.config.admin_token {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}

impl<S> axum::extract::FromRequestParts<S> for AuthenticatedProject
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedProject>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
