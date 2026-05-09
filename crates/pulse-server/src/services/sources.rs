use chrono::{DateTime, Utc};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::{destinations, modules};
use crate::state::SharedState;

const DEFAULT_EVENT_TYPE: &str = "webhook_event";

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventSource {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub source_type: String,
    pub description: Option<String>,
    pub token_prefix: String,
    pub schema: serde_json::Value,
    pub config: serde_json::Value,
    pub is_active: bool,
    pub last_received_at: Option<DateTime<Utc>>,
    pub failure_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SourceIngestion {
    pub id: Uuid,
    pub project_id: Uuid,
    pub source_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub headers: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub destination_deliveries: i32,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceInput {
    pub name: String,
    #[serde(default = "default_source_type")]
    pub source_type: String,
    pub description: Option<String>,
    #[serde(default = "default_object")]
    pub schema: serde_json::Value,
    #[serde(default = "default_object")]
    pub config: serde_json::Value,
    #[serde(default = "default_active")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceWithToken {
    pub source: EventSource,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceIngestResponse {
    pub ok: bool,
    pub ingestion_id: Uuid,
    pub event_type: String,
    pub destination_deliveries: usize,
}

#[derive(Debug, FromRow)]
struct EventSourceSecret {
    id: Uuid,
    project_id: Uuid,
    name: String,
    source_type: String,
    description: Option<String>,
    token_hash: String,
    token_prefix: String,
    schema: serde_json::Value,
    config: serde_json::Value,
    is_active: bool,
    last_received_at: Option<DateTime<Utc>>,
    failure_count: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EventSourceSecret> for EventSource {
    fn from(source: EventSourceSecret) -> Self {
        Self {
            id: source.id,
            project_id: source.project_id,
            name: source.name,
            source_type: source.source_type,
            description: source.description,
            token_prefix: source.token_prefix,
            schema: source.schema,
            config: source.config,
            is_active: source.is_active,
            last_received_at: source.last_received_at,
            failure_count: source.failure_count,
            created_at: source.created_at,
            updated_at: source.updated_at,
        }
    }
}

fn default_source_type() -> String {
    "webhook".to_string()
}

fn default_active() -> bool {
    true
}

fn default_object() -> serde_json::Value {
    serde_json::json!({})
}

fn generate_source_token() -> String {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &random_bytes,
    );
    format!("psrc_{encoded}")
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn token_prefix(token: &str) -> String {
    token.chars().take(16).collect()
}

pub async fn list_sources(db: &PgPool, project_id: Uuid) -> AppResult<Vec<EventSource>> {
    let sources = sqlx::query_as(
        "SELECT id, project_id, name, source_type, description, token_prefix, schema, config, \
         is_active, last_received_at, failure_count, created_at, updated_at \
         FROM event_sources WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(sources)
}

pub async fn get_source(db: &PgPool, project_id: Uuid, source_id: Uuid) -> AppResult<EventSource> {
    let source = sqlx::query_as(
        "SELECT id, project_id, name, source_type, description, token_prefix, schema, config, \
         is_active, last_received_at, failure_count, created_at, updated_at \
         FROM event_sources WHERE id = $1 AND project_id = $2",
    )
    .bind(source_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Event source not found".to_string()))?;
    Ok(source)
}

pub async fn create_source(
    db: &PgPool,
    project_id: Uuid,
    input: SourceInput,
) -> AppResult<SourceWithToken> {
    let input = validate_source_input(input)?;
    let token = generate_source_token();
    let source = sqlx::query_as(
        "INSERT INTO event_sources \
         (project_id, name, source_type, description, token_hash, token_prefix, schema, config, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id, project_id, name, source_type, description, token_prefix, schema, config, \
         is_active, last_received_at, failure_count, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.source_type)
    .bind(&input.description)
    .bind(hash_token(&token))
    .bind(token_prefix(&token))
    .bind(&input.schema)
    .bind(&input.config)
    .bind(input.is_active)
    .fetch_one(db)
    .await?;

    Ok(SourceWithToken { source, token })
}

pub async fn update_source(
    db: &PgPool,
    project_id: Uuid,
    source_id: Uuid,
    input: SourceInput,
) -> AppResult<EventSource> {
    let input = validate_source_input(input)?;
    let source = sqlx::query_as(
        "UPDATE event_sources SET \
           name = $3, source_type = $4, description = $5, schema = $6, config = $7, \
           is_active = $8, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, source_type, description, token_prefix, schema, config, \
         is_active, last_received_at, failure_count, created_at, updated_at",
    )
    .bind(source_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.source_type)
    .bind(&input.description)
    .bind(&input.schema)
    .bind(&input.config)
    .bind(input.is_active)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Event source not found".to_string()))?;
    Ok(source)
}

pub async fn delete_source(db: &PgPool, project_id: Uuid, source_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM event_sources WHERE id = $1 AND project_id = $2")
        .bind(source_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Event source not found".to_string()));
    }
    Ok(())
}

pub async fn list_ingestions(
    db: &PgPool,
    project_id: Uuid,
    source_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<SourceIngestion>> {
    let ingestions = sqlx::query_as(
        "SELECT id, project_id, source_id, event_type, payload, headers, status, error_message, \
         destination_deliveries, received_at \
         FROM source_ingestions \
         WHERE project_id = $1 AND source_id = $2 \
         ORDER BY received_at DESC LIMIT $3 OFFSET $4",
    )
    .bind(project_id)
    .bind(source_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(ingestions)
}

pub async fn ingest_source_payload(
    state: &SharedState,
    source_id: Uuid,
    token: &str,
    payload: serde_json::Value,
    headers: serde_json::Value,
) -> AppResult<SourceIngestResponse> {
    let source = verify_source_token(&state.db, source_id, token).await?;
    if !modules::is_module_enabled(state, source.project_id, "sources").await? {
        return Err(AppError::Forbidden(
            "Module 'sources' is not enabled for this project".to_string(),
        ));
    }

    let event_type = event_type_from_payload(&payload);
    let destination_payload = build_destination_payload(&source, &event_type, payload.clone());
    let destination_deliveries = destinations::enqueue_event(
        &state.db,
        source.project_id,
        &event_type,
        destination_payload,
    )
    .await?;

    let ingestion = record_ingestion(
        &state.db,
        source.project_id,
        source.id,
        &event_type,
        payload,
        headers,
        destination_deliveries as i32,
    )
    .await?;

    mark_source_received(&state.db, source.id).await?;

    Ok(SourceIngestResponse {
        ok: true,
        ingestion_id: ingestion.id,
        event_type,
        destination_deliveries,
    })
}

async fn verify_source_token(db: &PgPool, source_id: Uuid, token: &str) -> AppResult<EventSource> {
    if token.trim().is_empty() {
        return Err(AppError::Unauthorized);
    }

    let source: Option<EventSourceSecret> = sqlx::query_as(
        "SELECT id, project_id, name, source_type, description, token_hash, token_prefix, schema, \
         config, is_active, last_received_at, failure_count, created_at, updated_at \
         FROM event_sources WHERE id = $1 AND is_active = true",
    )
    .bind(source_id)
    .fetch_optional(db)
    .await?;

    let source = source.ok_or(AppError::Unauthorized)?;
    if source.token_hash != hash_token(token.trim()) {
        return Err(AppError::Unauthorized);
    }
    Ok(source.into())
}

async fn record_ingestion(
    db: &PgPool,
    project_id: Uuid,
    source_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
    headers: serde_json::Value,
    destination_deliveries: i32,
) -> AppResult<SourceIngestion> {
    let ingestion = sqlx::query_as(
        "INSERT INTO source_ingestions \
         (project_id, source_id, event_type, payload, headers, destination_deliveries) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, project_id, source_id, event_type, payload, headers, status, error_message, \
         destination_deliveries, received_at",
    )
    .bind(project_id)
    .bind(source_id)
    .bind(event_type)
    .bind(&payload)
    .bind(&headers)
    .bind(destination_deliveries)
    .fetch_one(db)
    .await?;
    Ok(ingestion)
}

async fn mark_source_received(db: &PgPool, source_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE event_sources SET last_received_at = NOW(), failure_count = 0, updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(source_id)
    .execute(db)
    .await?;
    Ok(())
}

fn build_destination_payload(
    source: &EventSource,
    event_type: &str,
    payload: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "event_type": event_type,
        "source": {
            "id": source.id,
            "name": source.name,
            "type": source.source_type,
        },
        "payload": payload,
        "received_at": Utc::now(),
    })
}

pub fn event_type_from_payload(payload: &serde_json::Value) -> String {
    let raw = payload
        .get("event_type")
        .or_else(|| payload.get("type"))
        .or_else(|| payload.get("event"))
        .or_else(|| payload.get("name"))
        .and_then(|value| value.as_str())
        .unwrap_or(DEFAULT_EVENT_TYPE);
    normalize_event_type(raw)
}

fn normalize_event_type(raw: &str) -> String {
    let mut normalized = String::with_capacity(raw.len().min(128));
    let mut previous_was_separator = false;

    for ch in raw.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':') {
            Some(ch.to_ascii_lowercase())
        } else if ch.is_ascii_whitespace() || matches!(ch, '/' | '\\') {
            Some('_')
        } else {
            None
        };

        if let Some(ch) = next {
            if ch == '_' {
                if previous_was_separator || normalized.is_empty() {
                    continue;
                }
                previous_was_separator = true;
            } else {
                previous_was_separator = false;
            }
            normalized.push(ch);
            if normalized.len() >= 128 {
                break;
            }
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        DEFAULT_EVENT_TYPE.to_string()
    } else {
        normalized
    }
}

fn validate_source_input(mut input: SourceInput) -> AppResult<SourceInput> {
    input.name = input.name.trim().to_string();
    if input.name.is_empty() {
        return Err(AppError::BadRequest("Source name is required".to_string()));
    }
    if input.name.len() > 255 {
        return Err(AppError::BadRequest(
            "Source name must be 255 characters or fewer".to_string(),
        ));
    }

    input.source_type = input.source_type.trim().to_ascii_lowercase();
    if input.source_type.is_empty() {
        input.source_type = default_source_type();
    }
    if input.source_type.len() > 64
        || !input.source_type.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '_' | '-' | '.')
        })
    {
        return Err(AppError::BadRequest(
            "Source type may only contain lowercase letters, numbers, '.', '_', or '-'".to_string(),
        ));
    }

    input.description = input
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());

    input.schema = validate_object(input.schema, "schema")?;
    input.config = validate_object(input.config, "config")?;
    Ok(input)
}

fn validate_object(value: serde_json::Value, field: &str) -> AppResult<serde_json::Value> {
    if value.is_null() {
        Ok(default_object())
    } else if value.is_object() {
        Ok(value)
    } else {
        Err(AppError::BadRequest(format!(
            "Source {field} must be a JSON object"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        event_type_from_payload, generate_source_token, hash_token, token_prefix,
        validate_source_input, SourceInput,
    };

    #[test]
    fn generated_tokens_have_hashable_secret_and_prefix() {
        let token = generate_source_token();

        assert!(token.starts_with("psrc_"));
        assert!(token.len() > 32);
        assert_eq!(hash_token(&token).len(), 64);
        assert_eq!(
            token_prefix(&token),
            token.chars().take(16).collect::<String>()
        );
    }

    #[test]
    fn event_type_prefers_explicit_fields_and_normalizes() {
        let payload = serde_json::json!({
            "event_type": "Stripe/Checkout Session.Completed",
            "type": "ignored"
        });

        assert_eq!(
            event_type_from_payload(&payload),
            "stripe_checkout_session.completed"
        );
    }

    #[test]
    fn event_type_falls_back_for_missing_or_invalid_values() {
        assert_eq!(
            event_type_from_payload(&serde_json::json!({ "event_type": " !!! " })),
            "webhook_event"
        );
        assert_eq!(
            event_type_from_payload(&serde_json::json!({ "name": "Lead Created" })),
            "lead_created"
        );
    }

    #[test]
    fn source_input_validation_trims_and_rejects_bad_shapes() {
        let input = validate_source_input(SourceInput {
            name: "  Stripe  ".to_string(),
            source_type: "Stripe.Webhook".to_string(),
            description: Some("  Billing events  ".to_string()),
            schema: serde_json::json!({ "required": ["id"] }),
            config: serde_json::json!({}),
            is_active: true,
        })
        .unwrap();

        assert_eq!(input.name, "Stripe");
        assert_eq!(input.source_type, "stripe.webhook");
        assert_eq!(input.description.as_deref(), Some("Billing events"));

        let invalid = validate_source_input(SourceInput {
            name: "Broken".to_string(),
            source_type: "webhook".to_string(),
            description: None,
            schema: serde_json::json!([]),
            config: serde_json::json!({}),
            is_active: true,
        });
        assert!(invalid.is_err());
    }
}
