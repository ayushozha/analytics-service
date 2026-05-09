use redis::AsyncCommands;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::module::{
    canonical_module_name, check_api_key_module_access, check_module_access,
    parse_project_settings, ModuleError, ProjectSettings,
};
use crate::state::SharedState;

const SETTINGS_CACHE_TTL: u64 = 300; // 5 minutes

/// Load project settings with Redis caching.
pub async fn get_project_settings(
    state: &SharedState,
    project_id: Uuid,
) -> AppResult<ProjectSettings> {
    let cache_key = state.redis_key(&format!("project_settings:{project_id}"));
    let mut redis = state.redis.clone();

    // Check cache
    let cached: Option<String> = redis.get(&cache_key).await.unwrap_or(None);
    if let Some(cached) = cached {
        if let Ok(settings) = serde_json::from_str::<ProjectSettings>(&cached) {
            return Ok(settings);
        }
    }

    // Query DB
    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT settings FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_optional(&state.db)
            .await?;

    let settings_json = row
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?
        .0;

    let settings = parse_project_settings(&settings_json);

    // Cache
    if let Ok(serialized) = serde_json::to_string(&settings) {
        let _: () = redis
            .set_ex(&cache_key, &serialized, SETTINGS_CACHE_TTL)
            .await
            .unwrap_or(());
    }

    Ok(settings)
}

/// Invalidate cached project settings (call after update).
pub async fn invalidate_settings_cache(state: &SharedState, project_id: Uuid) {
    let cache_key = state.redis_key(&format!("project_settings:{project_id}"));
    let mut redis = state.redis.clone();
    let _: () = redis.del(&cache_key).await.unwrap_or(());
}

/// Require a module to be enabled with read access.
/// Checks both project-level module config and API key module restrictions.
pub async fn require_module_read(
    state: &SharedState,
    project_id: Uuid,
    module: &str,
    api_key_modules: &Option<Vec<String>>,
) -> AppResult<()> {
    let settings = get_project_settings(state, project_id).await?;
    map_module_error(check_module_access(&settings, module, false))?;

    if !check_api_key_module_access(api_key_modules, module) {
        return Err(AppError::Forbidden(format!(
            "API key does not have access to module '{module}'"
        )));
    }

    Ok(())
}

/// Require a module to be enabled with write access.
pub async fn require_module_write(
    state: &SharedState,
    project_id: Uuid,
    module: &str,
    api_key_modules: &Option<Vec<String>>,
) -> AppResult<()> {
    let settings = get_project_settings(state, project_id).await?;
    map_module_error(check_module_access(&settings, module, true))?;

    if !check_api_key_module_access(api_key_modules, module) {
        return Err(AppError::Forbidden(format!(
            "API key does not have access to module '{module}'"
        )));
    }

    Ok(())
}

/// Check if a module is enabled (no API key check, for internal use).
pub async fn is_module_enabled(
    state: &SharedState,
    project_id: Uuid,
    module: &str,
) -> AppResult<bool> {
    let settings = get_project_settings(state, project_id).await?;
    let module = canonical_module_name(module);
    Ok(settings
        .modules
        .get(module)
        .map(|c| c.enabled)
        .unwrap_or(false))
}

fn map_module_error(result: Result<(), ModuleError>) -> AppResult<()> {
    result.map_err(|e| match e {
        ModuleError::ModuleDisabled(m) => {
            AppError::Forbidden(format!("Module '{m}' is not enabled for this project"))
        }
        ModuleError::InsufficientAccess {
            module,
            required,
            current,
        } => AppError::Forbidden(format!(
            "Module '{module}' requires '{required}' access, currently '{current}'"
        )),
        ModuleError::UnknownModule(m) => AppError::BadRequest(format!("Unknown module: {m}")),
    })
}
