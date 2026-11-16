//! Annotations (PostHog `Annotation`): dated notes shown on insight charts,
//! either project-wide or attached to one saved insight.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const MAX_CONTENT_LEN: usize = 4_000;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Annotation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub content: String,
    pub date_marker: DateTime<Utc>,
    pub scope: String,
    pub insight_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const COLUMNS: &str =
    "id, project_id, content, date_marker, scope, insight_id, created_at, updated_at";

#[derive(Debug, Clone, Deserialize)]
pub struct AnnotationInput {
    pub content: String,
    pub date_marker: DateTime<Utc>,
    /// When set, the annotation belongs to this insight only.
    #[serde(default)]
    pub insight_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnnotationPatch {
    pub content: Option<String>,
    pub date_marker: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnnotationListQuery {
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    /// Project-wide annotations plus those of this insight.
    pub insight_id: Option<Uuid>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn validate_content(c: &str) -> AppResult<()> {
    if c.trim().is_empty() || c.len() > MAX_CONTENT_LEN {
        return Err(AppError::BadRequest(format!(
            "content must be 1-{MAX_CONTENT_LEN} characters"
        )));
    }
    Ok(())
}

pub async fn create(db: &PgPool, project_id: Uuid, a: &AnnotationInput) -> AppResult<Annotation> {
    validate_content(&a.content)?;
    if let Some(insight) = a.insight_id {
        // The insight must belong to this project.
        super::insights::saved::get(db, project_id, insight).await?;
    }
    Ok(sqlx::query_as(&format!(
        "INSERT INTO annotations (project_id, content, date_marker, scope, insight_id) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {COLUMNS}"
    ))
    .bind(project_id)
    .bind(a.content.trim())
    .bind(a.date_marker)
    .bind(if a.insight_id.is_some() {
        "insight"
    } else {
        "project"
    })
    .bind(a.insight_id)
    .fetch_one(db)
    .await?)
}

pub async fn list(
    db: &PgPool,
    project_id: Uuid,
    f: &AnnotationListQuery,
) -> AppResult<Vec<Annotation>> {
    let limit = f.limit.unwrap_or(500);
    if !(1..=1000).contains(&limit) || f.offset.is_some_and(|o| o < 0) {
        return Err(AppError::BadRequest(
            "limit must be 1-1000 and offset >= 0".into(),
        ));
    }
    let search = f.search.as_deref().map(|s| {
        format!(
            "%{}%",
            s.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        )
    });
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM annotations WHERE project_id = $1 \
         AND ($2::timestamptz IS NULL OR date_marker >= $2) \
         AND ($3::timestamptz IS NULL OR date_marker < $3) \
         AND (scope = 'project' OR insight_id = $4) \
         AND ($5::text IS NULL OR content ILIKE $5) \
         ORDER BY date_marker, id LIMIT $6 OFFSET $7"
    ))
    .bind(project_id)
    .bind(f.date_from)
    .bind(f.date_to)
    .bind(f.insight_id)
    .bind(search)
    .bind(limit)
    .bind(f.offset.unwrap_or(0))
    .fetch_all(db)
    .await?)
}

pub async fn update(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
    p: &AnnotationPatch,
) -> AppResult<Annotation> {
    if let Some(c) = &p.content {
        validate_content(c)?;
    }
    sqlx::query_as(&format!(
        "UPDATE annotations SET content = COALESCE($3, content), \
         date_marker = COALESCE($4, date_marker), updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(project_id)
    .bind(p.content.as_deref().map(str::trim))
    .bind(p.date_marker)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Annotation not found".into()))
}

pub async fn delete(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<()> {
    let r = sqlx::query("DELETE FROM annotations WHERE id = $1 AND project_id = $2")
        .bind(id)
        .bind(project_id)
        .execute(db)
        .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::NotFound("Annotation not found".into()));
    }
    Ok(())
}
