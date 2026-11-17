use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{FromRow, PgPool};
use tokio::time;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

const MAX_DELIVERY_ATTEMPTS: i32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Destination {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub destination_type: String,
    pub endpoint_url: String,
    pub secret: Option<String>,
    pub headers: serde_json::Value,
    pub event_types: Vec<String>,
    pub transform: serde_json::Value,
    pub is_active: bool,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub failure_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DestinationDelivery {
    pub id: Uuid,
    pub project_id: Uuid,
    pub destination_id: Uuid,
    pub event_type: String,
    pub status: String,
    pub payload: serde_json::Value,
    pub attempts: i32,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub next_retry_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DestinationInput {
    pub name: String,
    #[serde(default = "default_destination_type")]
    pub destination_type: String,
    pub endpoint_url: String,
    pub secret: Option<String>,
    #[serde(default = "default_object")]
    pub headers: serde_json::Value,
    #[serde(default)]
    pub event_types: Vec<String>,
    #[serde(default = "default_object")]
    pub transform: serde_json::Value,
    #[serde(default = "default_active")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DestinationHealth {
    pub destination_id: Uuid,
    pub name: String,
    pub destination_type: String,
    pub is_active: bool,
    pub status: String,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub failure_count: i32,
    pub total_deliveries: i64,
    pub pending_deliveries: i64,
    pub retry_deliveries: i64,
    pub delivered_deliveries: i64,
    pub dead_letter_deliveries: i64,
}

#[derive(Debug, FromRow)]
struct DestinationRoute {
    id: Uuid,
    transform: serde_json::Value,
}

#[derive(Debug, FromRow)]
struct DeliveryJob {
    delivery_id: Uuid,
    destination_id: Uuid,
    event_type: String,
    payload: serde_json::Value,
    attempts: i32,
    endpoint_url: String,
    secret: Option<String>,
    headers: serde_json::Value,
}

#[derive(Debug, FromRow)]
struct HealthRow {
    destination_id: Uuid,
    name: String,
    destination_type: String,
    is_active: bool,
    last_success_at: Option<DateTime<Utc>>,
    last_failure_at: Option<DateTime<Utc>>,
    failure_count: i32,
    total_deliveries: i64,
    pending_deliveries: i64,
    retry_deliveries: i64,
    delivered_deliveries: i64,
    dead_letter_deliveries: i64,
}

fn default_destination_type() -> String {
    "webhook".to_string()
}

fn default_active() -> bool {
    true
}

fn default_object() -> serde_json::Value {
    serde_json::json!({})
}

pub fn start_destination_delivery_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticker = time::interval(std::time::Duration::from_secs(30));
        ticker.tick().await;

        loop {
            ticker.tick().await;
            if let Err(e) = dispatch_pending_deliveries(&state.db, 50).await {
                tracing::warn!("Destination delivery task failed: {e}");
            }
        }
    });
}

pub async fn list_destinations(db: &PgPool, project_id: Uuid) -> AppResult<Vec<Destination>> {
    let destinations = sqlx::query_as(
        "SELECT id, project_id, name, destination_type, endpoint_url, secret, headers, event_types, \
         transform, is_active, last_success_at, last_failure_at, failure_count, created_at, updated_at \
         FROM destinations WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(destinations)
}

pub async fn get_destination(
    db: &PgPool,
    project_id: Uuid,
    destination_id: Uuid,
) -> AppResult<Destination> {
    let destination = sqlx::query_as(
        "SELECT id, project_id, name, destination_type, endpoint_url, secret, headers, event_types, \
         transform, is_active, last_success_at, last_failure_at, failure_count, created_at, updated_at \
         FROM destinations WHERE id = $1 AND project_id = $2",
    )
    .bind(destination_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Destination not found".to_string()))?;
    Ok(destination)
}

pub async fn create_destination(
    db: &PgPool,
    project_id: Uuid,
    input: DestinationInput,
) -> AppResult<Destination> {
    let input = validate_input(input)?;
    let destination = sqlx::query_as(
        "INSERT INTO destinations \
         (project_id, name, destination_type, endpoint_url, secret, headers, event_types, transform, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id, project_id, name, destination_type, endpoint_url, secret, headers, event_types, \
         transform, is_active, last_success_at, last_failure_at, failure_count, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.destination_type)
    .bind(&input.endpoint_url)
    .bind(&input.secret)
    .bind(&input.headers)
    .bind(&input.event_types)
    .bind(&input.transform)
    .bind(input.is_active)
    .fetch_one(db)
    .await?;
    Ok(destination)
}

pub async fn update_destination(
    db: &PgPool,
    project_id: Uuid,
    destination_id: Uuid,
    input: DestinationInput,
) -> AppResult<Destination> {
    let input = validate_input(input)?;
    let destination = sqlx::query_as(
        "UPDATE destinations SET \
           name = $3, destination_type = $4, endpoint_url = $5, secret = $6, headers = $7, \
           event_types = $8, transform = $9, is_active = $10, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, destination_type, endpoint_url, secret, headers, event_types, \
         transform, is_active, last_success_at, last_failure_at, failure_count, created_at, updated_at",
    )
    .bind(destination_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.destination_type)
    .bind(&input.endpoint_url)
    .bind(&input.secret)
    .bind(&input.headers)
    .bind(&input.event_types)
    .bind(&input.transform)
    .bind(input.is_active)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Destination not found".to_string()))?;
    Ok(destination)
}

pub async fn delete_destination(
    db: &PgPool,
    project_id: Uuid,
    destination_id: Uuid,
) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM destinations WHERE id = $1 AND project_id = $2")
        .bind(destination_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Destination not found".to_string()));
    }
    Ok(())
}

pub async fn list_deliveries(
    db: &PgPool,
    project_id: Uuid,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<DestinationDelivery>> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);
    let deliveries = if let Some(status) = status {
        let status = validate_delivery_status(status)?;
        sqlx::query_as(
            "SELECT id, project_id, destination_id, event_type, status, payload, attempts, \
             response_status, response_body, error_message, next_retry_at, delivered_at, created_at, updated_at \
             FROM destination_deliveries \
             WHERE project_id = $1 AND status = $2 \
             ORDER BY created_at DESC LIMIT $3 OFFSET $4",
        )
        .bind(project_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, project_id, destination_id, event_type, status, payload, attempts, \
             response_status, response_body, error_message, next_retry_at, delivered_at, created_at, updated_at \
             FROM destination_deliveries \
             WHERE project_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };
    Ok(deliveries)
}

pub async fn retry_delivery(db: &PgPool, project_id: Uuid, delivery_id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE destination_deliveries SET \
           status = 'pending', attempts = 0, response_status = NULL, response_body = NULL, \
           error_message = NULL, next_retry_at = NOW(), updated_at = NOW() \
         WHERE id = $1 AND project_id = $2",
    )
    .bind(delivery_id)
    .bind(project_id)
    .execute(db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Destination delivery not found".to_string(),
        ));
    }
    Ok(())
}

pub async fn destination_health(
    db: &PgPool,
    project_id: Uuid,
) -> AppResult<Vec<DestinationHealth>> {
    let rows: Vec<HealthRow> = sqlx::query_as(
        "SELECT \
           d.id AS destination_id, d.name, d.destination_type, d.is_active, \
           d.last_success_at, d.last_failure_at, d.failure_count, \
           COUNT(dd.id)::bigint AS total_deliveries, \
           COUNT(dd.id) FILTER (WHERE dd.status = 'pending')::bigint AS pending_deliveries, \
           COUNT(dd.id) FILTER (WHERE dd.status = 'retry')::bigint AS retry_deliveries, \
           COUNT(dd.id) FILTER (WHERE dd.status = 'delivered')::bigint AS delivered_deliveries, \
           COUNT(dd.id) FILTER (WHERE dd.status = 'dead_letter')::bigint AS dead_letter_deliveries \
         FROM destinations d \
         LEFT JOIN destination_deliveries dd ON dd.destination_id = d.id \
         WHERE d.project_id = $1 \
         GROUP BY d.id, d.name, d.destination_type, d.is_active, d.last_success_at, \
                  d.last_failure_at, d.failure_count, d.created_at \
         ORDER BY d.created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let status = destination_status(&row);
            DestinationHealth {
                destination_id: row.destination_id,
                name: row.name,
                destination_type: row.destination_type,
                is_active: row.is_active,
                status,
                last_success_at: row.last_success_at,
                last_failure_at: row.last_failure_at,
                failure_count: row.failure_count,
                total_deliveries: row.total_deliveries,
                pending_deliveries: row.pending_deliveries,
                retry_deliveries: row.retry_deliveries,
                delivered_deliveries: row.delivered_deliveries,
                dead_letter_deliveries: row.dead_letter_deliveries,
            }
        })
        .collect())
}

pub async fn enqueue_event(
    db: &PgPool,
    project_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
) -> AppResult<usize> {
    let destinations: Vec<DestinationRoute> = sqlx::query_as(
        "SELECT id, transform FROM destinations \
         WHERE project_id = $1 AND is_active = true \
           AND (cardinality(event_types) = 0 OR $2 = ANY(event_types))",
    )
    .bind(project_id)
    .bind(event_type)
    .fetch_all(db)
    .await?;

    for destination in &destinations {
        let payload = apply_transform(&payload, &destination.transform)?;
        sqlx::query(
            "INSERT INTO destination_deliveries \
             (project_id, destination_id, event_type, payload) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(project_id)
        .bind(destination.id)
        .bind(event_type)
        .bind(&payload)
        .execute(db)
        .await?;
    }

    Ok(destinations.len())
}

pub async fn dispatch_pending_deliveries(db: &PgPool, limit: i64) -> AppResult<usize> {
    let jobs: Vec<DeliveryJob> = sqlx::query_as(
        "SELECT dd.id AS delivery_id, dd.destination_id, dd.event_type, \
                dd.payload, dd.attempts, d.endpoint_url, d.secret, d.headers \
         FROM destination_deliveries dd \
         JOIN destinations d ON d.id = dd.destination_id \
         WHERE dd.status IN ('pending', 'retry') \
           AND dd.next_retry_at <= NOW() \
           AND d.is_active = true \
         ORDER BY dd.created_at ASC \
         LIMIT $1",
    )
    .bind(limit.clamp(1, 100))
    .fetch_all(db)
    .await?;

    let client = reqwest::Client::new();
    let mut delivered = 0;
    for job in jobs {
        match send_delivery(&client, &job).await {
            Ok((status, body)) if (200..300).contains(&status) => {
                mark_delivery_success(db, &job, status, body).await?;
                delivered += 1;
            }
            Ok((status, body)) => {
                mark_delivery_failure(
                    db,
                    &job,
                    Some(status),
                    Some(body),
                    format!("Destination returned HTTP {status}"),
                )
                .await?;
            }
            Err(err) => {
                mark_delivery_failure(db, &job, None, None, err.to_string()).await?;
            }
        }
    }

    Ok(delivered)
}

async fn send_delivery(
    client: &reqwest::Client,
    job: &DeliveryJob,
) -> Result<(i32, String), reqwest::Error> {
    let body = serde_json::to_string(&job.payload).unwrap_or_else(|_| "{}".to_string());
    let mut request = client
        .post(&job.endpoint_url)
        .header("Content-Type", "application/json")
        .header("X-Pulse-Event", &job.event_type)
        .header("X-Pulse-Delivery", job.delivery_id.to_string())
        .timeout(std::time::Duration::from_secs(8));

    if let Some(headers) = job.headers.as_object() {
        for (key, value) in headers {
            if let Some(value) = value.as_str() {
                request = request.header(key, value);
            }
        }
    }

    if let Some(secret) = &job.secret {
        if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
            mac.update(body.as_bytes());
            let signature = hex::encode(mac.finalize().into_bytes());
            request = request.header("X-Pulse-Signature", signature);
        }
    }

    let response = request.body(body).send().await?;
    let status = response.status().as_u16() as i32;
    let body = response.text().await.unwrap_or_default();
    Ok((status, truncate(&body, 4096)))
}

async fn mark_delivery_success(
    db: &PgPool,
    job: &DeliveryJob,
    response_status: i32,
    response_body: String,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE destination_deliveries SET \
           status = 'delivered', attempts = attempts + 1, response_status = $2, response_body = $3, \
           error_message = NULL, delivered_at = NOW(), updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(job.delivery_id)
    .bind(response_status)
    .bind(response_body)
    .execute(db)
    .await?;

    sqlx::query(
        "UPDATE destinations SET last_success_at = NOW(), failure_count = 0, updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(job.destination_id)
    .execute(db)
    .await?;
    Ok(())
}

async fn mark_delivery_failure(
    db: &PgPool,
    job: &DeliveryJob,
    response_status: Option<i32>,
    response_body: Option<String>,
    error_message: String,
) -> AppResult<()> {
    let attempts = job.attempts + 1;
    let dead_letter = attempts >= MAX_DELIVERY_ATTEMPTS;
    let status = if dead_letter { "dead_letter" } else { "retry" };
    let delay = 60 * 2_i64.pow((attempts - 1).clamp(0, 5) as u32);
    let next_retry_at = if dead_letter {
        Utc::now()
    } else {
        Utc::now() + Duration::seconds(delay)
    };

    sqlx::query(
        "UPDATE destination_deliveries SET \
           status = $2, attempts = $3, response_status = $4, response_body = $5, \
           error_message = $6, next_retry_at = $7, updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(job.delivery_id)
    .bind(status)
    .bind(attempts)
    .bind(response_status)
    .bind(response_body)
    .bind(truncate(&error_message, 1024))
    .bind(next_retry_at)
    .execute(db)
    .await?;

    sqlx::query(
        "UPDATE destinations SET last_failure_at = NOW(), failure_count = failure_count + 1, updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(job.destination_id)
    .execute(db)
    .await?;
    Ok(())
}

fn validate_input(mut input: DestinationInput) -> AppResult<DestinationInput> {
    input.name = input.name.trim().to_string();
    input.destination_type = input.destination_type.trim().to_string();
    input.endpoint_url = input.endpoint_url.trim().to_string();
    input.event_types = normalize_event_types(input.event_types)?;

    if input.name.is_empty() {
        return Err(AppError::BadRequest(
            "Destination name cannot be empty".to_string(),
        ));
    }
    if input.destination_type != "webhook" {
        return Err(AppError::BadRequest(
            "Only webhook destinations are currently supported".to_string(),
        ));
    }
    if !input.endpoint_url.starts_with("https://") && !input.endpoint_url.starts_with("http://") {
        return Err(AppError::BadRequest(
            "endpoint_url must start with http:// or https://".to_string(),
        ));
    }
    crate::services::ssrf::ensure_public_http_url(&input.endpoint_url)
        .map_err(|reason| AppError::BadRequest(format!("endpoint_url rejected: {reason}")))?;
    if !input.headers.is_object() {
        return Err(AppError::BadRequest(
            "headers must be an object".to_string(),
        ));
    }
    if !input.transform.is_object() {
        return Err(AppError::BadRequest(
            "transform must be an object".to_string(),
        ));
    }
    apply_transform(&serde_json::json!({}), &input.transform)?;
    Ok(input)
}

fn normalize_event_types(values: Vec<String>) -> AppResult<Vec<String>> {
    if values.len() > 50 {
        return Err(AppError::BadRequest(
            "event_types supports at most 50 entries".to_string(),
        ));
    }
    Ok(values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect())
}

fn validate_delivery_status(status: &str) -> AppResult<&'static str> {
    match status.trim() {
        "pending" => Ok("pending"),
        "retry" => Ok("retry"),
        "delivered" => Ok("delivered"),
        "dead_letter" => Ok("dead_letter"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported delivery status: {other}"
        ))),
    }
}

fn destination_status(row: &HealthRow) -> String {
    if !row.is_active {
        "disabled".to_string()
    } else if row.dead_letter_deliveries > 0 {
        "degraded".to_string()
    } else if row.last_failure_at > row.last_success_at {
        "failing".to_string()
    } else {
        "healthy".to_string()
    }
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn apply_transform(
    payload: &serde_json::Value,
    transform: &serde_json::Value,
) -> AppResult<serde_json::Value> {
    let Some(transform) = transform.as_object() else {
        return Err(AppError::BadRequest(
            "transform must be an object".to_string(),
        ));
    };
    if transform.is_empty() {
        return Ok(payload.clone());
    }

    let mut output = if let Some(include) = string_list(transform, &["include", "include_fields"])?
    {
        include_paths(payload, &include)
    } else {
        payload.clone()
    };

    if let Some(exclude) = string_list(transform, &["exclude", "drop_fields"])? {
        for path in exclude {
            take_path(&mut output, &path);
        }
    }

    if let Some(rename) = object_field(transform, &["rename", "rename_fields"])? {
        for (from, to) in rename {
            let to = to.as_str().ok_or_else(|| {
                AppError::BadRequest("transform rename values must be strings".to_string())
            })?;
            if let Some(value) = take_path(&mut output, from) {
                set_path(&mut output, to, value);
            }
        }
    }

    if let Some(set) = object_field(transform, &["set", "static_fields"])? {
        for (path, value) in set {
            set_path(&mut output, path, value.clone());
        }
    }

    if let Some(wrap) = string_field(transform, &["wrap", "wrap_key"])? {
        let mut wrapped = serde_json::Map::new();
        wrapped.insert(wrap, output);
        output = serde_json::Value::Object(wrapped);
    }

    Ok(output)
}

fn string_field(
    transform: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> AppResult<Option<String>> {
    for key in keys {
        if let Some(value) = transform.get(*key) {
            let value = value
                .as_str()
                .ok_or_else(|| AppError::BadRequest(format!("transform {key} must be a string")))?;
            let value = value.trim();
            if value.is_empty() {
                return Err(AppError::BadRequest(format!(
                    "transform {key} cannot be empty"
                )));
            }
            return Ok(Some(value.to_string()));
        }
    }
    Ok(None)
}

fn string_list(
    transform: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> AppResult<Option<Vec<String>>> {
    for key in keys {
        if let Some(value) = transform.get(*key) {
            let values = value
                .as_array()
                .ok_or_else(|| AppError::BadRequest(format!("transform {key} must be an array")))?;
            let mut paths = Vec::new();
            for value in values {
                let value = value.as_str().ok_or_else(|| {
                    AppError::BadRequest(format!("transform {key} entries must be strings"))
                })?;
                let value = value.trim();
                if !value.is_empty() {
                    paths.push(value.to_string());
                }
            }
            return Ok(Some(paths));
        }
    }
    Ok(None)
}

fn object_field<'a>(
    transform: &'a serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> AppResult<Option<&'a serde_json::Map<String, serde_json::Value>>> {
    for key in keys {
        if let Some(value) = transform.get(*key) {
            let object = value.as_object().ok_or_else(|| {
                AppError::BadRequest(format!("transform {key} must be an object"))
            })?;
            return Ok(Some(object));
        }
    }
    Ok(None)
}

fn include_paths(payload: &serde_json::Value, paths: &[String]) -> serde_json::Value {
    let mut output = serde_json::json!({});
    for path in paths {
        if let Some(value) = get_path(payload, path) {
            set_path(&mut output, path, value.clone());
        }
    }
    output
}

fn get_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in path_parts(path) {
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}

fn take_path(value: &mut serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let parts = path_parts(path);
    if parts.is_empty() {
        return None;
    }
    take_path_parts(value, &parts)
}

fn take_path_parts(value: &mut serde_json::Value, parts: &[&str]) -> Option<serde_json::Value> {
    let object = value.as_object_mut()?;
    if parts.len() == 1 {
        object.remove(parts[0])
    } else {
        let next = object.get_mut(parts[0])?;
        take_path_parts(next, &parts[1..])
    }
}

fn set_path(value: &mut serde_json::Value, path: &str, new_value: serde_json::Value) {
    let parts = path_parts(path);
    if parts.is_empty() {
        return;
    }

    let mut current = value;
    for part in &parts[..parts.len() - 1] {
        let object = ensure_object(current);
        current = object
            .entry((*part).to_string())
            .or_insert_with(|| serde_json::json!({}));
    }
    ensure_object(current).insert(parts[parts.len() - 1].to_string(), new_value);
}

fn ensure_object(value: &mut serde_json::Value) -> &mut serde_json::Map<String, serde_json::Value> {
    if !value.is_object() {
        *value = serde_json::json!({});
    }
    value
        .as_object_mut()
        .expect("value was converted to object")
}

fn path_parts(path: &str) -> Vec<&str> {
    path.split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_transform, normalize_event_types, validate_delivery_status, validate_input,
        DestinationInput,
    };
    use serde_json::json;

    fn valid_input() -> DestinationInput {
        DestinationInput {
            name: "Warehouse Hook".to_string(),
            destination_type: "webhook".to_string(),
            endpoint_url: "https://example.com/events".to_string(),
            secret: None,
            headers: json!({ "X-Team": "analytics" }),
            event_types: vec![
                " event ".to_string(),
                "".to_string(),
                "pageview".to_string(),
            ],
            transform: json!({}),
            is_active: true,
        }
    }

    #[test]
    fn validates_webhook_destinations() {
        let input = validate_input(valid_input()).unwrap();
        assert_eq!(input.event_types, vec!["event", "pageview"]);
    }

    #[test]
    fn rejects_invalid_destination_url() {
        let mut input = valid_input();
        input.endpoint_url = "ftp://example.com/events".to_string();
        assert!(validate_input(input).is_err());
    }

    #[test]
    fn validates_delivery_statuses() {
        assert!(validate_delivery_status("pending").is_ok());
        assert!(validate_delivery_status("dead_letter").is_ok());
        assert!(validate_delivery_status("unknown").is_err());
    }

    #[test]
    fn limits_event_type_filters() {
        let values = (0..51).map(|i| format!("event_{i}")).collect();
        assert!(normalize_event_types(values).is_err());
    }

    #[test]
    fn applies_destination_transform_rules() {
        let payload = json!({
            "event_type": "checkout.session.completed",
            "payload": {
                "customer_id": "cus_123",
                "amount": 4900,
                "card_token": "tok_secret"
            },
            "source": { "type": "stripe.webhook" }
        });
        let transform = json!({
            "include": ["event_type", "payload.customer_id", "payload.amount"],
            "rename": { "payload.customer_id": "customer.id" },
            "drop_fields": ["payload.card_token"],
            "static_fields": { "platform": "pulse" },
            "wrap": "data"
        });

        let transformed = apply_transform(&payload, &transform).unwrap();

        assert_eq!(
            transformed,
            json!({
                "data": {
                    "event_type": "checkout.session.completed",
                    "payload": { "amount": 4900 },
                    "customer": { "id": "cus_123" },
                    "platform": "pulse"
                }
            })
        );
    }

    #[test]
    fn rejects_invalid_destination_transform_rules() {
        let mut input = valid_input();
        input.transform = json!({ "include": "event_type" });
        assert!(validate_input(input).is_err());

        let mut input = valid_input();
        input.transform = json!({ "rename": { "a": 42 } });
        assert!(validate_input(input).is_err());
    }
}
