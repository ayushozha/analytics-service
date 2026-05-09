use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub project_id: Uuid,
    pub actor: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub async fn record_audit_log(
    db: &PgPool,
    project_id: Uuid,
    actor: &str,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    metadata: serde_json::Value,
) -> AppResult<AuditLog> {
    let log: AuditLog = sqlx::query_as(
        "INSERT INTO audit_logs \
         (project_id, actor, action, target_type, target_id, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, project_id, actor, action, target_type, target_id, metadata, created_at",
    )
    .bind(project_id)
    .bind(actor)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(metadata)
    .fetch_one(db)
    .await?;

    Ok(log)
}

pub async fn list_audit_logs(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AuditLog>> {
    let logs: Vec<AuditLog> = sqlx::query_as(
        "SELECT id, project_id, actor, action, target_type, target_id, metadata, created_at \
         FROM audit_logs \
         WHERE project_id = $1 \
         ORDER BY created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(logs)
}
