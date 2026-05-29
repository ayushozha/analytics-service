use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Extension;
use rand::RngExt;
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::module::{
    module_metadata, ModuleConfig, ModuleInfo, UpdateModulesRequest, ALL_MODULES,
};
use crate::models::project::{ApiKeyResponse, ApiKeyRow, CreateApiKey, CreateProject, Project};
use crate::models::webhook::{CreateWebhook, UpdateWebhook, Webhook};
use crate::services;
use crate::state::SharedState;

fn generate_api_key(prefix: &str) -> String {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..24).map(|_| rng.random()).collect();
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &random_bytes,
    );
    format!("{prefix}{}", &encoded[..24])
}

fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn create_project(
    Extension(state): Extension<SharedState>,
    axum::Json(input): axum::Json<CreateProject>,
) -> AppResult<impl IntoResponse> {
    let project: Project = sqlx::query_as(
        "INSERT INTO projects (name, domain, umami_website_id, umami_share_url, settings) VALUES ($1, $2, $3, $4, $5) RETURNING id, name, domain, umami_website_id, umami_share_url, settings, created_at, updated_at"
    )
    .bind(&input.name)
    .bind(&input.domain)
    .bind(&input.umami_website_id)
    .bind(&input.umami_share_url)
    .bind(&input.settings)
    .fetch_one(&state.db)
    .await?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(project)))
}

pub async fn list_projects(
    Extension(state): Extension<SharedState>,
) -> AppResult<impl IntoResponse> {
    let projects: Vec<Project> = sqlx::query_as(
        "SELECT id, name, domain, umami_website_id, umami_share_url, settings, created_at, updated_at FROM projects ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(axum::Json(projects))
}

pub async fn get_project(
    Extension(state): Extension<SharedState>,
    Path(project_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let project: Option<Project> = sqlx::query_as(
        "SELECT id, name, domain, umami_website_id, umami_share_url, settings, created_at, updated_at FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_optional(&state.db)
    .await?;

    let project = project.ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
    Ok(axum::Json(project))
}

pub async fn create_api_key(
    Extension(state): Extension<SharedState>,
    Path(project_id): Path<Uuid>,
    axum::Json(input): axum::Json<CreateApiKey>,
) -> AppResult<impl IntoResponse> {
    // Verify project exists
    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Project not found".to_string()));
    }

    let full_key = generate_api_key("pa_live_");
    let key_hash = hash_key(&full_key);
    let key_prefix = full_key[..8].to_string();

    let row: ApiKeyRow = sqlx::query_as(
        "INSERT INTO api_keys (project_id, key_hash, key_prefix, name, scopes, expires_at, allowed_modules) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id, project_id, key_hash, key_prefix, name, scopes, last_used_at, expires_at, created_at, is_active, allowed_modules"
    )
    .bind(project_id)
    .bind(&key_hash)
    .bind(&key_prefix)
    .bind(&input.name)
    .bind(&input.scopes)
    .bind(input.expires_at)
    .bind(&input.allowed_modules)
    .fetch_one(&state.db)
    .await?;

    let response = ApiKeyResponse {
        id: row.id,
        project_id: row.project_id,
        key: full_key,
        name: row.name,
        scopes: row.scopes,
        created_at: row.created_at,
    };

    Ok((axum::http::StatusCode::CREATED, axum::Json(response)))
}

pub async fn list_api_keys(
    Extension(state): Extension<SharedState>,
    Path(project_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let keys: Vec<ApiKeyRow> = sqlx::query_as(
        "SELECT id, project_id, key_hash, key_prefix, name, scopes, last_used_at, expires_at, created_at, is_active, allowed_modules FROM api_keys WHERE project_id = $1 ORDER BY created_at DESC"
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await?;

    Ok(axum::Json(keys))
}

pub async fn revoke_api_key(
    Extension(state): Extension<SharedState>,
    Path((project_id, key_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    let revoked: Option<(String,)> = sqlx::query_as(
        "UPDATE api_keys SET is_active = false WHERE id = $1 AND project_id = $2 RETURNING key_hash",
    )
    .bind(key_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await?;

    let Some((key_hash,)) = revoked else {
        return Err(AppError::NotFound("API key not found".to_string()));
    };

    // Invalidate the auth middleware's cached key resolution so the revocation takes effect
    // immediately rather than after the 5-minute cache TTL (see middleware::auth).
    let cache_key = state.redis_key(&format!("apikey:{key_hash}"));
    let mut redis = state.redis.clone();
    let _: Result<i64, _> = redis.del(&cache_key).await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// --- Webhook CRUD ---

pub async fn create_webhook(
    Extension(state): Extension<SharedState>,
    Path(project_id): Path<Uuid>,
    axum::Json(input): axum::Json<CreateWebhook>,
) -> AppResult<impl IntoResponse> {
    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Project not found".to_string()));
    }

    crate::services::ssrf::ensure_public_http_url(&input.url)
        .map_err(|reason| AppError::BadRequest(format!("webhook url rejected: {reason}")))?;

    let webhook: Webhook = sqlx::query_as(
        "INSERT INTO webhooks (project_id, url, events, secret) VALUES ($1, $2, $3, $4) \
         RETURNING id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.url)
    .bind(&input.events)
    .bind(&input.secret)
    .fetch_one(&state.db)
    .await?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(webhook)))
}

pub async fn list_webhooks(
    Extension(state): Extension<SharedState>,
    Path(project_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let webhooks: Vec<Webhook> = sqlx::query_as(
        "SELECT id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at \
         FROM webhooks WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await?;

    Ok(axum::Json(webhooks))
}

pub async fn update_webhook(
    Extension(state): Extension<SharedState>,
    Path((project_id, webhook_id)): Path<(Uuid, Uuid)>,
    axum::Json(input): axum::Json<UpdateWebhook>,
) -> AppResult<impl IntoResponse> {
    let existing: Option<Webhook> = sqlx::query_as(
        "SELECT id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at \
         FROM webhooks WHERE id = $1 AND project_id = $2",
    )
    .bind(webhook_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await?;

    let existing = existing.ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    let url = input.url.unwrap_or(existing.url);
    crate::services::ssrf::ensure_public_http_url(&url)
        .map_err(|reason| AppError::BadRequest(format!("webhook url rejected: {reason}")))?;
    let events = input.events.unwrap_or(existing.events);
    let is_active = input.is_active.unwrap_or(existing.is_active);

    let webhook: Webhook = sqlx::query_as(
        "UPDATE webhooks SET url = $1, events = $2, is_active = $3, updated_at = NOW() \
         WHERE id = $4 AND project_id = $5 \
         RETURNING id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at",
    )
    .bind(&url)
    .bind(&events)
    .bind(is_active)
    .bind(webhook_id)
    .bind(project_id)
    .fetch_one(&state.db)
    .await?;

    Ok(axum::Json(webhook))
}

pub async fn delete_webhook(
    Extension(state): Extension<SharedState>,
    Path((project_id, webhook_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    let result = sqlx::query("DELETE FROM webhooks WHERE id = $1 AND project_id = $2")
        .bind(webhook_id)
        .bind(project_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn test_webhook(
    Extension(state): Extension<SharedState>,
    Path((project_id, webhook_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    let webhook: Option<Webhook> = sqlx::query_as(
        "SELECT id, project_id, url, events, secret, is_active, last_triggered_at, created_at, updated_at \
         FROM webhooks WHERE id = $1 AND project_id = $2",
    )
    .bind(webhook_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await?;

    let webhook = webhook.ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    let payload = serde_json::json!({
        "event": "test",
        "project_id": project_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": {
            "message": "This is a test webhook from Pulse Analytics"
        }
    });

    let body = serde_json::to_string(&payload).unwrap();
    let mut req = reqwest::Client::new()
        .post(&webhook.url)
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(5));

    if let Some(secret) = &webhook.secret {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid key");
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        req = req.header("X-Pulse-Signature", signature);
    }

    let result = req.body(body).send().await;

    match result {
        Ok(resp) => Ok(axum::Json(serde_json::json!({
            "success": resp.status().is_success(),
            "status": resp.status().as_u16(),
        }))),
        Err(e) => Ok(axum::Json(serde_json::json!({
            "success": false,
            "error": e.to_string(),
        }))),
    }
}

// --- Module Management ---

/// List all available modules with their current config for a project.
pub async fn list_modules(
    Extension(state): Extension<SharedState>,
    Path(project_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let settings = services::modules::get_project_settings(&state, project_id).await?;

    let modules: Vec<ModuleInfo> = ALL_MODULES
        .iter()
        .map(|&name| {
            let config = settings.modules.get(name).cloned().unwrap_or_default();
            let (description, category) = module_metadata(name);
            ModuleInfo {
                name: name.to_string(),
                enabled: config.enabled,
                access: config.access,
                description: description.to_string(),
                category: category.to_string(),
            }
        })
        .collect();

    Ok(axum::Json(modules))
}

/// Update module configuration for a project.
/// Only updates the modules specified in the request; others remain unchanged.
pub async fn update_modules(
    Extension(state): Extension<SharedState>,
    Path(project_id): Path<Uuid>,
    axum::Json(input): axum::Json<UpdateModulesRequest>,
) -> AppResult<impl IntoResponse> {
    // Load current settings
    let mut settings = services::modules::get_project_settings(&state, project_id).await?;

    // Validate module names
    for name in input.modules.keys() {
        if name == "core" {
            return Err(AppError::BadRequest(
                "Cannot modify 'core' module — it is always enabled".to_string(),
            ));
        }
        if !ALL_MODULES.contains(&name.as_str()) {
            return Err(AppError::BadRequest(format!("Unknown module: {name}")));
        }
    }

    // Merge updates
    for (name, config) in input.modules {
        settings.modules.insert(name, config);
    }

    // Ensure core stays enabled
    settings.modules.entry("core".to_string()).and_modify(|c| {
        c.enabled = true;
    });

    let settings_json =
        serde_json::to_value(&settings).map_err(|e| AppError::Internal(e.to_string()))?;

    // Persist
    sqlx::query("UPDATE projects SET settings = $1, updated_at = NOW() WHERE id = $2")
        .bind(&settings_json)
        .bind(project_id)
        .execute(&state.db)
        .await?;

    // Invalidate cache
    services::modules::invalidate_settings_cache(&state, project_id).await;

    Ok(axum::Json(serde_json::json!({
        "ok": true,
        "modules": settings.modules,
    })))
}

/// Enable a single module.
pub async fn enable_module(
    Extension(state): Extension<SharedState>,
    Path((project_id, module_name)): Path<(Uuid, String)>,
) -> AppResult<impl IntoResponse> {
    if !ALL_MODULES.contains(&module_name.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unknown module: {module_name}"
        )));
    }

    let mut settings = services::modules::get_project_settings(&state, project_id).await?;
    settings
        .modules
        .entry(module_name.clone())
        .and_modify(|c| {
            c.enabled = true;
        })
        .or_insert(ModuleConfig {
            enabled: true,
            access: crate::models::module::ModuleAccess::All,
        });

    let settings_json =
        serde_json::to_value(&settings).map_err(|e| AppError::Internal(e.to_string()))?;

    sqlx::query("UPDATE projects SET settings = $1, updated_at = NOW() WHERE id = $2")
        .bind(&settings_json)
        .bind(project_id)
        .execute(&state.db)
        .await?;

    services::modules::invalidate_settings_cache(&state, project_id).await;

    Ok(axum::Json(serde_json::json!({
        "ok": true,
        "module": module_name,
        "enabled": true,
    })))
}

/// Disable a single module.
pub async fn disable_module(
    Extension(state): Extension<SharedState>,
    Path((project_id, module_name)): Path<(Uuid, String)>,
) -> AppResult<impl IntoResponse> {
    if module_name == "core" {
        return Err(AppError::BadRequest(
            "Cannot disable 'core' module".to_string(),
        ));
    }

    if !ALL_MODULES.contains(&module_name.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unknown module: {module_name}"
        )));
    }

    let mut settings = services::modules::get_project_settings(&state, project_id).await?;
    settings.modules.entry(module_name.clone()).and_modify(|c| {
        c.enabled = false;
    });

    let settings_json =
        serde_json::to_value(&settings).map_err(|e| AppError::Internal(e.to_string()))?;

    sqlx::query("UPDATE projects SET settings = $1, updated_at = NOW() WHERE id = $2")
        .bind(&settings_json)
        .bind(project_id)
        .execute(&state.db)
        .await?;

    services::modules::invalidate_settings_cache(&state, project_id).await;

    Ok(axum::Json(serde_json::json!({
        "ok": true,
        "module": module_name,
        "enabled": false,
    })))
}

/// Update access level for a module.
#[derive(Debug, serde::Deserialize)]
pub struct UpdateModuleAccess {
    pub access: crate::models::module::ModuleAccess,
}

pub async fn update_module_access(
    Extension(state): Extension<SharedState>,
    Path((project_id, module_name)): Path<(Uuid, String)>,
    axum::Json(input): axum::Json<UpdateModuleAccess>,
) -> AppResult<impl IntoResponse> {
    if !ALL_MODULES.contains(&module_name.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unknown module: {module_name}"
        )));
    }

    let mut settings = services::modules::get_project_settings(&state, project_id).await?;
    settings.modules.entry(module_name.clone()).and_modify(|c| {
        c.access = input.access.clone();
    });

    let settings_json =
        serde_json::to_value(&settings).map_err(|e| AppError::Internal(e.to_string()))?;

    sqlx::query("UPDATE projects SET settings = $1, updated_at = NOW() WHERE id = $2")
        .bind(&settings_json)
        .bind(project_id)
        .execute(&state.db)
        .await?;

    services::modules::invalidate_settings_cache(&state, project_id).await;

    Ok(axum::Json(serde_json::json!({
        "ok": true,
        "module": module_name,
        "access": input.access,
    })))
}
