use std::collections::HashSet;
use std::net::IpAddr;
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration, Utc};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::product_analytics;

const BI_EMBED_COLUMNS: &str =
    "id, project_id, name, resource_type, resource_id, resource_config, \
    allowed_origins, theme, token_prefix, is_active, expires_at, last_accessed_at, \
    access_count, created_by, created_at, updated_at";
const SUPPORTED_BI_DATABASE_TYPES: &[&str] = &["postgres", "clickhouse", "http_json"];

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SemanticMetric {
    pub id: Uuid,
    pub project_id: Uuid,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub dataset: String,
    pub expression: String,
    pub filters: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedSqlQuery {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub sql_text: String,
    pub parameters: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BiQueryRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub query_id: Option<Uuid>,
    pub query_type: String,
    pub sql_text: String,
    pub result: serde_json::Value,
    pub row_count: i32,
    pub duration_ms: i32,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CsvUpload {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub columns: serde_json::Value,
    pub row_count: i32,
    pub uploaded_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BiRowPolicy {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub dataset: String,
    pub field: String,
    pub operator: String,
    pub values: serde_json::Value,
    pub is_active: bool,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiDatabaseConnection {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub database_type: String,
    pub connection_string_masked: String,
    pub allowed_schemas: serde_json::Value,
    pub is_active: bool,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct BiDatabaseConnectionRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub database_type: String,
    pub connection_string: String,
    pub allowed_schemas: serde_json::Value,
    pub is_active: bool,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BiDatabaseConnectionRow> for BiDatabaseConnection {
    fn from(row: BiDatabaseConnectionRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            database_type: row.database_type,
            connection_string_masked: mask_connection_string(&row.connection_string),
            allowed_schemas: row.allowed_schemas,
            is_active: row.is_active,
            last_tested_at: row.last_tested_at,
            last_error: row.last_error,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiEmbed {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub resource_config: serde_json::Value,
    pub allowed_origins: serde_json::Value,
    pub theme: serde_json::Value,
    pub token_prefix: String,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub access_count: i64,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct BiEmbedRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub resource_config: serde_json::Value,
    pub allowed_origins: serde_json::Value,
    pub theme: serde_json::Value,
    pub token_prefix: String,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub access_count: i64,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BiEmbedRow> for BiEmbed {
    fn from(row: BiEmbedRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            resource_config: row.resource_config,
            allowed_origins: row.allowed_origins,
            theme: row.theme,
            token_prefix: row.token_prefix,
            is_active: row.is_active,
            expires_at: row.expires_at,
            last_accessed_at: row.last_accessed_at,
            access_count: row.access_count,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SemanticMetricInput {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub dataset: String,
    pub expression: String,
    #[serde(default = "empty_object")]
    pub filters: serde_json::Value,
    #[serde(default = "default_active")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SavedSqlInput {
    pub name: String,
    pub description: Option<String>,
    pub sql_text: String,
    #[serde(default = "empty_object")]
    pub parameters: serde_json::Value,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SqlRunRequest {
    pub sql_text: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VisualQueryRequest {
    pub dataset: String,
    #[serde(default)]
    pub dimensions: Vec<String>,
    #[serde(default)]
    pub metrics: Vec<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DrillThroughRequest {
    pub dataset: String,
    #[serde(default = "empty_object")]
    pub filters: serde_json::Value,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CsvUploadInput {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub rows: Vec<serde_json::Value>,
    pub uploaded_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BiRowPolicyInput {
    pub name: String,
    pub dataset: String,
    pub field: String,
    #[serde(default = "default_policy_operator")]
    pub operator: String,
    #[serde(default = "empty_array")]
    pub values: serde_json::Value,
    #[serde(default = "default_active")]
    pub is_active: bool,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BiDatabaseConnectionInput {
    pub name: String,
    #[serde(default = "default_database_type")]
    pub database_type: String,
    pub connection_string: String,
    #[serde(default = "default_allowed_schemas")]
    pub allowed_schemas: serde_json::Value,
    #[serde(default = "default_active")]
    pub is_active: bool,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalSqlRunRequest {
    pub sql_text: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BiEmbedInput {
    pub name: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    #[serde(default = "empty_object")]
    pub resource_config: serde_json::Value,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(default = "empty_object")]
    pub theme: serde_json::Value,
    #[serde(default = "default_active")]
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BiQueryResponse {
    pub run: BiQueryRun,
    pub rows: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BiEmbedWithToken {
    pub embed: BiEmbed,
    pub token: String,
    pub embed_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BiEmbedResolved {
    pub embed: BiEmbed,
    pub resource: serde_json::Value,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BiConnectionTestResponse {
    pub connection: BiDatabaseConnection,
    pub ok: bool,
    pub error: Option<String>,
}

fn empty_object() -> serde_json::Value {
    json!({})
}

fn empty_array() -> serde_json::Value {
    json!([])
}

fn default_active() -> bool {
    true
}

fn default_policy_operator() -> String {
    "eq".to_string()
}

fn default_database_type() -> String {
    "postgres".to_string()
}

fn default_allowed_schemas() -> serde_json::Value {
    json!(["public"])
}

pub async fn list_metrics(db: &PgPool, project_id: Uuid) -> AppResult<Vec<SemanticMetric>> {
    let metrics = sqlx::query_as(
        "SELECT id, project_id, key, name, description, dataset, expression, filters, is_active, created_at, updated_at \
         FROM semantic_metrics WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(metrics)
}

pub async fn get_metric(
    db: &PgPool,
    project_id: Uuid,
    metric_id: Uuid,
) -> AppResult<SemanticMetric> {
    sqlx::query_as(
        "SELECT id, project_id, key, name, description, dataset, expression, filters, is_active, created_at, updated_at \
         FROM semantic_metrics WHERE id = $1 AND project_id = $2",
    )
    .bind(metric_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Semantic metric not found".to_string()))
}

pub async fn create_metric(
    db: &PgPool,
    project_id: Uuid,
    input: SemanticMetricInput,
) -> AppResult<SemanticMetric> {
    let input = validate_metric_input(input)?;
    let metric = sqlx::query_as(
        "INSERT INTO semantic_metrics (project_id, key, name, description, dataset, expression, filters, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id, project_id, key, name, description, dataset, expression, filters, is_active, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.key)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.dataset)
    .bind(&input.expression)
    .bind(&input.filters)
    .bind(input.is_active)
    .fetch_one(db)
    .await?;
    Ok(metric)
}

pub async fn update_metric(
    db: &PgPool,
    project_id: Uuid,
    metric_id: Uuid,
    input: SemanticMetricInput,
) -> AppResult<SemanticMetric> {
    let input = validate_metric_input(input)?;
    let metric = sqlx::query_as(
        "UPDATE semantic_metrics SET key = $3, name = $4, description = $5, dataset = $6, \
           expression = $7, filters = $8, is_active = $9, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, key, name, description, dataset, expression, filters, is_active, created_at, updated_at",
    )
    .bind(metric_id)
    .bind(project_id)
    .bind(&input.key)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.dataset)
    .bind(&input.expression)
    .bind(&input.filters)
    .bind(input.is_active)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Semantic metric not found".to_string()))?;
    Ok(metric)
}

pub async fn delete_metric(db: &PgPool, project_id: Uuid, metric_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM semantic_metrics WHERE id = $1 AND project_id = $2")
        .bind(metric_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Semantic metric not found".to_string()));
    }
    Ok(())
}

pub async fn list_row_policies(db: &PgPool, project_id: Uuid) -> AppResult<Vec<BiRowPolicy>> {
    let policies = sqlx::query_as(
        "SELECT id, project_id, name, dataset, field, operator, values, is_active, created_by, created_at, updated_at \
         FROM bi_row_policies WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(policies)
}

pub async fn create_row_policy(
    db: &PgPool,
    project_id: Uuid,
    input: BiRowPolicyInput,
) -> AppResult<BiRowPolicy> {
    let input = validate_row_policy_input(input)?;
    let policy = sqlx::query_as(
        "INSERT INTO bi_row_policies (project_id, name, dataset, field, operator, values, is_active, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id, project_id, name, dataset, field, operator, values, is_active, created_by, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.dataset)
    .bind(&input.field)
    .bind(&input.operator)
    .bind(&input.values)
    .bind(input.is_active)
    .bind(&input.created_by)
    .fetch_one(db)
    .await?;
    Ok(policy)
}

pub async fn update_row_policy(
    db: &PgPool,
    project_id: Uuid,
    policy_id: Uuid,
    input: BiRowPolicyInput,
) -> AppResult<BiRowPolicy> {
    let input = validate_row_policy_input(input)?;
    let policy = sqlx::query_as(
        "UPDATE bi_row_policies SET name = $3, dataset = $4, field = $5, operator = $6, \
           values = $7, is_active = $8, created_by = $9, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, dataset, field, operator, values, is_active, created_by, created_at, updated_at",
    )
    .bind(policy_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.dataset)
    .bind(&input.field)
    .bind(&input.operator)
    .bind(&input.values)
    .bind(input.is_active)
    .bind(&input.created_by)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("BI row policy not found".to_string()))?;
    Ok(policy)
}

pub async fn delete_row_policy(db: &PgPool, project_id: Uuid, policy_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM bi_row_policies WHERE id = $1 AND project_id = $2")
        .bind(policy_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("BI row policy not found".to_string()));
    }
    Ok(())
}

pub async fn list_database_connections(
    db: &PgPool,
    project_id: Uuid,
) -> AppResult<Vec<BiDatabaseConnection>> {
    let rows: Vec<BiDatabaseConnectionRow> = sqlx::query_as(
        "SELECT id, project_id, name, database_type, connection_string, allowed_schemas, \
                is_active, last_tested_at, last_error, created_by, created_at, updated_at \
         FROM bi_database_connections WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(BiDatabaseConnection::from).collect())
}

pub async fn get_database_connection(
    db: &PgPool,
    project_id: Uuid,
    connection_id: Uuid,
) -> AppResult<BiDatabaseConnection> {
    let row = get_database_connection_row(db, project_id, connection_id).await?;
    Ok(BiDatabaseConnection::from(row))
}

pub async fn create_database_connection(
    db: &PgPool,
    project_id: Uuid,
    input: BiDatabaseConnectionInput,
) -> AppResult<BiDatabaseConnection> {
    let input = validate_database_connection_input(input)?;
    let row: BiDatabaseConnectionRow = sqlx::query_as(
        "INSERT INTO bi_database_connections \
         (project_id, name, database_type, connection_string, allowed_schemas, is_active, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id, project_id, name, database_type, connection_string, allowed_schemas, \
                   is_active, last_tested_at, last_error, created_by, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.database_type)
    .bind(&input.connection_string)
    .bind(&input.allowed_schemas)
    .bind(input.is_active)
    .bind(&input.created_by)
    .fetch_one(db)
    .await?;
    Ok(BiDatabaseConnection::from(row))
}

pub async fn update_database_connection(
    db: &PgPool,
    project_id: Uuid,
    connection_id: Uuid,
    input: BiDatabaseConnectionInput,
) -> AppResult<BiDatabaseConnection> {
    let input = validate_database_connection_input(input)?;
    let row: BiDatabaseConnectionRow = sqlx::query_as(
        "UPDATE bi_database_connections \
         SET name = $3, database_type = $4, connection_string = $5, allowed_schemas = $6, \
             is_active = $7, created_by = $8, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, database_type, connection_string, allowed_schemas, \
                   is_active, last_tested_at, last_error, created_by, created_at, updated_at",
    )
    .bind(connection_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.database_type)
    .bind(&input.connection_string)
    .bind(&input.allowed_schemas)
    .bind(input.is_active)
    .bind(&input.created_by)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("BI database connection not found".to_string()))?;
    Ok(BiDatabaseConnection::from(row))
}

pub async fn delete_database_connection(
    db: &PgPool,
    project_id: Uuid,
    connection_id: Uuid,
) -> AppResult<()> {
    let result =
        sqlx::query("DELETE FROM bi_database_connections WHERE id = $1 AND project_id = $2")
            .bind(connection_id)
            .bind(project_id)
            .execute(db)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "BI database connection not found".to_string(),
        ));
    }
    Ok(())
}

pub async fn test_database_connection(
    db: &PgPool,
    project_id: Uuid,
    connection_id: Uuid,
) -> AppResult<BiConnectionTestResponse> {
    let connection = get_database_connection_row(db, project_id, connection_id).await?;
    let test_result = test_external_connection(&connection).await;
    let (ok, error) = match test_result {
        Ok(()) => (true, None),
        Err(err) => (false, Some(err.to_string())),
    };
    let row: BiDatabaseConnectionRow = sqlx::query_as(
        "UPDATE bi_database_connections \
         SET last_tested_at = NOW(), last_error = $3, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, database_type, connection_string, allowed_schemas, \
                   is_active, last_tested_at, last_error, created_by, created_at, updated_at",
    )
    .bind(connection_id)
    .bind(project_id)
    .bind(&error)
    .fetch_one(db)
    .await?;
    Ok(BiConnectionTestResponse {
        connection: BiDatabaseConnection::from(row),
        ok,
        error,
    })
}

pub async fn list_embeds(db: &PgPool, project_id: Uuid) -> AppResult<Vec<BiEmbed>> {
    let rows: Vec<BiEmbedRow> = sqlx::query_as(&format!(
        "SELECT {BI_EMBED_COLUMNS} FROM bi_embeds \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(BiEmbed::from).collect())
}

pub async fn get_embed(db: &PgPool, project_id: Uuid, embed_id: Uuid) -> AppResult<BiEmbed> {
    let row = get_embed_row(db, project_id, embed_id).await?;
    Ok(BiEmbed::from(row))
}

pub async fn create_embed(
    db: &PgPool,
    project_id: Uuid,
    input: BiEmbedInput,
) -> AppResult<BiEmbedWithToken> {
    let input = validate_embed_input(input)?;
    verify_embed_resource(db, project_id, &input).await?;
    let token = generate_embed_token();
    let token_hash = hash_embed_token(&token);
    let token_prefix = embed_token_prefix(&token);
    let allowed_origins = json!(input.allowed_origins);

    let row: BiEmbedRow = sqlx::query_as(&format!(
        "INSERT INTO bi_embeds \
         (project_id, name, resource_type, resource_id, resource_config, allowed_origins, theme, \
          token_hash, token_prefix, is_active, expires_at, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         RETURNING {BI_EMBED_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.resource_type)
    .bind(input.resource_id)
    .bind(&input.resource_config)
    .bind(&allowed_origins)
    .bind(&input.theme)
    .bind(&token_hash)
    .bind(&token_prefix)
    .bind(input.is_active)
    .bind(input.expires_at)
    .bind(&input.created_by)
    .fetch_one(db)
    .await?;

    Ok(embed_with_token(row, token))
}

pub async fn update_embed(
    db: &PgPool,
    project_id: Uuid,
    embed_id: Uuid,
    input: BiEmbedInput,
) -> AppResult<BiEmbed> {
    let input = validate_embed_input(input)?;
    verify_embed_resource(db, project_id, &input).await?;
    let allowed_origins = json!(input.allowed_origins);

    let row: BiEmbedRow = sqlx::query_as(&format!(
        "UPDATE bi_embeds SET \
           name = $3, resource_type = $4, resource_id = $5, resource_config = $6, \
           allowed_origins = $7, theme = $8, is_active = $9, expires_at = $10, \
           created_by = $11, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING {BI_EMBED_COLUMNS}"
    ))
    .bind(embed_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.resource_type)
    .bind(input.resource_id)
    .bind(&input.resource_config)
    .bind(&allowed_origins)
    .bind(&input.theme)
    .bind(input.is_active)
    .bind(input.expires_at)
    .bind(&input.created_by)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("BI embed not found".to_string()))?;
    Ok(BiEmbed::from(row))
}

pub async fn delete_embed(db: &PgPool, project_id: Uuid, embed_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM bi_embeds WHERE id = $1 AND project_id = $2")
        .bind(embed_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("BI embed not found".to_string()));
    }
    Ok(())
}

pub async fn rotate_embed_token(
    db: &PgPool,
    project_id: Uuid,
    embed_id: Uuid,
) -> AppResult<BiEmbedWithToken> {
    let token = generate_embed_token();
    let token_hash = hash_embed_token(&token);
    let token_prefix = embed_token_prefix(&token);
    let row: BiEmbedRow = sqlx::query_as(&format!(
        "UPDATE bi_embeds SET token_hash = $3, token_prefix = $4, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 RETURNING {BI_EMBED_COLUMNS}"
    ))
    .bind(embed_id)
    .bind(project_id)
    .bind(&token_hash)
    .bind(&token_prefix)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("BI embed not found".to_string()))?;
    Ok(embed_with_token(row, token))
}

pub async fn resolve_embed(
    db: &PgPool,
    token: &str,
    request_origin: Option<&str>,
) -> AppResult<BiEmbedResolved> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::Unauthorized);
    }
    let token_hash = hash_embed_token(token);
    let row: BiEmbedRow = sqlx::query_as(&format!(
        "SELECT {BI_EMBED_COLUMNS} FROM bi_embeds \
         WHERE token_hash = $1 AND is_active = true \
           AND (expires_at IS NULL OR expires_at > NOW())"
    ))
    .bind(&token_hash)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !origin_is_allowed(&row.allowed_origins, request_origin) {
        return Err(AppError::Forbidden(
            "Origin is not allowed for this BI embed".to_string(),
        ));
    }

    let (resource, result) = embed_payload(db, &row).await?;
    let row: BiEmbedRow = sqlx::query_as(&format!(
        "UPDATE bi_embeds SET last_accessed_at = NOW(), access_count = access_count + 1 \
         WHERE id = $1 RETURNING {BI_EMBED_COLUMNS}"
    ))
    .bind(row.id)
    .fetch_one(db)
    .await?;

    Ok(BiEmbedResolved {
        embed: BiEmbed::from(row),
        resource,
        result,
    })
}

pub async fn list_saved_queries(db: &PgPool, project_id: Uuid) -> AppResult<Vec<SavedSqlQuery>> {
    let queries = sqlx::query_as(
        "SELECT id, project_id, name, description, sql_text, parameters, created_by, created_at, updated_at \
         FROM saved_sql_queries WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(queries)
}

pub async fn get_saved_query(
    db: &PgPool,
    project_id: Uuid,
    query_id: Uuid,
) -> AppResult<SavedSqlQuery> {
    sqlx::query_as(
        "SELECT id, project_id, name, description, sql_text, parameters, created_by, created_at, updated_at \
         FROM saved_sql_queries WHERE id = $1 AND project_id = $2",
    )
    .bind(query_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Saved SQL query not found".to_string()))
}

pub async fn create_saved_query(
    db: &PgPool,
    project_id: Uuid,
    input: SavedSqlInput,
) -> AppResult<SavedSqlQuery> {
    let input = validate_saved_sql_input(input, project_id)?;
    let query = sqlx::query_as(
        "INSERT INTO saved_sql_queries (project_id, name, description, sql_text, parameters, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, project_id, name, description, sql_text, parameters, created_by, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.sql_text)
    .bind(&input.parameters)
    .bind(&input.created_by)
    .fetch_one(db)
    .await?;
    Ok(query)
}

pub async fn update_saved_query(
    db: &PgPool,
    project_id: Uuid,
    query_id: Uuid,
    input: SavedSqlInput,
) -> AppResult<SavedSqlQuery> {
    let input = validate_saved_sql_input(input, project_id)?;
    let query = sqlx::query_as(
        "UPDATE saved_sql_queries SET name = $3, description = $4, sql_text = $5, parameters = $6, \
           created_by = $7, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING id, project_id, name, description, sql_text, parameters, created_by, created_at, updated_at",
    )
    .bind(query_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.sql_text)
    .bind(&input.parameters)
    .bind(&input.created_by)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Saved SQL query not found".to_string()))?;
    Ok(query)
}

pub async fn delete_saved_query(db: &PgPool, project_id: Uuid, query_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM saved_sql_queries WHERE id = $1 AND project_id = $2")
        .bind(query_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Saved SQL query not found".to_string()));
    }
    Ok(())
}

pub async fn run_ad_hoc_sql(
    db: &PgPool,
    project_id: Uuid,
    input: SqlRunRequest,
) -> AppResult<BiQueryResponse> {
    execute_sql(db, project_id, None, "sql", &input.sql_text, input.limit).await
}

pub async fn run_saved_query(
    db: &PgPool,
    project_id: Uuid,
    query_id: Uuid,
    limit: Option<i64>,
) -> AppResult<BiQueryResponse> {
    let query = get_saved_query(db, project_id, query_id).await?;
    execute_sql(
        db,
        project_id,
        Some(query_id),
        "saved_sql",
        &query.sql_text,
        limit,
    )
    .await
}

pub async fn run_visual_query(
    db: &PgPool,
    project_id: Uuid,
    input: VisualQueryRequest,
) -> AppResult<BiQueryResponse> {
    let policies = active_row_policy_clauses(db, project_id, &input.dataset).await?;
    let sql_text = build_visual_sql_with_policies(&input, &policies)?;
    execute_sql(db, project_id, None, "visual", &sql_text, input.limit).await
}

pub async fn run_drill_through(
    db: &PgPool,
    project_id: Uuid,
    input: DrillThroughRequest,
) -> AppResult<BiQueryResponse> {
    let policies = active_row_policy_clauses(db, project_id, &input.dataset).await?;
    let sql_text = build_drill_through_sql_with_policies(&input, &policies)?;
    execute_sql(
        db,
        project_id,
        None,
        "drill_through",
        &sql_text,
        input.limit,
    )
    .await
}

pub async fn run_external_sql(
    db: &PgPool,
    project_id: Uuid,
    connection_id: Uuid,
    input: ExternalSqlRunRequest,
) -> AppResult<BiQueryResponse> {
    let connection = get_database_connection_row(db, project_id, connection_id).await?;
    if !connection.is_active {
        return Err(AppError::BadRequest(
            "BI database connection is inactive".to_string(),
        ));
    }
    execute_external_sql(db, project_id, &connection, &input.sql_text, input.limit).await
}

pub async fn list_query_runs(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<BiQueryRun>> {
    let runs = sqlx::query_as(
        "SELECT id, project_id, query_id, query_type, sql_text, result, row_count, duration_ms, status, error_message, created_at \
         FROM bi_query_runs WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(runs)
}

pub async fn create_csv_upload(
    db: &PgPool,
    project_id: Uuid,
    input: CsvUploadInput,
) -> AppResult<CsvUpload> {
    let input = validate_csv_upload(input)?;
    let mut tx = db.begin().await?;
    let columns_json = json!(input.columns);
    let upload: CsvUpload = sqlx::query_as(
        "INSERT INTO csv_uploads (project_id, name, description, columns, row_count, uploaded_by) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, project_id, name, description, columns, row_count, uploaded_by, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(columns_json)
    .bind(input.rows.len() as i32)
    .bind(&input.uploaded_by)
    .fetch_one(&mut *tx)
    .await?;

    for (idx, row) in input.rows.iter().enumerate() {
        sqlx::query(
            "INSERT INTO csv_upload_rows (upload_id, project_id, row_number, row_data) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(upload.id)
        .bind(project_id)
        .bind((idx + 1) as i32)
        .bind(row)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(upload)
}

pub async fn list_csv_uploads(db: &PgPool, project_id: Uuid) -> AppResult<Vec<CsvUpload>> {
    let uploads = sqlx::query_as(
        "SELECT id, project_id, name, description, columns, row_count, uploaded_by, created_at, updated_at \
         FROM csv_uploads WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(uploads)
}

pub async fn get_csv_upload_rows(
    db: &PgPool,
    project_id: Uuid,
    upload_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<serde_json::Value>> {
    let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT row_data FROM csv_upload_rows \
         WHERE project_id = $1 AND upload_id = $2 \
         ORDER BY row_number ASC LIMIT $3 OFFSET $4",
    )
    .bind(project_id)
    .bind(upload_id)
    .bind(limit.clamp(1, 1000))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|row| row.0).collect())
}

pub async fn delete_csv_upload(db: &PgPool, project_id: Uuid, upload_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM csv_uploads WHERE id = $1 AND project_id = $2")
        .bind(upload_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("CSV upload not found".to_string()));
    }
    Ok(())
}

async fn execute_sql(
    db: &PgPool,
    project_id: Uuid,
    query_id: Option<Uuid>,
    query_type: &str,
    sql_text: &str,
    limit: Option<i64>,
) -> AppResult<BiQueryResponse> {
    let prepared = prepare_safe_sql(sql_text, project_id)?;
    let limit = limit.unwrap_or(100).clamp(1, 1000);
    let wrapped = format!("SELECT row_to_json(q)::jsonb AS row FROM ({prepared}) q LIMIT {limit}");
    let start = Instant::now();
    // Run inside a READ ONLY transaction with a hard statement timeout so the internal
    // analytics DB cannot be mutated or tied up (e.g. pg_sleep) even if a malicious query
    // slips past the keyword/allowlist checks — mirroring the external-postgres path.
    let execution = run_internal_readonly_select(db, &wrapped).await;
    let duration_ms = start.elapsed().as_millis().min(i32::MAX as u128) as i32;

    match execution {
        Ok(rows) => {
            let rows: Vec<serde_json::Value> = rows.into_iter().map(|row| row.0).collect();
            let result = serde_json::Value::Array(rows.clone());
            let run = insert_query_run(
                db,
                project_id,
                query_id,
                query_type,
                sql_text,
                &result,
                rows.len() as i32,
                duration_ms,
                "success",
                None,
            )
            .await?;
            Ok(BiQueryResponse { run, rows })
        }
        Err(err) => {
            let error = err.to_string();
            let _ = insert_query_run(
                db,
                project_id,
                query_id,
                query_type,
                sql_text,
                &json!([]),
                0,
                duration_ms,
                "error",
                Some(&error),
            )
            .await;
            Err(AppError::BadRequest(format!(
                "SQL execution failed: {error}"
            )))
        }
    }
}

/// Execute a prepared internal BI SELECT inside a READ ONLY, time-bounded transaction.
async fn run_internal_readonly_select(
    db: &PgPool,
    wrapped_sql: &str,
) -> Result<Vec<(serde_json::Value,)>, sqlx::Error> {
    let mut tx = db.begin().await?;
    // SET TRANSACTION must precede any query in the transaction.
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query_as::<_, (serde_json::Value,)>(wrapped_sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(rows)
}

async fn execute_external_sql(
    db: &PgPool,
    project_id: Uuid,
    connection: &BiDatabaseConnectionRow,
    sql_text: &str,
    limit: Option<i64>,
) -> AppResult<BiQueryResponse> {
    let prepared = prepare_external_sql(sql_text)?;
    let limit = limit.unwrap_or(100).clamp(1, 1000);
    let start = Instant::now();
    let execution = match connection.database_type.as_str() {
        "postgres" => execute_external_postgres_sql(connection, &prepared, limit).await,
        "clickhouse" => execute_external_clickhouse_sql(connection, &prepared, limit).await,
        "http_json" => execute_external_http_json_sql(connection, &prepared, limit).await,
        other => Err(AppError::BadRequest(format!(
            "Unsupported BI database type: {other}"
        ))),
    };
    let duration_ms = start.elapsed().as_millis().min(i32::MAX as u128) as i32;

    match execution {
        Ok(rows) => {
            let result = serde_json::Value::Array(rows.clone());
            let run = insert_query_run(
                db,
                project_id,
                None,
                "external_sql",
                sql_text,
                &result,
                rows.len() as i32,
                duration_ms,
                "success",
                None,
            )
            .await?;
            Ok(BiQueryResponse { run, rows })
        }
        Err(err) => {
            let error = err.to_string();
            let _ = insert_query_run(
                db,
                project_id,
                None,
                "external_sql",
                sql_text,
                &json!([]),
                0,
                duration_ms,
                "error",
                Some(&error),
            )
            .await;
            Err(AppError::BadRequest(format!(
                "External SQL execution failed: {error}"
            )))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn insert_query_run(
    db: &PgPool,
    project_id: Uuid,
    query_id: Option<Uuid>,
    query_type: &str,
    sql_text: &str,
    result: &serde_json::Value,
    row_count: i32,
    duration_ms: i32,
    status: &str,
    error_message: Option<&str>,
) -> AppResult<BiQueryRun> {
    let run = sqlx::query_as(
        "INSERT INTO bi_query_runs \
         (project_id, query_id, query_type, sql_text, result, row_count, duration_ms, status, error_message) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id, project_id, query_id, query_type, sql_text, result, row_count, duration_ms, status, error_message, created_at",
    )
    .bind(project_id)
    .bind(query_id)
    .bind(query_type)
    .bind(sql_text)
    .bind(result)
    .bind(row_count)
    .bind(duration_ms)
    .bind(status)
    .bind(error_message)
    .fetch_one(db)
    .await?;
    Ok(run)
}

async fn get_database_connection_row(
    db: &PgPool,
    project_id: Uuid,
    connection_id: Uuid,
) -> AppResult<BiDatabaseConnectionRow> {
    sqlx::query_as(
        "SELECT id, project_id, name, database_type, connection_string, allowed_schemas, \
                is_active, last_tested_at, last_error, created_by, created_at, updated_at \
         FROM bi_database_connections WHERE id = $1 AND project_id = $2",
    )
    .bind(connection_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("BI database connection not found".to_string()))
}

async fn test_external_connection(connection: &BiDatabaseConnectionRow) -> AppResult<()> {
    match connection.database_type.as_str() {
        "postgres" => test_external_postgres_connection(connection).await,
        "clickhouse" => execute_external_clickhouse_sql(connection, "SELECT 1 AS ok", 1)
            .await
            .map(|_| ()),
        "http_json" => execute_external_http_json_sql(connection, "SELECT 1 AS ok", 1)
            .await
            .map(|_| ()),
        other => Err(AppError::BadRequest(format!(
            "Unsupported BI database type: {other}"
        ))),
    }
}

async fn test_external_postgres_connection(connection: &BiDatabaseConnectionRow) -> AppResult<()> {
    let pool = connect_external_pool(connection).await?;
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    set_external_search_path(&mut tx, &connection.allowed_schemas).await?;
    let _: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    tx.rollback().await.map_err(AppError::Database)?;
    Ok(())
}

async fn execute_external_postgres_sql(
    connection: &BiDatabaseConnectionRow,
    prepared: &str,
    limit: i64,
) -> AppResult<Vec<serde_json::Value>> {
    let wrapped = format!("SELECT row_to_json(q)::jsonb AS row FROM ({prepared}) q LIMIT {limit}");
    let pool = connect_external_pool(connection).await?;
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    set_external_search_path(&mut tx, &connection.allowed_schemas).await?;
    let rows = sqlx::query_as::<_, (serde_json::Value,)>(&wrapped)
        .fetch_all(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    tx.rollback().await.map_err(AppError::Database)?;
    Ok(rows.into_iter().map(|row| row.0).collect())
}

async fn execute_external_clickhouse_sql(
    connection: &BiDatabaseConnectionRow,
    prepared: &str,
    limit: i64,
) -> AppResult<Vec<serde_json::Value>> {
    let (mut url, auth) = parse_http_connection_url(&connection.connection_string, "ClickHouse")?;
    ensure_http_adapter_target_allowed(&url).await?;
    apply_clickhouse_database_scope(&mut url, &connection.allowed_schemas)?;
    validate_clickhouse_sql_scope(prepared, &connection.allowed_schemas)?;
    url.query_pairs_mut().append_pair("readonly", "1");
    let sql = format!("SELECT * FROM ({prepared}) AS q LIMIT {limit} FORMAT JSONEachRow");
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let mut request = client
        .post(url)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(sql);
    if let Some((username, password)) = auth {
        request = request.basic_auth(username, password);
    }
    let response = request
        .send()
        .await
        .map_err(|err| AppError::BadRequest(format!("ClickHouse query failed: {err}")))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::BadRequest(format!(
            "ClickHouse returned HTTP {}: {}",
            status.as_u16(),
            truncate_text(&body, 512)
        )));
    }
    parse_json_each_row(&body)
}

async fn execute_external_http_json_sql(
    connection: &BiDatabaseConnectionRow,
    prepared: &str,
    limit: i64,
) -> AppResult<Vec<serde_json::Value>> {
    let (url, auth) = parse_http_connection_url(&connection.connection_string, "HTTP JSON")?;
    ensure_http_adapter_target_allowed(&url).await?;
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let mut request = client.post(url).json(&json!({
        "sql": prepared,
        "limit": limit,
        "allowed_schemas": connection.allowed_schemas,
        "enforce_allowed_schemas": true,
    }));
    if let Some((username, password)) = auth {
        request = request.basic_auth(username, password);
    }
    let response = request
        .send()
        .await
        .map_err(|err| AppError::BadRequest(format!("HTTP JSON adapter failed: {err}")))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::BadRequest(format!(
            "HTTP JSON adapter returned HTTP {}: {}",
            status.as_u16(),
            truncate_text(&body, 512)
        )));
    }
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|err| {
        AppError::BadRequest(format!("HTTP JSON adapter returned invalid JSON: {err}"))
    })?;
    rows_from_http_json_response(value)
}

async fn connect_external_pool(connection: &BiDatabaseConnectionRow) -> AppResult<PgPool> {
    if connection.database_type != "postgres" {
        return Err(AppError::BadRequest(format!(
            "Unsupported BI database type: {}",
            connection.database_type
        )));
    }
    let connect = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(StdDuration::from_secs(5))
        .connect(&connection.connection_string);
    match tokio::time::timeout(StdDuration::from_secs(5), connect).await {
        Ok(Ok(pool)) => Ok(pool),
        Ok(Err(err)) => Err(AppError::BadRequest(format!(
            "External database connection failed: {err}"
        ))),
        Err(_) => Err(AppError::BadRequest(
            "External database connection timed out".to_string(),
        )),
    }
}

fn parse_http_connection_url(
    connection_string: &str,
    label: &str,
) -> AppResult<(url::Url, Option<(String, Option<String>)>)> {
    let mut parsed = url::Url::parse(connection_string)
        .map_err(|_| AppError::BadRequest(format!("{label} connection_string must be a URL")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::BadRequest(format!(
            "{label} connection_string must start with http:// or https://"
        )));
    }
    reject_local_http_adapter_host(&parsed)?;

    let auth = if parsed.username().is_empty() {
        None
    } else {
        let username = parsed.username().to_string();
        let password = parsed.password().map(ToString::to_string);
        let _ = parsed.set_username("");
        let _ = parsed.set_password(None);
        Some((username, password))
    };
    Ok((parsed, auth))
}

fn reject_local_http_adapter_host(url: &url::Url) -> AppResult<()> {
    let Some(host) = url.host_str() else {
        return Err(AppError::BadRequest(
            "HTTP adapter URL requires a host".to_string(),
        ));
    };
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    if normalized == "localhost" || normalized.ends_with(".localhost") {
        return Err(AppError::BadRequest(
            "HTTP adapter URL cannot target localhost".to_string(),
        ));
    }
    if let Ok(ip) = normalized.parse::<IpAddr>() {
        reject_private_adapter_ip(ip)?;
    }
    Ok(())
}

async fn ensure_http_adapter_target_allowed(url: &url::Url) -> AppResult<()> {
    let host = url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("HTTP adapter URL requires a host".to_string()))?;
    reject_local_http_adapter_host(url)?;
    let port = url.port_or_known_default().ok_or_else(|| {
        AppError::BadRequest("HTTP adapter URL requires a resolvable port".to_string())
    })?;
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|err| AppError::BadRequest(format!("HTTP adapter host lookup failed: {err}")))?;
    let mut resolved_any = false;
    for address in addresses {
        resolved_any = true;
        reject_private_adapter_ip(address.ip())?;
    }
    if !resolved_any {
        return Err(AppError::BadRequest(
            "HTTP adapter host did not resolve to any addresses".to_string(),
        ));
    }
    Ok(())
}

fn reject_private_adapter_ip(ip: IpAddr) -> AppResult<()> {
    if is_private_adapter_ip(ip) {
        return Err(AppError::BadRequest(
            "HTTP adapter URL cannot target private, local, or reserved network addresses"
                .to_string(),
        ));
    }
    Ok(())
}

fn is_private_adapter_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
                || ip.is_multicast()
                || octets[0] == 0
                || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 64)
                || (octets[0] == 198 && matches!(octets[1], 18 | 19))
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

fn apply_clickhouse_database_scope(
    url: &mut url::Url,
    allowed_schemas: &serde_json::Value,
) -> AppResult<()> {
    let schemas = schema_names(allowed_schemas)?;
    if schemas.len() != 1 {
        return Err(AppError::BadRequest(
            "ClickHouse BI connections require exactly one allowed schema/database".to_string(),
        ));
    }
    let schema = &schemas[0];
    let existing_database = url
        .query_pairs()
        .find_map(|(key, value)| (key == "database").then(|| value.to_string()));
    if let Some(database) = existing_database {
        if database != *schema {
            return Err(AppError::BadRequest(
                "ClickHouse database query parameter must match allowed_schemas".to_string(),
            ));
        }
    } else {
        url.query_pairs_mut().append_pair("database", schema);
    }
    Ok(())
}

fn validate_clickhouse_sql_scope(sql: &str, allowed_schemas: &serde_json::Value) -> AppResult<()> {
    let schemas = schema_names(allowed_schemas)?;
    let tokens: Vec<&str> = sql.split_whitespace().collect();
    let mut expect_table = false;
    for token in tokens {
        let normalized = token
            .trim_matches(|ch: char| matches!(ch, '(' | ')' | ','))
            .trim_matches('"')
            .trim_matches('`');
        let lower = normalized.to_ascii_lowercase();
        if expect_table {
            expect_table = false;
            if normalized.is_empty() || normalized.starts_with('(') {
                continue;
            }
            if normalized.contains('(') {
                return Err(AppError::BadRequest(
                    "ClickHouse external SQL cannot use table functions".to_string(),
                ));
            }
            if let Some((schema, _table)) = normalized.split_once('.') {
                let schema = schema.trim_matches('"').trim_matches('`');
                if !schemas.iter().any(|allowed| allowed == schema) {
                    return Err(AppError::BadRequest(format!(
                        "ClickHouse table reference uses disallowed database: {schema}"
                    )));
                }
            }
        }
        if lower == "from" || lower.ends_with("join") {
            expect_table = true;
        }
    }
    Ok(())
}

fn parse_json_each_row(body: &str) -> AppResult<Vec<serde_json::Value>> {
    let mut rows = Vec::new();
    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line).map_err(|err| {
            AppError::BadRequest(format!("ClickHouse returned invalid JSONEachRow: {err}"))
        })?;
        rows.push(value);
    }
    Ok(rows)
}

fn rows_from_http_json_response(value: serde_json::Value) -> AppResult<Vec<serde_json::Value>> {
    if let Some(rows) = value.as_array() {
        return Ok(rows.clone());
    }
    if let Some(rows) = value.get("rows").and_then(|rows| rows.as_array()) {
        return Ok(rows.clone());
    }
    if let Some(rows) = value.get("data").and_then(|rows| rows.as_array()) {
        return Ok(rows.clone());
    }
    Err(AppError::BadRequest(
        "HTTP JSON adapter must return an array, {\"rows\": [...]}, or {\"data\": [...]}"
            .to_string(),
    ))
}

fn truncate_text(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

async fn set_external_search_path(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    allowed_schemas: &serde_json::Value,
) -> AppResult<()> {
    let schemas = schema_names(allowed_schemas)?;
    if schemas.is_empty() {
        return Ok(());
    }
    let quoted = schemas
        .iter()
        .map(|schema| quote_pg_identifier(schema))
        .collect::<AppResult<Vec<_>>>()?
        .join(", ");
    sqlx::query(&format!("SET LOCAL search_path TO {quoted}"))
        .execute(&mut **tx)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn get_embed_row(db: &PgPool, project_id: Uuid, embed_id: Uuid) -> AppResult<BiEmbedRow> {
    sqlx::query_as(&format!(
        "SELECT {BI_EMBED_COLUMNS} FROM bi_embeds WHERE id = $1 AND project_id = $2"
    ))
    .bind(embed_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("BI embed not found".to_string()))
}

fn embed_with_token(row: BiEmbedRow, token: String) -> BiEmbedWithToken {
    let embed_url = format!("/api/embed/bi/{token}");
    BiEmbedWithToken {
        embed: BiEmbed::from(row),
        token,
        embed_url,
    }
}

async fn verify_embed_resource(
    db: &PgPool,
    project_id: Uuid,
    input: &BiEmbedInput,
) -> AppResult<()> {
    match input.resource_type.as_str() {
        "dashboard" => {
            let id = required_embed_resource_id(input)?;
            product_analytics::get_dashboard(db, project_id, id).await?;
        }
        "report" => {
            let id = required_embed_resource_id(input)?;
            product_analytics::get_report(db, project_id, id).await?;
        }
        "sql_query" => {
            let id = required_embed_resource_id(input)?;
            get_saved_query(db, project_id, id).await?;
        }
        "visual_query" => {
            parse_visual_embed_config(&input.resource_config)?;
        }
        "metric" => {
            if let Some(id) = input.resource_id {
                get_metric(db, project_id, id).await?;
            } else if input.resource_config.get("metric_key").is_none() {
                return Err(AppError::BadRequest(
                    "Metric embeds require resource_id or resource_config.metric_key".to_string(),
                ));
            }
        }
        _ => unreachable!("validated resource type"),
    }
    Ok(())
}

fn required_embed_resource_id(input: &BiEmbedInput) -> AppResult<Uuid> {
    input.resource_id.ok_or_else(|| {
        AppError::BadRequest(format!(
            "{} embeds require resource_id",
            input.resource_type
        ))
    })
}

async fn embed_payload(
    db: &PgPool,
    row: &BiEmbedRow,
) -> AppResult<(serde_json::Value, Option<serde_json::Value>)> {
    match row.resource_type.as_str() {
        "dashboard" => {
            let dashboard = product_analytics::get_dashboard(
                db,
                row.project_id,
                row.resource_id.ok_or_else(|| {
                    AppError::BadRequest("Embed missing dashboard id".to_string())
                })?,
            )
            .await?;
            Ok((json!(dashboard), None))
        }
        "report" => {
            let report = product_analytics::get_report(
                db,
                row.project_id,
                row.resource_id
                    .ok_or_else(|| AppError::BadRequest("Embed missing report id".to_string()))?,
            )
            .await?;
            Ok((json!(report), None))
        }
        "sql_query" => {
            let query_id = row
                .resource_id
                .ok_or_else(|| AppError::BadRequest("Embed missing SQL query id".to_string()))?;
            let query = get_saved_query(db, row.project_id, query_id).await?;
            let result = run_saved_query(db, row.project_id, query_id, embed_limit(row)).await?;
            Ok((json!(query), Some(json!(result))))
        }
        "visual_query" => {
            let input = parse_visual_embed_config(&row.resource_config)?;
            let result = run_visual_query(db, row.project_id, input).await?;
            Ok((row.resource_config.clone(), Some(json!(result))))
        }
        "metric" => {
            if let Some(id) = row.resource_id {
                let metric = get_metric(db, row.project_id, id).await?;
                Ok((json!(metric), None))
            } else {
                Ok((row.resource_config.clone(), None))
            }
        }
        other => Err(AppError::BadRequest(format!(
            "Unsupported embed resource type: {other}"
        ))),
    }
}

fn embed_limit(row: &BiEmbedRow) -> Option<i64> {
    row.resource_config
        .get("limit")
        .and_then(|value| value.as_i64())
}

fn parse_visual_embed_config(config: &serde_json::Value) -> AppResult<VisualQueryRequest> {
    if !config.is_object() {
        return Err(AppError::BadRequest(
            "visual_query embeds require an object resource_config".to_string(),
        ));
    }
    serde_json::from_value(config.clone())
        .map_err(|err| AppError::BadRequest(format!("Invalid visual query embed config: {err}")))
}

fn validate_metric_input(mut input: SemanticMetricInput) -> AppResult<SemanticMetricInput> {
    input.key = input.key.trim().to_string();
    input.name = input.name.trim().to_string();
    input.dataset = validate_dataset(&input.dataset)?.to_string();
    input.expression = input.expression.trim().to_string();
    if input.key.is_empty() || input.name.is_empty() || input.expression.is_empty() {
        return Err(AppError::BadRequest(
            "Metric key, name, and expression are required".to_string(),
        ));
    }
    if !input.filters.is_object() {
        return Err(AppError::BadRequest(
            "filters must be an object".to_string(),
        ));
    }
    Ok(input)
}

fn validate_saved_sql_input(
    mut input: SavedSqlInput,
    project_id: Uuid,
) -> AppResult<SavedSqlInput> {
    input.name = input.name.trim().to_string();
    input.sql_text = input.sql_text.trim().to_string();
    if input.name.is_empty() {
        return Err(AppError::BadRequest("Query name is required".to_string()));
    }
    prepare_safe_sql(&input.sql_text, project_id)?;
    if !input.parameters.is_object() {
        return Err(AppError::BadRequest(
            "parameters must be an object".to_string(),
        ));
    }
    Ok(input)
}

fn validate_row_policy_input(mut input: BiRowPolicyInput) -> AppResult<BiRowPolicyInput> {
    input.name = input.name.trim().to_string();
    input.dataset = validate_dataset(&input.dataset)?.to_string();
    input.field = input.field.trim().to_string();
    input.operator = input.operator.trim().to_ascii_lowercase();
    input.created_by = input
        .created_by
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if input.name.is_empty() || input.field.is_empty() {
        return Err(AppError::BadRequest(
            "Policy name and field are required".to_string(),
        ));
    }
    drill_filter_column(&input.dataset, &input.field)?;
    if !matches!(input.operator.as_str(), "eq" | "neq" | "in" | "not_in") {
        return Err(AppError::BadRequest(format!(
            "Unsupported row policy operator: {}",
            input.operator
        )));
    }
    if !input.values.is_array() {
        input.values = json!([input.values]);
    }
    let values = input
        .values
        .as_array()
        .ok_or_else(|| AppError::BadRequest("Row policy values must be an array".to_string()))?;
    if values.is_empty() {
        return Err(AppError::BadRequest(
            "Row policy values cannot be empty".to_string(),
        ));
    }
    for value in values {
        sql_literal(value)?;
    }
    Ok(input)
}

fn validate_database_connection_input(
    mut input: BiDatabaseConnectionInput,
) -> AppResult<BiDatabaseConnectionInput> {
    input.name = input.name.trim().to_string();
    input.database_type = input.database_type.trim().to_ascii_lowercase();
    input.connection_string = input.connection_string.trim().to_string();
    input.allowed_schemas = normalize_allowed_schemas(&input.allowed_schemas)?;
    input.created_by = input
        .created_by
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if input.name.is_empty() || input.connection_string.is_empty() {
        return Err(AppError::BadRequest(
            "Connection name and connection_string are required".to_string(),
        ));
    }
    if !SUPPORTED_BI_DATABASE_TYPES.contains(&input.database_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unsupported BI database type: {}. Supported types: {}",
            input.database_type,
            SUPPORTED_BI_DATABASE_TYPES.join(", ")
        )));
    }
    match input.database_type.as_str() {
        "postgres" => {
            if !(input.connection_string.starts_with("postgres://")
                || input.connection_string.starts_with("postgresql://"))
            {
                return Err(AppError::BadRequest(
                    "Postgres connections require a postgres:// or postgresql:// connection string"
                        .to_string(),
                ));
            }
        }
        "clickhouse" => {
            let (mut url, _) = parse_http_connection_url(&input.connection_string, "ClickHouse")?;
            apply_clickhouse_database_scope(&mut url, &input.allowed_schemas)?;
        }
        "http_json" => {
            parse_http_connection_url(&input.connection_string, "HTTP JSON")?;
        }
        _ => unreachable!("database type checked above"),
    }
    Ok(input)
}

fn validate_embed_input(mut input: BiEmbedInput) -> AppResult<BiEmbedInput> {
    input.name = input.name.trim().to_string();
    input.resource_type = input.resource_type.trim().to_ascii_lowercase();
    input.allowed_origins = normalize_embed_origins(&input.allowed_origins)?;
    input.created_by = input
        .created_by
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if input.name.is_empty() {
        return Err(AppError::BadRequest("Embed name is required".to_string()));
    }
    if !matches!(
        input.resource_type.as_str(),
        "dashboard" | "report" | "sql_query" | "visual_query" | "metric"
    ) {
        return Err(AppError::BadRequest(format!(
            "Unsupported BI embed resource type: {}",
            input.resource_type
        )));
    }
    if !input.resource_config.is_object() {
        return Err(AppError::BadRequest(
            "resource_config must be an object".to_string(),
        ));
    }
    if !input.theme.is_object() {
        return Err(AppError::BadRequest("theme must be an object".to_string()));
    }
    if input
        .expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now())
    {
        return Err(AppError::BadRequest(
            "expires_at must be in the future".to_string(),
        ));
    }
    if input.resource_type == "visual_query" {
        parse_visual_embed_config(&input.resource_config)?;
    }
    Ok(input)
}

fn validate_csv_upload(mut input: CsvUploadInput) -> AppResult<CsvUploadInput> {
    input.name = input.name.trim().to_string();
    input.columns = input
        .columns
        .into_iter()
        .map(|column| column.trim().to_string())
        .filter(|column| !column.is_empty())
        .collect();
    if input.name.is_empty() {
        return Err(AppError::BadRequest(
            "CSV upload name is required".to_string(),
        ));
    }
    if input.columns.is_empty() {
        return Err(AppError::BadRequest(
            "CSV uploads require at least one column".to_string(),
        ));
    }
    if input.rows.len() > 10_000 {
        return Err(AppError::BadRequest(
            "CSV uploads support at most 10000 rows per request".to_string(),
        ));
    }
    if input.rows.iter().any(|row| !row.is_object()) {
        return Err(AppError::BadRequest(
            "CSV upload rows must be JSON objects".to_string(),
        ));
    }
    Ok(input)
}

fn prepare_external_sql(sql_text: &str) -> AppResult<String> {
    let sql = sql_text.trim();
    validate_read_only_sql(sql)?;
    if sql.contains("{{project_id}}") {
        return Err(AppError::BadRequest(
            "External BI SQL cannot use the {{project_id}} tenant placeholder".to_string(),
        ));
    }
    Ok(sql.to_string())
}

fn prepare_safe_sql(sql_text: &str, project_id: Uuid) -> AppResult<String> {
    let sql = sql_text.trim();
    validate_read_only_sql(sql)?;
    if !sql.contains("{{project_id}}") {
        return Err(AppError::BadRequest(
            "BI SQL must include the {{project_id}} tenant placeholder".to_string(),
        ));
    }
    // The {{project_id}} placeholder is NOT a tenant boundary on its own — a query
    // may reference any table while still containing the placeholder somewhere
    // (e.g. `... WHERE '{{project_id}}' = '{{project_id}}'`). Restrict the query to
    // an allowlist of tenant-scoped analytics tables so a query-scoped key can never
    // read another tenant's data or platform secrets (api_keys, bi_database_connections, ...).
    enforce_table_allowlist(sql)?;
    Ok(sql.replace("{{project_id}}", &format!("'{}'::uuid", project_id)))
}

/// Tables a BI SQL query is permitted to read. Every entry is tenant-scoped by a
/// `project_id` column, which the required `{{project_id}}` placeholder constrains.
/// Anything not listed here — control-plane and secret tables such as `api_keys`,
/// `bi_database_connections`, `projects`, `webhooks`, `destinations`, `shared_dashboards`,
/// `bi_embeds`, `privacy_settings` — is rejected, closing cross-tenant exfiltration.
const BI_ALLOWED_TABLES: &[&str] = &[
    // raw fact / event tables
    "pageviews",
    "events",
    "sessions",
    "web_vitals",
    "scroll_depths",
    "search_queries",
    "outlinks",
    "js_errors",
    "log_entries",
    "click_events",
    "survey_responses",
    "goal_conversions",
    "experiment_assignments",
    "feature_flag_evaluations",
    "session_recordings",
    "guide_events",
    // identity / profile tables (project-scoped)
    "user_profiles",
    "account_profiles",
    "account_memberships",
    "user_aliases",
    // daily rollups
    "daily_stats",
    "daily_pages",
    "daily_events",
    "daily_referrers",
    "daily_devices",
    "daily_geo",
    "daily_campaigns",
    // user-uploaded BI data
    "csv_uploads",
    "csv_upload_rows",
];

/// Keywords that appear in a FROM/JOIN target position but are not table names.
const RELATION_NOISE_KEYWORDS: &[&str] = &["lateral", "only"];

/// Keywords that stop the current relation expectation without ending the FROM clause.
const RELATION_STOP_KEYWORDS: &[&str] = &["on", "using"];

/// Clause keywords that terminate a FROM table list.
const FROM_TERMINATORS: &[&str] = &[
    "where",
    "group",
    "order",
    "having",
    "limit",
    "offset",
    "union",
    "except",
    "intersect",
    "window",
    "fetch",
    "for",
];

/// PostgreSQL helpers that can execute arbitrary SQL or dump whole tables/schemas.
const FORBIDDEN_BI_SQL_FUNCTIONS: &[&str] = &[
    "query_to_xml",
    "query_to_xmlschema",
    "query_to_xml_and_xmlschema",
    "table_to_xml",
    "table_to_xmlschema",
    "table_to_xml_and_xmlschema",
    "schema_to_xml",
    "schema_to_xmlschema",
    "schema_to_xml_and_xmlschema",
    "database_to_xml",
    "database_to_xmlschema",
    "database_to_xml_and_xmlschema",
    "cursor_to_xml",
    "cursor_to_xmlschema",
    "cursor_to_xml_and_xmlschema",
];

#[derive(Clone, PartialEq)]
enum SqlTok {
    Word(String),
    Dot,
    OpenParen,
    CloseParen,
    Comma,
    Other,
}

/// Tokenize already-lowercased SQL. Single-quoted string literals are collapsed to
/// `Other` (so their contents never look like keywords/tables), and double-quoted
/// identifiers become `Word`s with the quotes stripped (so `FROM "api_keys"` is caught).
fn tokenize_sql(lower_sql: &str) -> Vec<SqlTok> {
    let chars: Vec<char> = lower_sql.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            // skip a single-quoted string literal, honoring the '' escape
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            toks.push(SqlTok::Other);
        } else if c == '"' {
            // double-quoted identifier -> Word(inner)
            i += 1;
            let mut ident = String::new();
            while i < chars.len() {
                if chars[i] == '"' {
                    if i + 1 < chars.len() && chars[i + 1] == '"' {
                        ident.push('"');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                ident.push(chars[i]);
                i += 1;
            }
            toks.push(SqlTok::Word(ident));
        } else if c.is_ascii_alphanumeric() || c == '_' {
            let mut w = String::new();
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                w.push(chars[i]);
                i += 1;
            }
            toks.push(SqlTok::Word(w));
        } else {
            match c {
                '.' => toks.push(SqlTok::Dot),
                '(' => toks.push(SqlTok::OpenParen),
                ')' => toks.push(SqlTok::CloseParen),
                ',' => toks.push(SqlTok::Comma),
                ch if ch.is_whitespace() => {}
                _ => toks.push(SqlTok::Other),
            }
            i += 1;
        }
    }
    toks
}

/// Names introduced by `WITH name AS ( ... )` so they are not mistaken for base tables.
fn collect_cte_names(toks: &[SqlTok]) -> HashSet<String> {
    let mut names = HashSet::new();
    for i in 0..toks.len() {
        if let SqlTok::Word(kw) = &toks[i] {
            if kw == "as" && cte_body_starts_after_as(toks, i) {
                if let Some(name) = cte_name_before_as(toks, i) {
                    names.insert(name);
                }
            }
        }
    }
    names
}

fn cte_body_starts_after_as(toks: &[SqlTok], as_idx: usize) -> bool {
    match toks.get(as_idx + 1) {
        Some(SqlTok::OpenParen) => true,
        Some(SqlTok::Word(w)) if w == "materialized" => {
            matches!(toks.get(as_idx + 2), Some(SqlTok::OpenParen))
        }
        Some(SqlTok::Word(w)) if w == "not" => {
            matches!(
                (toks.get(as_idx + 2), toks.get(as_idx + 3)),
                (Some(SqlTok::Word(materialized)), Some(SqlTok::OpenParen))
                    if materialized == "materialized"
            )
        }
        _ => false,
    }
}

fn cte_name_before_as(toks: &[SqlTok], as_idx: usize) -> Option<String> {
    if as_idx == 0 {
        return None;
    }

    match &toks[as_idx - 1] {
        SqlTok::Word(name) => Some(name.clone()),
        SqlTok::CloseParen => {
            let mut depth = 1usize;
            let mut j = as_idx - 1;
            while j > 0 {
                j -= 1;
                match &toks[j] {
                    SqlTok::CloseParen => depth += 1,
                    SqlTok::OpenParen => {
                        depth -= 1;
                        if depth == 0 {
                            return match j.checked_sub(1).and_then(|name_idx| toks.get(name_idx)) {
                                Some(SqlTok::Word(name)) => Some(name.clone()),
                                _ => None,
                            };
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        _ => None,
    }
}

/// Per parenthesis-level parser state used to find every base relation that follows a
/// real FROM/JOIN clause — including subqueries nested inside function calls such as
/// `ARRAY(SELECT ... FROM t)` — while ignoring the `FROM` inside `EXTRACT(x FROM ts)`.
struct LevelState {
    select_seen: bool,
    expect_rel: bool,
    from_list: bool,
}

/// Collect base-table relations referenced after FROM/JOIN. Errors on schema-qualified
/// references (e.g. `pg_catalog.x`, `public.api_keys`) which are never legitimate here.
fn referenced_base_relations(toks: &[SqlTok]) -> AppResult<Vec<String>> {
    let mut relations = Vec::new();
    let mut stack = vec![LevelState {
        select_seen: false,
        expect_rel: false,
        from_list: false,
    }];

    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            SqlTok::OpenParen => {
                if let Some(top) = stack.last_mut() {
                    // a parenthesis where a relation was expected is a subquery source
                    top.expect_rel = false;
                }
                stack.push(LevelState {
                    select_seen: false,
                    expect_rel: false,
                    from_list: false,
                });
            }
            SqlTok::CloseParen => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            SqlTok::Comma => {
                if let Some(top) = stack.last_mut() {
                    if top.from_list {
                        top.expect_rel = true;
                    }
                }
            }
            SqlTok::Word(w) => {
                let top = stack.last_mut().expect("level stack is never empty");
                if w == "select" || w == "values" {
                    top.select_seen = true;
                    top.expect_rel = false;
                    top.from_list = false;
                } else if (w == "from" || w == "join") && top.select_seen {
                    top.expect_rel = true;
                    if w == "from" {
                        top.from_list = true;
                    }
                } else if FROM_TERMINATORS.contains(&w.as_str()) && top.select_seen {
                    top.expect_rel = false;
                    top.from_list = false;
                } else if RELATION_STOP_KEYWORDS.contains(&w.as_str()) && top.select_seen {
                    top.expect_rel = false;
                } else if top.expect_rel {
                    if RELATION_NOISE_KEYWORDS.contains(&w.as_str()) {
                        // skip "lateral"/"only" and keep expecting the relation
                    } else {
                        match toks.get(i + 1) {
                            Some(SqlTok::Dot) => {
                                return Err(AppError::BadRequest(format!(
                                    "BI SQL cannot reference schema-qualified tables ('{w}.*')"
                                )));
                            }
                            // word( ... ) is a table function, not a base table
                            Some(SqlTok::OpenParen) => {
                                top.expect_rel = false;
                            }
                            _ => {
                                relations.push(w.clone());
                                top.expect_rel = false;
                            }
                        }
                    }
                }
            }
            SqlTok::Dot | SqlTok::Other => {}
        }
        i += 1;
    }
    Ok(relations)
}

fn reject_forbidden_bi_functions(toks: &[SqlTok]) -> AppResult<()> {
    for (i, tok) in toks.iter().enumerate() {
        let SqlTok::Word(name) = tok else {
            continue;
        };
        if FORBIDDEN_BI_SQL_FUNCTIONS.contains(&name.as_str())
            && matches!(toks.get(i + 1), Some(SqlTok::OpenParen))
        {
            return Err(AppError::BadRequest(format!(
                "BI SQL cannot call PostgreSQL XML/query helper function {name}"
            )));
        }
    }
    Ok(())
}

/// Reject any BI SQL that reads a table outside [`BI_ALLOWED_TABLES`] (CTE names excepted).
fn enforce_table_allowlist(sql: &str) -> AppResult<()> {
    let lower = sql.to_ascii_lowercase();
    let toks = tokenize_sql(&lower);
    reject_forbidden_bi_functions(&toks)?;
    let ctes = collect_cte_names(&toks);
    for rel in referenced_base_relations(&toks)? {
        if ctes.contains(&rel) || BI_ALLOWED_TABLES.contains(&rel.as_str()) {
            continue;
        }
        return Err(AppError::BadRequest(format!(
            "BI SQL may only read approved analytics tables; table '{rel}' is not permitted"
        )));
    }
    Ok(())
}

fn validate_read_only_sql(sql: &str) -> AppResult<()> {
    let lower = sql.to_ascii_lowercase();
    if !(lower.starts_with("select ") || lower.starts_with("with ")) {
        return Err(AppError::BadRequest(
            "BI SQL must start with SELECT or WITH".to_string(),
        ));
    }
    if lower.contains(';') || lower.contains("--") || lower.contains("/*") || lower.contains("*/") {
        return Err(AppError::BadRequest(
            "BI SQL cannot contain comments or statement separators".to_string(),
        ));
    }
    for keyword in [
        "insert", "update", "delete", "drop", "alter", "create", "truncate", "grant", "revoke",
        "copy", "vacuum", "analyze", "call", "execute", "prepare", "refresh",
    ] {
        if contains_keyword(&lower, keyword) {
            return Err(AppError::BadRequest(format!(
                "BI SQL cannot contain the keyword {keyword}"
            )));
        }
    }
    Ok(())
}

fn contains_keyword(sql: &str, keyword: &str) -> bool {
    sql.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|part| part == keyword)
}

fn generate_embed_token() -> String {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    format!("pemb_{}", hex::encode(random_bytes))
}

fn hash_embed_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    hex::encode(hasher.finalize())
}

fn embed_token_prefix(token: &str) -> String {
    token.chars().take(16).collect()
}

fn normalize_embed_origins(origins: &[String]) -> AppResult<Vec<String>> {
    if origins.len() > 20 {
        return Err(AppError::BadRequest(
            "allowed_origins supports at most 20 origins".to_string(),
        ));
    }
    let mut normalized = Vec::new();
    for origin in origins {
        let origin = normalize_embed_origin(origin)?;
        if !normalized.contains(&origin) {
            normalized.push(origin);
        }
    }
    Ok(normalized)
}

fn normalize_embed_origin(origin: &str) -> AppResult<String> {
    let origin = origin.trim();
    if origin == "*" {
        return Ok(origin.to_string());
    }
    let parsed = url::Url::parse(origin)
        .map_err(|_| AppError::BadRequest(format!("Invalid embed origin: {origin}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::BadRequest(format!(
            "Invalid embed origin: {origin}"
        )));
    }
    let port = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Ok(format!(
        "{}://{}{}",
        parsed.scheme(),
        parsed.host_str().unwrap_or_default(),
        port
    ))
}

fn origin_is_allowed(allowed_origins: &serde_json::Value, request_origin: Option<&str>) -> bool {
    let Some(origins) = allowed_origins.as_array() else {
        return false;
    };
    if origins.is_empty() || origins.iter().any(|origin| origin.as_str() == Some("*")) {
        return true;
    }
    let Some(request_origin) = request_origin else {
        return false;
    };
    let Ok(request_origin) = normalize_embed_origin(request_origin) else {
        return false;
    };
    origins
        .iter()
        .filter_map(|origin| origin.as_str())
        .any(|origin| origin == request_origin)
}

fn normalize_allowed_schemas(value: &serde_json::Value) -> AppResult<serde_json::Value> {
    let schemas = schema_names(value)?;
    let mut normalized = Vec::new();
    for schema in schemas {
        if !normalized.contains(&schema) {
            normalized.push(schema);
        }
    }
    if normalized.is_empty() {
        normalized.push("public".to_string());
    }
    if normalized.len() > 20 {
        return Err(AppError::BadRequest(
            "allowed_schemas supports at most 20 schemas".to_string(),
        ));
    }
    Ok(json!(normalized))
}

fn schema_names(value: &serde_json::Value) -> AppResult<Vec<String>> {
    let schemas = value.as_array().ok_or_else(|| {
        AppError::BadRequest("allowed_schemas must be an array of schema names".to_string())
    })?;
    schemas
        .iter()
        .map(|schema| {
            let schema = schema.as_str().ok_or_else(|| {
                AppError::BadRequest("allowed_schemas values must be strings".to_string())
            })?;
            let schema = schema.trim();
            if !is_pg_identifier(schema) {
                return Err(AppError::BadRequest(format!(
                    "Invalid Postgres schema name: {schema}"
                )));
            }
            Ok(schema.to_string())
        })
        .collect()
}

fn is_pg_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if value.len() > 63 || !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn quote_pg_identifier(value: &str) -> AppResult<String> {
    if !is_pg_identifier(value) {
        return Err(AppError::BadRequest(format!(
            "Invalid Postgres schema name: {value}"
        )));
    }
    Ok(format!("\"{}\"", value.replace('"', "\"\"")))
}

fn mask_connection_string(connection_string: &str) -> String {
    if let Ok(mut parsed) = url::Url::parse(connection_string) {
        if parsed.password().is_some() {
            let _ = parsed.set_password(Some("redacted"));
        }
        let pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(key, value)| {
                let key_string = key.to_string();
                let lower = key_string.to_ascii_lowercase();
                let value_string = if is_sensitive_connection_param(&lower) {
                    "redacted".to_string()
                } else {
                    value.to_string()
                };
                (key_string, value_string)
            })
            .collect();
        if !pairs.is_empty() {
            parsed.set_query(None);
            {
                let mut query = parsed.query_pairs_mut();
                for (key, value) in pairs {
                    query.append_pair(&key, &value);
                }
            }
        }
        return parsed.to_string();
    }
    let prefix: String = connection_string.chars().take(12).collect();
    format!("{prefix}...")
}

fn is_sensitive_connection_param(key: &str) -> bool {
    [
        "token",
        "secret",
        "pass",
        "key",
        "auth",
        "sig",
        "credential",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn build_visual_sql_with_policies(
    input: &VisualQueryRequest,
    policy_clauses: &[String],
) -> AppResult<String> {
    let dataset = validate_dataset(&input.dataset)?;
    if dataset == "csv_uploads" {
        return Err(AppError::BadRequest(
            "csv_uploads can be queried with the SQL editor".to_string(),
        ));
    }
    let metrics = if input.metrics.is_empty() {
        vec!["count".to_string()]
    } else {
        input.metrics.clone()
    };
    let dimensions = input.dimensions.iter().take(5).cloned().collect::<Vec<_>>();
    let select_dimensions = dimensions
        .iter()
        .map(|dimension| visual_dimension(dataset, dimension))
        .collect::<AppResult<Vec<_>>>()?;
    let select_metrics = metrics
        .iter()
        .map(|metric| visual_metric(dataset, metric))
        .collect::<AppResult<Vec<_>>>()?;
    let end = input.end_at.unwrap_or_else(Utc::now);
    let start = input.start_at.unwrap_or_else(|| end - Duration::days(30));
    if start >= end {
        return Err(AppError::BadRequest(
            "start_at must be before end_at".to_string(),
        ));
    }

    let mut select_parts = Vec::new();
    select_parts.extend(
        select_dimensions
            .iter()
            .map(|(expr, alias)| format!("{expr} AS {alias}")),
    );
    select_parts.extend(
        select_metrics
            .iter()
            .map(|(expr, alias)| format!("{expr} AS {alias}")),
    );
    let table = match dataset {
        "pageviews" => "pageviews",
        "events" => "events",
        "sessions" => "sessions",
        "daily_stats" => "daily_stats",
        _ => unreachable!("validated dataset"),
    };
    let time_col = if dataset == "daily_stats" {
        "date"
    } else {
        "created_at"
    };
    let group_by = if select_dimensions.is_empty() {
        String::new()
    } else {
        format!(
            " GROUP BY {}",
            (1..=select_dimensions.len())
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let order_by = if select_metrics.is_empty() {
        "1".to_string()
    } else {
        (select_dimensions.len() + 1).to_string()
    };

    let policy_sql = if policy_clauses.is_empty() {
        String::new()
    } else {
        format!(" AND {}", policy_clauses.join(" AND "))
    };

    Ok(format!(
        "SELECT {} FROM {table} WHERE project_id = {{{{project_id}}}} \
         AND {time_col} >= '{}' AND {time_col} <= '{}'{}{} ORDER BY {order_by} DESC",
        select_parts.join(", "),
        start.to_rfc3339(),
        end.to_rfc3339(),
        policy_sql,
        group_by
    ))
}

#[cfg(test)]
fn build_drill_through_sql(input: &DrillThroughRequest) -> AppResult<String> {
    build_drill_through_sql_with_policies(input, &[])
}

fn build_drill_through_sql_with_policies(
    input: &DrillThroughRequest,
    policy_clauses: &[String],
) -> AppResult<String> {
    let dataset = validate_dataset(&input.dataset)?;
    let filters = input.filters.as_object().ok_or_else(|| {
        AppError::BadRequest("Drill-through filters must be an object".to_string())
    })?;
    if filters.len() > 10 {
        return Err(AppError::BadRequest(
            "Drill-through supports at most 10 filters".to_string(),
        ));
    }

    let end = input.end_at.unwrap_or_else(Utc::now);
    let start = input.start_at.unwrap_or_else(|| end - Duration::days(30));
    if start >= end {
        return Err(AppError::BadRequest(
            "start_at must be before end_at".to_string(),
        ));
    }

    let mut clauses = vec!["project_id = {{project_id}}".to_string()];
    let (table, time_col, columns) = match dataset {
        "pageviews" => (
            "pageviews",
            "created_at",
            "id, visitor_id, session_id, path, title, referrer_domain, utm_source, utm_medium, utm_campaign, browser, os, device, country, created_at",
        ),
        "events" => (
            "events",
            "created_at",
            "id, visitor_id, session_id, event_name, path, props, revenue_amount, revenue_currency, created_at",
        ),
        "sessions" => (
            "sessions",
            "first_at",
            "id, visitor_id, entry_path, exit_path, referrer_domain, browser, os, device, country, duration_ms, pageview_count, event_count, first_at, last_at",
        ),
        "daily_stats" => (
            "daily_stats",
            "date",
            "date, pageviews, visitors, sessions, bounces, total_duration_ms",
        ),
        "csv_uploads" => (
            "csv_upload_rows",
            "created_at",
            "upload_id, row_number, row_data, created_at",
        ),
        _ => unreachable!("validated dataset"),
    };

    if dataset == "daily_stats" {
        clauses.push(format!(
            "{time_col} >= '{}'::timestamptz::date AND {time_col} <= '{}'::timestamptz::date",
            start.to_rfc3339(),
            end.to_rfc3339()
        ));
    } else {
        clauses.push(format!(
            "{time_col} >= '{}' AND {time_col} <= '{}'",
            start.to_rfc3339(),
            end.to_rfc3339()
        ));
    }

    for (key, value) in filters {
        clauses.push(drill_filter_clause(dataset, key, value)?);
    }
    clauses.extend(policy_clauses.iter().cloned());

    Ok(format!(
        "SELECT {columns} FROM {table} WHERE {} ORDER BY {time_col} DESC",
        clauses.join(" AND ")
    ))
}

fn validate_dataset(dataset: &str) -> AppResult<&'static str> {
    match dataset.trim() {
        "pageviews" => Ok("pageviews"),
        "events" => Ok("events"),
        "sessions" => Ok("sessions"),
        "daily_stats" => Ok("daily_stats"),
        "csv_uploads" => Ok("csv_uploads"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported BI dataset: {other}"
        ))),
    }
}

fn visual_dimension(dataset: &str, dimension: &str) -> AppResult<(&'static str, &'static str)> {
    match (dataset, dimension) {
        ("pageviews", "path") => Ok(("path", "path")),
        ("pageviews", "referrer_domain") => {
            Ok(("COALESCE(referrer_domain, 'Direct')", "referrer_domain"))
        }
        ("pageviews", "utm_source") => Ok(("COALESCE(utm_source, '')", "utm_source")),
        ("pageviews", "utm_medium") => Ok(("COALESCE(utm_medium, '')", "utm_medium")),
        ("pageviews", "utm_campaign") => Ok(("COALESCE(utm_campaign, '')", "utm_campaign")),
        ("pageviews", "date") => Ok(("date_trunc('day', created_at)::date", "date")),
        ("events", "event_name") => Ok(("event_name", "event_name")),
        ("events", "path") => Ok(("COALESCE(path, '')", "path")),
        ("events", "date") => Ok(("date_trunc('day', created_at)::date", "date")),
        ("sessions", "browser") => Ok(("COALESCE(browser, 'Unknown')", "browser")),
        ("sessions", "os") => Ok(("COALESCE(os, 'Unknown')", "os")),
        ("sessions", "device") => Ok(("COALESCE(device, 'desktop')", "device")),
        ("sessions", "country") => Ok(("COALESCE(country, 'XX')", "country")),
        ("sessions", "date") => Ok(("date_trunc('day', first_at)::date", "date")),
        ("daily_stats", "date") => Ok(("date", "date")),
        (_, other) => Err(AppError::BadRequest(format!(
            "Unsupported dimension '{other}' for dataset '{dataset}'"
        ))),
    }
}

fn visual_metric(dataset: &str, metric: &str) -> AppResult<(&'static str, &'static str)> {
    match (dataset, metric) {
        ("pageviews", "count") => Ok(("COUNT(*)::bigint", "count")),
        ("pageviews", "unique_visitors") => {
            Ok(("COUNT(DISTINCT visitor_id)::bigint", "unique_visitors"))
        }
        ("pageviews", "sessions") => Ok(("COUNT(DISTINCT session_id)::bigint", "sessions")),
        ("events", "count") => Ok(("COUNT(*)::bigint", "count")),
        ("events", "unique_visitors") => {
            Ok(("COUNT(DISTINCT visitor_id)::bigint", "unique_visitors"))
        }
        ("events", "revenue") => Ok(("COALESCE(SUM(revenue_amount), 0)::float8", "revenue")),
        ("sessions", "count") => Ok(("COUNT(*)::bigint", "count")),
        ("sessions", "visitors") => Ok(("COUNT(DISTINCT visitor_id)::bigint", "visitors")),
        ("sessions", "avg_duration") => {
            Ok(("COALESCE(AVG(duration_ms), 0)::float8", "avg_duration"))
        }
        ("daily_stats", "pageviews") => Ok(("COALESCE(SUM(pageviews), 0)::bigint", "pageviews")),
        ("daily_stats", "visitors") => Ok(("COALESCE(SUM(visitors), 0)::bigint", "visitors")),
        ("daily_stats", "sessions") => Ok(("COALESCE(SUM(sessions), 0)::bigint", "sessions")),
        (_, other) => Err(AppError::BadRequest(format!(
            "Unsupported metric '{other}' for dataset '{dataset}'"
        ))),
    }
}

fn drill_filter_clause(dataset: &str, key: &str, value: &serde_json::Value) -> AppResult<String> {
    let column = drill_filter_column(dataset, key)?;
    if let Some(items) = value.as_array() {
        let values = items
            .iter()
            .map(sql_literal)
            .collect::<AppResult<Vec<_>>>()?;
        if values.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Filter '{key}' must not be an empty array"
            )));
        }
        return Ok(format!("{column} IN ({})", values.join(", ")));
    }
    Ok(format!("{column} = {}", sql_literal(value)?))
}

async fn active_row_policy_clauses(
    db: &PgPool,
    project_id: Uuid,
    dataset: &str,
) -> AppResult<Vec<String>> {
    let dataset = validate_dataset(dataset)?;
    let policies: Vec<(String, String, serde_json::Value)> = sqlx::query_as(
        "SELECT field, operator, values FROM bi_row_policies \
         WHERE project_id = $1 AND dataset = $2 AND is_active = true \
         ORDER BY created_at ASC",
    )
    .bind(project_id)
    .bind(dataset)
    .fetch_all(db)
    .await?;

    policies
        .into_iter()
        .map(|(field, operator, values)| row_policy_clause(dataset, &field, &operator, &values))
        .collect()
}

fn row_policy_clause(
    dataset: &str,
    field: &str,
    operator: &str,
    values: &serde_json::Value,
) -> AppResult<String> {
    let column = drill_filter_column(dataset, field)?;
    let values = values.as_array().ok_or_else(|| {
        AppError::BadRequest("Stored row policy values must be an array".to_string())
    })?;
    if values.is_empty() {
        return Err(AppError::BadRequest(
            "Stored row policy values cannot be empty".to_string(),
        ));
    }
    let literals = values
        .iter()
        .map(sql_literal)
        .collect::<AppResult<Vec<_>>>()?;

    match operator {
        "eq" => {
            if literals.len() == 1 {
                Ok(format!("{column} = {}", literals[0]))
            } else {
                Ok(format!("{column} IN ({})", literals.join(", ")))
            }
        }
        "neq" => {
            if literals.len() == 1 {
                Ok(format!("{column} <> {}", literals[0]))
            } else {
                Ok(format!("{column} NOT IN ({})", literals.join(", ")))
            }
        }
        "in" => Ok(format!("{column} IN ({})", literals.join(", "))),
        "not_in" => Ok(format!("{column} NOT IN ({})", literals.join(", "))),
        other => Err(AppError::BadRequest(format!(
            "Unsupported row policy operator: {other}"
        ))),
    }
}

fn drill_filter_column(dataset: &str, key: &str) -> AppResult<&'static str> {
    match (dataset, key) {
        ("pageviews", "path") => Ok("path"),
        ("pageviews", "visitor_id") => Ok("visitor_id"),
        ("pageviews", "session_id") => Ok("session_id"),
        ("pageviews", "referrer_domain") => Ok("referrer_domain"),
        ("pageviews", "utm_source") => Ok("utm_source"),
        ("pageviews", "utm_medium") => Ok("utm_medium"),
        ("pageviews", "utm_campaign") => Ok("utm_campaign"),
        ("pageviews", "browser") => Ok("browser"),
        ("pageviews", "os") => Ok("os"),
        ("pageviews", "device") => Ok("device"),
        ("pageviews", "country") => Ok("country"),
        ("events", "event_name") => Ok("event_name"),
        ("events", "path") => Ok("path"),
        ("events", "visitor_id") => Ok("visitor_id"),
        ("events", "session_id") => Ok("session_id"),
        ("events", "revenue_currency") => Ok("revenue_currency"),
        ("sessions", "visitor_id") => Ok("visitor_id"),
        ("sessions", "entry_path") => Ok("entry_path"),
        ("sessions", "exit_path") => Ok("exit_path"),
        ("sessions", "referrer_domain") => Ok("referrer_domain"),
        ("sessions", "browser") => Ok("browser"),
        ("sessions", "os") => Ok("os"),
        ("sessions", "device") => Ok("device"),
        ("sessions", "country") => Ok("country"),
        ("daily_stats", "date") => Ok("date"),
        ("csv_uploads", "upload_id") => Ok("upload_id"),
        ("csv_uploads", "row_number") => Ok("row_number"),
        (_, other) => Err(AppError::BadRequest(format!(
            "Unsupported drill-through filter '{other}' for dataset '{dataset}'"
        ))),
    }
}

fn sql_literal(value: &serde_json::Value) -> AppResult<String> {
    if let Some(value) = value.as_str() {
        let escaped = value.replace('\'', "''");
        return Ok(format!("'{escaped}'"));
    }
    if let Some(value) = value.as_i64() {
        return Ok(value.to_string());
    }
    if let Some(value) = value.as_u64() {
        return Ok(value.to_string());
    }
    if let Some(value) = value.as_f64() {
        if !value.is_finite() {
            return Err(AppError::BadRequest(
                "Numeric filter values must be finite".to_string(),
            ));
        }
        return Ok(value.to_string());
    }
    if let Some(value) = value.as_bool() {
        return Ok(value.to_string());
    }
    Err(AppError::BadRequest(
        "Filter values must be strings, numbers, booleans, or arrays of those values".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        build_drill_through_sql, build_visual_sql_with_policies, embed_token_prefix,
        enforce_table_allowlist, generate_embed_token, hash_embed_token, is_private_adapter_ip,
        mask_connection_string, normalize_allowed_schemas, normalize_embed_origins,
        origin_is_allowed, parse_json_each_row, prepare_external_sql, prepare_safe_sql,
        quote_pg_identifier, row_policy_clause, rows_from_http_json_response,
        validate_clickhouse_sql_scope, validate_csv_upload, validate_database_connection_input,
        validate_embed_input, validate_row_policy_input, BiDatabaseConnectionInput, BiEmbedInput,
        BiRowPolicyInput, CsvUploadInput, DrillThroughRequest, VisualQueryRequest,
    };
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn rejects_unsafe_sql() {
        let project_id = Uuid::new_v4();
        assert!(prepare_safe_sql("DELETE FROM pageviews", project_id).is_err());
        assert!(prepare_safe_sql("SELECT * FROM pageviews", project_id).is_err());
        assert!(prepare_safe_sql(
            "SELECT * FROM pageviews WHERE project_id = {{project_id}}",
            project_id
        )
        .is_ok());
    }

    #[test]
    fn allowlist_blocks_cross_tenant_secret_tables() {
        // The original breach: an always-true placeholder predicate over a secret table.
        assert!(enforce_table_allowlist(
            "SELECT key_hash, project_id FROM api_keys WHERE '{{project_id}}' = '{{project_id}}'"
        )
        .is_err());
        assert!(enforce_table_allowlist(
            "SELECT connection_string FROM bi_database_connections WHERE project_id = {{project_id}}"
        )
        .is_err());
        assert!(
            enforce_table_allowlist("SELECT * FROM projects WHERE id = {{project_id}}").is_err()
        );
        // and end-to-end through prepare_safe_sql
        let pid = Uuid::new_v4();
        assert!(prepare_safe_sql(
            "SELECT key_hash FROM api_keys WHERE '{{project_id}}' = '{{project_id}}'",
            pid
        )
        .is_err());
    }

    #[test]
    fn allowlist_blocks_schema_qualified_and_catalog_access() {
        assert!(enforce_table_allowlist(
            "SELECT * FROM information_schema.tables WHERE project_id = {{project_id}}"
        )
        .is_err());
        assert!(enforce_table_allowlist(
            "SELECT * FROM pg_catalog.pg_tables WHERE project_id = {{project_id}}"
        )
        .is_err());
        assert!(enforce_table_allowlist(
            "SELECT * FROM public.api_keys WHERE project_id = {{project_id}}"
        )
        .is_err());
    }

    #[test]
    fn allowlist_blocks_quoted_identifier_bypass() {
        // double-quoted identifiers must not slip past the allowlist
        assert!(enforce_table_allowlist(
            "SELECT key_hash FROM \"api_keys\" WHERE '{{project_id}}' = '{{project_id}}'"
        )
        .is_err());
    }

    #[test]
    fn allowlist_blocks_subquery_into_secret_table() {
        assert!(enforce_table_allowlist(
            "SELECT * FROM pageviews WHERE project_id = {{project_id}} \
             AND visitor_id IN (SELECT key_hash FROM api_keys)"
        )
        .is_err());
        // even when the secret table is hidden inside a function-call subquery
        assert!(enforce_table_allowlist(
            "SELECT ARRAY(SELECT key_hash FROM api_keys) FROM pageviews \
             WHERE project_id = {{project_id}}"
        )
        .is_err());
        // ... or behind a UNION
        assert!(enforce_table_allowlist(
            "SELECT path FROM pageviews WHERE project_id = {{project_id}} \
             UNION SELECT key_hash FROM api_keys"
        )
        .is_err());
    }

    #[test]
    fn allowlist_blocks_comma_join_after_explicit_join() {
        assert!(enforce_table_allowlist(
            "SELECT p.path FROM pageviews p JOIN events e ON true, api_keys k \
             WHERE '{{project_id}}' = '{{project_id}}'"
        )
        .is_err());
    }

    #[test]
    fn allowlist_blocks_postgres_xml_sql_helpers() {
        for sql in [
            "SELECT query_to_xml('SELECT key_hash FROM api_keys', true, true, '') \
             WHERE '{{project_id}}' = '{{project_id}}'",
            "SELECT pg_catalog.query_to_xml('SELECT key_hash FROM api_keys', true, true, '') \
             WHERE '{{project_id}}' = '{{project_id}}'",
            "SELECT table_to_xml('api_keys'::regclass, true, true, '') \
             WHERE '{{project_id}}' = '{{project_id}}'",
            "SELECT schema_to_xml('public', true, true, '') \
             WHERE '{{project_id}}' = '{{project_id}}'",
            "SELECT database_to_xml(true, true, '') \
             WHERE '{{project_id}}' = '{{project_id}}'",
        ] {
            assert!(enforce_table_allowlist(sql).is_err(), "{sql}");
        }
    }

    #[test]
    fn allowlist_permits_legitimate_analytics_queries() {
        assert!(enforce_table_allowlist(
            "SELECT path, count(*) FROM pageviews WHERE project_id = {{project_id}} GROUP BY 1"
        )
        .is_ok());
        // joins between allowlisted tables
        assert!(enforce_table_allowlist(
            "SELECT s.id FROM sessions s JOIN events e ON e.session_id = s.id \
             WHERE s.project_id = {{project_id}}"
        )
        .is_ok());
        // CTEs are not mistaken for base tables
        assert!(enforce_table_allowlist(
            "WITH recent AS (SELECT * FROM events WHERE project_id = {{project_id}}) \
             SELECT count(*) FROM recent"
        )
        .is_ok());
        // daily rollups + csv uploads
        assert!(enforce_table_allowlist(
            "SELECT date, visitors FROM daily_stats WHERE project_id = {{project_id}}"
        )
        .is_ok());
        // commas in GROUP BY / ORDER BY clauses are not FROM-list separators
        assert!(enforce_table_allowlist(
            "SELECT path, browser, count(*) FROM pageviews \
             WHERE project_id = {{project_id}} GROUP BY path, browser ORDER BY path, browser"
        )
        .is_ok());
        // CTE column aliases should not hide the CTE relation name
        assert!(enforce_table_allowlist(
            "WITH recent(path, cnt) AS ( \
                 SELECT path, count(*) FROM events WHERE project_id = {{project_id}} GROUP BY path \
             ) SELECT path, cnt FROM recent ORDER BY path, cnt"
        )
        .is_ok());
    }

    #[test]
    fn allowlist_handles_extract_from_without_false_positive() {
        // the FROM inside EXTRACT(... FROM ts) is an argument separator, not a table clause
        assert!(enforce_table_allowlist(
            "SELECT extract(dow from created_at) AS d, count(*) FROM pageviews \
             WHERE project_id = {{project_id}} GROUP BY 1"
        )
        .is_ok());
        assert!(enforce_table_allowlist(
            "SELECT substring(path from 1 for 10) FROM pageviews \
             WHERE project_id = {{project_id}}"
        )
        .is_ok());
    }

    #[test]
    fn allowlist_accepts_builder_output() {
        // the visual + drill-through builders must keep passing the allowlist
        let pid = Uuid::new_v4();
        let visual = build_visual_sql_with_policies(
            &VisualQueryRequest {
                dataset: "pageviews".to_string(),
                dimensions: vec!["path".to_string()],
                metrics: vec!["count".to_string()],
                start_at: None,
                end_at: None,
                limit: Some(50),
            },
            &[],
        )
        .expect("visual sql builds");
        assert!(prepare_safe_sql(&visual, pid).is_ok());

        let drill = build_drill_through_sql(&DrillThroughRequest {
            dataset: "events".to_string(),
            filters: json!({"event_name": "signup"}),
            start_at: None,
            end_at: None,
            limit: Some(50),
        })
        .expect("drill sql builds");
        assert!(prepare_safe_sql(&drill, pid).is_ok());
    }

    #[test]
    fn validates_external_sql_without_tenant_placeholder() {
        assert!(prepare_external_sql("SELECT * FROM accounts").is_ok());
        assert!(
            prepare_external_sql("SELECT * FROM accounts WHERE project_id = {{project_id}}")
                .is_err()
        );
        assert!(prepare_external_sql("DROP TABLE accounts").is_err());
        assert!(prepare_external_sql("SELECT * FROM accounts; SELECT 1").is_err());
    }

    #[test]
    fn validates_and_masks_database_connections() {
        let input = validate_database_connection_input(BiDatabaseConnectionInput {
            name: " Warehouse ".to_string(),
            database_type: "POSTGRES".to_string(),
            connection_string: " postgresql://user:secret@example.com/db ".to_string(),
            allowed_schemas: json!(["public", "analytics", "public"]),
            is_active: true,
            created_by: Some(" analyst@example.com ".to_string()),
        })
        .expect("valid connection");

        assert_eq!(input.name, "Warehouse");
        assert_eq!(input.database_type, "postgres");
        assert_eq!(input.allowed_schemas, json!(["public", "analytics"]));
        assert_eq!(input.created_by.as_deref(), Some("analyst@example.com"));
        assert_eq!(
            mask_connection_string(&input.connection_string),
            "postgresql://user:redacted@example.com/db"
        );

        let clickhouse = validate_database_connection_input(BiDatabaseConnectionInput {
            name: " Events lake ".to_string(),
            database_type: "clickhouse".to_string(),
            connection_string:
                " https://user:secret@clickhouse.example.com:8443/?database=pulse&token=abc "
                    .to_string(),
            allowed_schemas: json!(["pulse"]),
            is_active: true,
            created_by: None,
        })
        .expect("valid clickhouse connection");
        assert_eq!(clickhouse.database_type, "clickhouse");
        assert_eq!(
            mask_connection_string(&clickhouse.connection_string),
            "https://user:redacted@clickhouse.example.com:8443/?database=pulse&token=redacted"
        );
        assert_eq!(
            mask_connection_string(
                "https://adapter.example.com/query?access_token=a&client_secret=b&X-Amz-Signature=c&db=pulse"
            ),
            "https://adapter.example.com/query?access_token=redacted&client_secret=redacted&X-Amz-Signature=redacted&db=pulse"
        );

        assert!(
            validate_database_connection_input(BiDatabaseConnectionInput {
                name: " Local adapter ".to_string(),
                database_type: "http_json".to_string(),
                connection_string: "http://localhost:9000/query".to_string(),
                allowed_schemas: json!([]),
                is_active: true,
                created_by: None,
            })
            .is_err()
        );

        let adapter = validate_database_connection_input(BiDatabaseConnectionInput {
            name: " Snowflake adapter ".to_string(),
            database_type: "http_json".to_string(),
            connection_string: "https://adapter.example.com/query".to_string(),
            allowed_schemas: json!([]),
            is_active: true,
            created_by: None,
        })
        .expect("valid adapter connection");
        assert_eq!(adapter.database_type, "http_json");
    }

    #[test]
    fn validates_http_adapter_network_and_clickhouse_scope() {
        assert!(is_private_adapter_ip("127.0.0.1".parse().unwrap()));
        assert!(is_private_adapter_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_adapter_ip("169.254.169.254".parse().unwrap()));
        assert!(!is_private_adapter_ip("8.8.8.8".parse().unwrap()));

        assert!(
            validate_clickhouse_sql_scope("SELECT * FROM pulse.events", &json!(["pulse"])).is_ok()
        );
        assert!(
            validate_clickhouse_sql_scope("SELECT * FROM other.events", &json!(["pulse"])).is_err()
        );
        assert!(validate_clickhouse_sql_scope(
            "SELECT * FROM remote('host', db, table)",
            &json!(["pulse"])
        )
        .is_err());
    }

    #[test]
    fn parses_adapter_rows() {
        let rows = rows_from_http_json_response(json!({"rows": [{"ok": true}]})).unwrap();
        assert_eq!(rows, vec![json!({"ok": true})]);

        let rows = parse_json_each_row("{\"a\":1}\n{\"a\":2}\n").unwrap();
        assert_eq!(rows, vec![json!({"a": 1}), json!({"a": 2})]);
    }

    #[test]
    fn validates_schema_names_for_external_search_path() {
        assert_eq!(
            normalize_allowed_schemas(&json!([])).unwrap(),
            json!(["public"])
        );
        assert_eq!(quote_pg_identifier("analytics").unwrap(), "\"analytics\"");
        assert!(normalize_allowed_schemas(&json!(["bad-name"])).is_err());
    }

    #[test]
    fn validates_bi_embed_inputs_and_origins() {
        let input = validate_embed_input(BiEmbedInput {
            name: " Executive dashboard ".to_string(),
            resource_type: "VISUAL_QUERY".to_string(),
            resource_id: None,
            resource_config: json!({
                "dataset": "pageviews",
                "dimensions": ["path"],
                "metrics": ["count"],
                "limit": 10
            }),
            allowed_origins: vec![
                "https://app.example.com/dashboard".to_string(),
                "https://app.example.com".to_string(),
            ],
            theme: json!({"brand": "Acme", "primary_color": "#2563eb"}),
            is_active: true,
            expires_at: Some(Utc::now() + chrono::Duration::days(7)),
            created_by: Some(" analyst@example.com ".to_string()),
        })
        .expect("valid embed");

        assert_eq!(input.name, "Executive dashboard");
        assert_eq!(input.resource_type, "visual_query");
        assert_eq!(input.allowed_origins, vec!["https://app.example.com"]);
        assert_eq!(input.created_by.as_deref(), Some("analyst@example.com"));
    }

    #[test]
    fn validates_bi_embed_tokens_and_origin_matching() {
        let token = generate_embed_token();
        assert!(token.starts_with("pemb_"));
        assert_eq!(hash_embed_token(&token).len(), 64);
        assert_eq!(
            embed_token_prefix(&token),
            token.chars().take(16).collect::<String>()
        );

        let origins = normalize_embed_origins(&[
            "https://app.example.com/reports".to_string(),
            "http://localhost:3000".to_string(),
        ])
        .unwrap();
        let allowed = json!(origins);
        assert!(origin_is_allowed(&allowed, Some("https://app.example.com")));
        assert!(origin_is_allowed(
            &allowed,
            Some("http://localhost:3000/path")
        ));
        assert!(!origin_is_allowed(
            &allowed,
            Some("https://evil.example.com")
        ));
        assert!(!origin_is_allowed(&allowed, None));
        assert!(origin_is_allowed(&json!(["*"]), None));
    }

    #[test]
    fn builds_visual_query_for_allowed_fields() {
        let sql = build_visual_sql_with_policies(
            &VisualQueryRequest {
                dataset: "pageviews".to_string(),
                dimensions: vec!["path".to_string()],
                metrics: vec!["count".to_string()],
                start_at: Some(Utc::now() - chrono::Duration::days(1)),
                end_at: Some(Utc::now()),
                limit: Some(10),
            },
            &["country = 'US'".to_string()],
        )
        .unwrap();
        assert!(sql.contains("GROUP BY 1"));
        assert!(sql.contains("{{project_id}}"));
        assert!(sql.contains("country = 'US'"));
    }

    #[test]
    fn builds_drill_through_query_for_allowed_filters() {
        let sql = build_drill_through_sql(&DrillThroughRequest {
            dataset: "events".to_string(),
            filters: json!({
                "event_name": "signup",
                "path": ["/pricing", "/signup"]
            }),
            start_at: Some(Utc::now() - chrono::Duration::days(1)),
            end_at: Some(Utc::now()),
            limit: Some(50),
        })
        .unwrap();

        assert!(sql.contains("FROM events"));
        assert!(sql.contains("project_id = {{project_id}}"));
        assert!(sql.contains("event_name = 'signup'"));
        assert!(sql.contains("path IN ('/pricing', '/signup')"));
    }

    #[test]
    fn rejects_unsupported_drill_through_filters() {
        assert!(build_drill_through_sql(&DrillThroughRequest {
            dataset: "events".to_string(),
            filters: json!({"props": {"plan": "pro"}}),
            start_at: Some(Utc::now() - chrono::Duration::days(1)),
            end_at: Some(Utc::now()),
            limit: Some(50),
        })
        .is_err());
    }

    #[test]
    fn validates_and_builds_row_policy_clauses() {
        let policy = validate_row_policy_input(BiRowPolicyInput {
            name: "US only".to_string(),
            dataset: "pageviews".to_string(),
            field: "country".to_string(),
            operator: "in".to_string(),
            values: json!(["US", "CA"]),
            is_active: true,
            created_by: Some("analyst@example.com".to_string()),
        })
        .expect("valid policy");

        assert_eq!(policy.operator, "in");
        let clause = row_policy_clause(
            &policy.dataset,
            &policy.field,
            &policy.operator,
            &policy.values,
        )
        .expect("policy clause");
        assert_eq!(clause, "country IN ('US', 'CA')");
    }

    #[test]
    fn validates_csv_rows_are_objects() {
        assert!(validate_csv_upload(CsvUploadInput {
            name: "Accounts".to_string(),
            description: None,
            columns: vec!["account_id".to_string()],
            rows: vec![json!({"account_id": "a1"})],
            uploaded_by: None,
        })
        .is_ok());
        assert!(validate_csv_upload(CsvUploadInput {
            name: "Bad".to_string(),
            description: None,
            columns: vec!["account_id".to_string()],
            rows: vec![json!(["a1"])],
            uploaded_by: None,
        })
        .is_err());
    }
}
