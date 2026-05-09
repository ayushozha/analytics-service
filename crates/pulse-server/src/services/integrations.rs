use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Integration {
    pub key: String,
    pub name: String,
    pub category: String,
    pub status: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub setup: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationFilter {
    pub category: Option<String>,
    pub capability: Option<String>,
    pub status: Option<String>,
}

pub fn list_integrations(filter: IntegrationFilter) -> Vec<Integration> {
    catalog()
        .into_iter()
        .filter(|integration| {
            filter
                .category
                .as_deref()
                .is_none_or(|category| integration.category == category)
        })
        .filter(|integration| {
            filter
                .status
                .as_deref()
                .is_none_or(|status| integration.status == status)
        })
        .filter(|integration| {
            filter.capability.as_deref().is_none_or(|capability| {
                integration
                    .capabilities
                    .iter()
                    .any(|item| item == capability)
            })
        })
        .collect()
}

pub fn get_integration(key: &str) -> AppResult<Integration> {
    catalog()
        .into_iter()
        .find(|integration| integration.key == key)
        .ok_or_else(|| AppError::NotFound("Integration not found".to_string()))
}

fn catalog() -> Vec<Integration> {
    vec![
        integration(
            "webhook-destination",
            "Webhook Destination",
            "destinations",
            "available",
            "Send collected events to any HTTPS endpoint with retries, signatures, transforms, and delivery health.",
            &["destination", "event_routing", "transforms"],
            serde_json::json!({
                "routes": ["/api/v1/destinations", "/api/v1/destination-deliveries"],
                "module": "destinations"
            }),
        ),
        integration(
            "webhook-source",
            "Webhook Source",
            "sources",
            "available",
            "Receive external JSON events through source-specific tokens and route accepted events onward.",
            &["source", "webhook_ingest", "event_routing"],
            serde_json::json!({
                "routes": ["/api/v1/sources", "/api/source/{id}/collect"],
                "module": "sources"
            }),
        ),
        integration(
            "react",
            "React",
            "frameworks",
            "available",
            "Use PulseProvider and hooks for React applications without wiring a custom context.",
            &["sdk", "framework_helper", "browser_tracking"],
            serde_json::json!({
                "package": "@ayushojha/pulse-analytics/react",
                "exports": ["PulseProvider", "usePulse", "usePulsePageview", "usePulseEvent"]
            }),
        ),
        integration(
            "nextjs",
            "Next.js",
            "frameworks",
            "available",
            "Generate typed Next Script props and route tracking helpers for App Router or Pages Router projects.",
            &["sdk", "framework_helper", "script_tag"],
            serde_json::json!({
                "package": "@ayushojha/pulse-analytics/next",
                "exports": ["getPulseScriptProps", "createNextPulseClient", "trackNextPageview"]
            }),
        ),
        integration(
            "vue",
            "Vue",
            "frameworks",
            "available",
            "Use the Pulse Vue plugin and composition helpers for Vue 3 applications.",
            &["sdk", "framework_helper", "browser_tracking"],
            serde_json::json!({
                "package": "@ayushojha/pulse-analytics/vue",
                "exports": ["createPulseVue", "providePulse", "usePulse", "usePulsePageview", "usePulseEvent"]
            }),
        ),
        integration(
            "node-server",
            "Node.js Server",
            "server_sdks",
            "available",
            "Track server-side events, identify calls, logs, survey responses, and source webhooks from Node runtimes.",
            &["sdk", "server_tracking", "identity"],
            serde_json::json!({
                "package": "@ayushojha/pulse-analytics/server",
                "client": "PulseServerClient"
            }),
        ),
        integration(
            "react-native",
            "React Native",
            "mobile_sdks",
            "available",
            "Track mobile screens, events, identify calls, logs, and survey responses from React Native without browser APIs.",
            &["sdk", "mobile_sdk", "react_native"],
            serde_json::json!({
                "package": "@ayushojha/pulse-analytics/react-native",
                "client": "PulseNativeClient",
                "exports": ["createPulseNative", "PulseNativeClient"]
            }),
        ),
        integration(
            "csv-upload",
            "CSV Upload",
            "bi",
            "available",
            "Upload external tabular data into the BI layer and query it through governed saved SQL.",
            &["bi", "data_import", "csv"],
            serde_json::json!({
                "routes": ["/api/v1/bi/csv-uploads"],
                "module": "bi"
            }),
        ),
        integration(
            "google-ads",
            "Google Ads",
            "marketing",
            "available",
            "Import exported campaign spend, impressions, clicks, and conversions for paid-search ROI reporting.",
            &["marketing", "ads_import"],
            serde_json::json!({
                "routes": ["/api/v1/marketing/imports", "/api/v1/marketing/imports/summary"],
                "provider": "google_ads"
            }),
        ),
        integration(
            "search-console",
            "Google Search Console",
            "marketing",
            "available",
            "Import exported organic query and page performance for SEO analytics.",
            &["marketing", "search_import"],
            serde_json::json!({
                "routes": ["/api/v1/marketing/imports", "/api/v1/marketing/imports/summary"],
                "provider": "search_console"
            }),
        ),
        integration(
            "google-analytics",
            "Google Analytics",
            "marketing",
            "available",
            "Import exported GA4 session, user, conversion, and revenue rows for blended marketing reporting.",
            &["marketing", "ga_import"],
            serde_json::json!({
                "routes": ["/api/v1/marketing/imports", "/api/v1/marketing/imports/summary"],
                "provider": "google_analytics"
            }),
        ),
    ]
}

fn integration(
    key: &str,
    name: &str,
    category: &str,
    status: &str,
    description: &str,
    capabilities: &[&str],
    setup: serde_json::Value,
) -> Integration {
    Integration {
        key: key.to_string(),
        name: name.to_string(),
        category: category.to_string(),
        status: status.to_string(),
        description: description.to_string(),
        capabilities: capabilities
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        setup,
    }
}

#[cfg(test)]
mod tests {
    use super::{get_integration, list_integrations, IntegrationFilter};

    #[test]
    fn catalog_exposes_available_framework_helpers() {
        let integrations = list_integrations(IntegrationFilter {
            category: Some("frameworks".to_string()),
            capability: Some("framework_helper".to_string()),
            status: Some("available".to_string()),
        });

        assert_eq!(integrations.len(), 3);
        assert!(integrations
            .iter()
            .any(|integration| integration.key == "react"));
        assert!(integrations
            .iter()
            .any(|integration| integration.key == "nextjs"));
        assert!(integrations
            .iter()
            .any(|integration| integration.key == "vue"));
    }

    #[test]
    fn lookup_returns_not_found_for_unknown_key() {
        assert!(get_integration("webhook-source").is_ok());
        assert!(get_integration("react-native").is_ok());
        assert!(get_integration("missing").is_err());
    }
}
