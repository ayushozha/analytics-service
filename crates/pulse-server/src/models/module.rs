use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// All available modules in the system.
/// "core" is always enabled and cannot be disabled.
pub const ALL_MODULES: &[&str] = &[
    "core",
    "identity",
    "segments",
    "dashboards",
    "governance",
    "funnels",
    "utm",
    "goals",
    "retention",
    "cohorts",
    "paths",
    "webvitals",
    "scroll",
    "revenue",
    "search",
    "outlinks",
    "logs",
    "exports",
    "integrations",
    "sources",
    "destinations",
    "bi",
    "sharing",
    "email_reports",
    "alerts",
    "feature_flags",
    "ab_testing",
    "session_replay",
    "heatmaps",
    "ai_queries",
    "predictions",
    "error_tracking",
    "surveys",
];

/// Modules that are enabled by default for new projects.
pub const DEFAULT_ENABLED_MODULES: &[&str] = &[
    "core",
    "identity",
    "segments",
    "dashboards",
    "governance",
    "utm",
    "goals",
    "funnels",
    "retention",
    "exports",
    "integrations",
    "sources",
    "destinations",
    "bi",
    "alerts",
    "feature_flags",
    "revenue",
    "error_tracking",
    "logs",
];

/// Convert legacy/table-oriented names to the canonical module names stored in
/// project settings.
pub fn canonical_module_name(module: &str) -> &str {
    match module {
        "web_vitals" => "webvitals",
        "scroll_depth" | "scroll_depths" => "scroll",
        "search_queries" => "search",
        "js_errors" => "error_tracking",
        "click_events" => "heatmaps",
        other => other,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModuleAccess {
    Read,
    Write,
    #[default]
    All,
}

impl ModuleAccess {
    pub fn allows_read(&self) -> bool {
        matches!(self, ModuleAccess::Read | ModuleAccess::All)
    }

    pub fn allows_write(&self) -> bool {
        matches!(self, ModuleAccess::Write | ModuleAccess::All)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub enabled: bool,
    #[serde(default)]
    pub access: ModuleAccess,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            access: ModuleAccess::All,
        }
    }
}

/// Project settings with typed module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    #[serde(default)]
    pub modules: HashMap<String, ModuleConfig>,
}

fn default_retention_days() -> u32 {
    365
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            retention_days: 365,
            modules: default_module_config(),
        }
    }
}

/// Generate default module configuration for new projects.
pub fn default_module_config() -> HashMap<String, ModuleConfig> {
    let mut modules = HashMap::new();
    for &module in ALL_MODULES {
        let enabled = DEFAULT_ENABLED_MODULES.contains(&module);
        modules.insert(
            module.to_string(),
            ModuleConfig {
                enabled: enabled || module == "core",
                access: ModuleAccess::All,
            },
        );
    }
    modules
}

/// Generate default project settings JSON.
pub fn default_project_settings() -> serde_json::Value {
    serde_json::to_value(ProjectSettings::default())
        .unwrap_or_else(|_| serde_json::json!({ "retention_days": 365 }))
}

/// Parse project settings from JSONB value.
/// Merges with defaults so missing modules get default (disabled) config.
pub fn parse_project_settings(settings: &serde_json::Value) -> ProjectSettings {
    let mut parsed: ProjectSettings = serde_json::from_value(settings.clone()).unwrap_or_default();

    // Ensure core is always enabled
    parsed
        .modules
        .entry("core".to_string())
        .and_modify(|c| {
            c.enabled = true;
        })
        .or_insert(ModuleConfig {
            enabled: true,
            access: ModuleAccess::All,
        });

    // Ensure all known modules have an entry, using the same defaults as new
    // projects so older settings documents pick up newly default-enabled modules.
    for &module in ALL_MODULES {
        let enabled = DEFAULT_ENABLED_MODULES.contains(&module) || module == "core";
        parsed
            .modules
            .entry(module.to_string())
            .or_insert(ModuleConfig {
                enabled,
                access: ModuleAccess::All,
            });
    }

    parsed
}

/// Check if a module is enabled for a project and has the required access level.
pub fn check_module_access(
    settings: &ProjectSettings,
    module: &str,
    require_write: bool,
) -> Result<(), ModuleError> {
    let module = canonical_module_name(module);

    if module == "core" {
        return Ok(());
    }

    if !ALL_MODULES.contains(&module) {
        return Err(ModuleError::UnknownModule(module.to_string()));
    }

    let config = settings.modules.get(module).cloned().unwrap_or_default();

    if !config.enabled {
        return Err(ModuleError::ModuleDisabled(module.to_string()));
    }

    if require_write && !config.access.allows_write() {
        return Err(ModuleError::InsufficientAccess {
            module: module.to_string(),
            required: "write".to_string(),
            current: "read".to_string(),
        });
    }

    if !require_write && !config.access.allows_read() {
        return Err(ModuleError::InsufficientAccess {
            module: module.to_string(),
            required: "read".to_string(),
            current: "write".to_string(),
        });
    }

    Ok(())
}

/// Check if an API key is allowed to access a specific module.
/// If allowed_modules is None/empty, the key can access all enabled modules.
pub fn check_api_key_module_access(allowed_modules: &Option<Vec<String>>, module: &str) -> bool {
    let module = canonical_module_name(module);

    if module == "core" {
        return true;
    }
    match allowed_modules {
        Some(modules) if !modules.is_empty() => {
            modules.iter().any(|m| canonical_module_name(m) == module)
        }
        _ => true, // No restrictions = access all enabled modules
    }
}

#[derive(Debug)]
pub enum ModuleError {
    UnknownModule(String),
    ModuleDisabled(String),
    InsufficientAccess {
        module: String,
        required: String,
        current: String,
    },
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleError::UnknownModule(m) => write!(f, "Unknown module: {m}"),
            ModuleError::ModuleDisabled(m) => {
                write!(f, "Module '{m}' is not enabled for this project")
            }
            ModuleError::InsufficientAccess {
                module,
                required,
                current,
            } => {
                write!(
                    f,
                    "Module '{module}' requires '{required}' access, but configured as '{current}'"
                )
            }
        }
    }
}

/// API types for module management
#[derive(Debug, Deserialize)]
pub struct UpdateModulesRequest {
    pub modules: HashMap<String, ModuleConfig>,
}

#[derive(Debug, Serialize)]
pub struct ModuleInfo {
    pub name: String,
    pub enabled: bool,
    pub access: ModuleAccess,
    pub description: String,
    pub category: String,
}

/// Get description and category for a module.
pub fn module_metadata(name: &str) -> (&'static str, &'static str) {
    match name {
        "core" => (
            "Core analytics: pageviews, sessions, events, devices, geo, referrers, realtime",
            "core",
        ),
        "identity" => ("User identity, profiles, aliases, and traits", "core"),
        "segments" => (
            "Saved behavioral and identity-based visitor segments",
            "analysis",
        ),
        "dashboards" => (
            "Custom dashboards, saved reports, query explorer, and product insights",
            "analysis",
        ),
        "governance" => (
            "Tracking plans, event schemas, data dictionary, and quality monitoring",
            "platform",
        ),
        "funnels" => ("User-defined multi-step conversion funnels", "analysis"),
        "utm" => ("UTM campaign parameter tracking and reporting", "tracking"),
        "goals" => (
            "Goal/conversion tracking with configurable triggers",
            "analysis",
        ),
        "retention" => (
            "Returning visitor retention analysis (D1/D7/D30)",
            "analysis",
        ),
        "cohorts" => (
            "Cohort analysis by acquisition date or behavior",
            "analysis",
        ),
        "paths" => ("User journey and page flow visualization", "analysis"),
        "webvitals" => (
            "Core Web Vitals (LCP, FID, INP, CLS, FCP, TTFB)",
            "tracking",
        ),
        "scroll" => ("Scroll depth tracking (25/50/75/100%)", "tracking"),
        "revenue" => ("Revenue and ecommerce event tracking", "tracking"),
        "search" => ("Internal site search query tracking", "tracking"),
        "outlinks" => (
            "External link clicks and file download tracking",
            "tracking",
        ),
        "logs" => (
            "Application log ingestion and release filtering",
            "tracking",
        ),
        "exports" => ("CSV data export from dashboard and API", "export"),
        "integrations" => (
            "Integrations marketplace catalog for sources, destinations, SDKs, and imports",
            "export",
        ),
        "sources" => (
            "Source catalog, webhook collection, ingestion audit, and source tokens",
            "export",
        ),
        "destinations" => (
            "Destination catalog, event routing, retries, and dead letters",
            "export",
        ),
        "bi" => (
            "Safe SQL editor, semantic metrics, visual query builder, and CSV uploads",
            "analysis",
        ),
        "sharing" => ("Public shareable dashboard links", "export"),
        "email_reports" => ("Scheduled email digest reports", "export"),
        "alerts" => ("Custom metric threshold alerting", "export"),
        "feature_flags" => (
            "Feature flags, remote config, targeting rules, and rollout evaluation",
            "advanced",
        ),
        "ab_testing" => ("A/B testing with experiment variants and goals", "advanced"),
        "session_replay" => ("Session recording and replay", "advanced"),
        "heatmaps" => ("Click and interaction heatmaps", "advanced"),
        "ai_queries" => ("Natural language analytics queries", "advanced"),
        "predictions" => ("Predictive analytics and churn probability", "advanced"),
        "error_tracking" => ("JavaScript error collection and grouping", "tracking"),
        "surveys" => ("In-app user surveys with targeting", "advanced"),
        _ => ("Unknown module", "unknown"),
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_module_name, check_api_key_module_access, check_module_access};
    use super::{ModuleAccess, ModuleConfig, ProjectSettings};
    use std::collections::HashMap;

    #[test]
    fn canonicalizes_ingestion_module_aliases() {
        assert_eq!(canonical_module_name("web_vitals"), "webvitals");
        assert_eq!(canonical_module_name("scroll_depth"), "scroll");
        assert_eq!(canonical_module_name("search_queries"), "search");
        assert_eq!(canonical_module_name("js_errors"), "error_tracking");
        assert_eq!(canonical_module_name("click_events"), "heatmaps");
        assert_eq!(canonical_module_name("surveys"), "surveys");
    }

    #[test]
    fn module_access_accepts_legacy_aliases() {
        let mut modules = HashMap::new();
        modules.insert(
            "webvitals".to_string(),
            ModuleConfig {
                enabled: true,
                access: ModuleAccess::All,
            },
        );
        let settings = ProjectSettings {
            retention_days: 365,
            modules,
        };

        assert!(check_module_access(&settings, "web_vitals", true).is_ok());
    }

    #[test]
    fn api_key_module_access_accepts_legacy_aliases() {
        let allowed = Some(vec!["error_tracking".to_string()]);
        assert!(check_api_key_module_access(&allowed, "js_errors"));
    }
}
