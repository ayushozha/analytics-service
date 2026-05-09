use chrono::{DateTime, Utc};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SharedDashboard {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub token: String,
    pub password_hash: Option<String>,
    pub modules: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateSharedDashboardInput {
    pub name: String,
    pub modules: Vec<String>,
    pub password: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

fn generate_token() -> String {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    hex::encode(random_bytes)
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

/// Create a new shared dashboard link.
pub async fn create_shared_dashboard(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    modules: &[String],
    password: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<SharedDashboard, sqlx::Error> {
    let token = generate_token();
    let password_hash = password.map(hash_password);

    let dashboard: SharedDashboard = sqlx::query_as(
        "INSERT INTO shared_dashboards (project_id, name, token, password_hash, modules, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, project_id, name, token, password_hash, modules, expires_at, created_at",
    )
    .bind(project_id)
    .bind(name)
    .bind(&token)
    .bind(&password_hash)
    .bind(modules)
    .bind(expires_at)
    .fetch_one(db)
    .await?;

    Ok(dashboard)
}

/// List all shared dashboards for a project.
pub async fn list_shared_dashboards(
    db: &PgPool,
    project_id: Uuid,
) -> Result<Vec<SharedDashboard>, sqlx::Error> {
    let dashboards: Vec<SharedDashboard> = sqlx::query_as(
        "SELECT id, project_id, name, token, password_hash, modules, expires_at, created_at \
         FROM shared_dashboards WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(dashboards)
}

/// Delete a shared dashboard.
pub async fn delete_shared_dashboard(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM shared_dashboards WHERE id = $1 AND project_id = $2")
        .bind(id)
        .bind(project_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Resolve a shared dashboard by its token.
#[allow(dead_code)]
pub async fn resolve_shared_token(
    db: &PgPool,
    token: &str,
) -> Result<Option<SharedDashboard>, sqlx::Error> {
    let dashboard: Option<SharedDashboard> = sqlx::query_as(
        "SELECT id, project_id, name, token, password_hash, modules, expires_at, created_at \
         FROM shared_dashboards WHERE token = $1",
    )
    .bind(token)
    .fetch_optional(db)
    .await?;

    // Check expiration
    if let Some(ref d) = dashboard {
        if let Some(expires) = d.expires_at {
            if expires < Utc::now() {
                return Ok(None);
            }
        }
    }

    Ok(dashboard)
}

/// Verify password for a shared dashboard.
#[allow(dead_code)]
pub fn verify_shared_password(dashboard: &SharedDashboard, password: &str) -> bool {
    match &dashboard.password_hash {
        Some(stored_hash) => {
            let provided_hash = hash_password(password);
            stored_hash == &provided_hash
        }
        None => true, // No password set, always passes
    }
}
