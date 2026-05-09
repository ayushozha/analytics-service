use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct ClickPoint {
    pub x: f64,
    pub y: f64,
    pub count: i64,
    pub element_selector: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PageClickStats {
    pub path: String,
    pub total_clicks: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Serialize)]
pub struct FrictionSignal {
    pub signal_type: String,
    pub severity: String,
    pub path: String,
    pub element_selector: Option<String>,
    pub visitor_id: String,
    pub session_id: Option<Uuid>,
    pub occurrences: i64,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VisualEventLabel {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub event_name: String,
    pub path_pattern: String,
    pub element_selector: String,
    pub properties: serde_json::Value,
    pub status: String,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VisualEventLabelInput {
    pub name: String,
    pub event_name: String,
    #[serde(default = "default_path_pattern")]
    pub path_pattern: String,
    pub element_selector: String,
    #[serde(default = "default_properties")]
    pub properties: serde_json::Value,
    #[serde(default = "default_label_status")]
    pub status: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VisualEventLabelStats {
    pub label_id: Uuid,
    pub name: String,
    pub event_name: String,
    pub path_pattern: String,
    pub element_selector: String,
    pub total_clicks: i64,
    pub unique_visitors: i64,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

const VISUAL_LABEL_COLUMNS: &str = "id, project_id, name, event_name, path_pattern, \
    element_selector, properties, status, created_by, created_at, updated_at";

fn default_path_pattern() -> String {
    "*".to_string()
}

fn default_properties() -> serde_json::Value {
    serde_json::json!({})
}

fn default_label_status() -> String {
    "active".to_string()
}

/// Get click heatmap for a specific page path.
/// Aggregates nearby clicks by rounding coordinates to integer grid points.
pub async fn get_click_heatmap(
    db: &PgPool,
    project_id: Uuid,
    path: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<ClickPoint>, sqlx::Error> {
    let rows: Vec<(f64, f64, i64, Option<String>)> = sqlx::query_as(
        "SELECT ROUND(x)::double precision, ROUND(y)::double precision, \
         COUNT(*)::bigint, \
         (array_agg(element_selector) FILTER (WHERE element_selector IS NOT NULL))[1] \
         FROM click_events WHERE project_id = $1 AND path = $2 \
         AND created_at >= $3 AND created_at <= $4 \
         GROUP BY ROUND(x), ROUND(y) ORDER BY 3 DESC",
    )
    .bind(project_id)
    .bind(path)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| ClickPoint {
            x: r.0,
            y: r.1,
            count: r.2,
            element_selector: r.3,
        })
        .collect();

    Ok(results)
}

pub async fn list_visual_event_labels(
    db: &PgPool,
    project_id: Uuid,
) -> AppResult<Vec<VisualEventLabel>> {
    let labels = sqlx::query_as(&format!(
        "SELECT {VISUAL_LABEL_COLUMNS} FROM visual_event_labels \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(labels)
}

pub async fn create_visual_event_label(
    db: &PgPool,
    project_id: Uuid,
    input: VisualEventLabelInput,
) -> AppResult<VisualEventLabel> {
    let input = validate_visual_event_label_input(input)?;
    let label = sqlx::query_as(&format!(
        "INSERT INTO visual_event_labels \
         (project_id, name, event_name, path_pattern, element_selector, properties, status, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING {VISUAL_LABEL_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.event_name)
    .bind(&input.path_pattern)
    .bind(&input.element_selector)
    .bind(&input.properties)
    .bind(&input.status)
    .bind(&input.created_by)
    .fetch_one(db)
    .await?;
    Ok(label)
}

pub async fn update_visual_event_label(
    db: &PgPool,
    project_id: Uuid,
    label_id: Uuid,
    input: VisualEventLabelInput,
) -> AppResult<VisualEventLabel> {
    let input = validate_visual_event_label_input(input)?;
    let label = sqlx::query_as(&format!(
        "UPDATE visual_event_labels SET \
           name = $3, event_name = $4, path_pattern = $5, element_selector = $6, \
           properties = $7, status = $8, created_by = $9, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING {VISUAL_LABEL_COLUMNS}"
    ))
    .bind(label_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.event_name)
    .bind(&input.path_pattern)
    .bind(&input.element_selector)
    .bind(&input.properties)
    .bind(&input.status)
    .bind(&input.created_by)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Visual event label not found".to_string()))?;
    Ok(label)
}

pub async fn delete_visual_event_label(
    db: &PgPool,
    project_id: Uuid,
    label_id: Uuid,
) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM visual_event_labels WHERE id = $1 AND project_id = $2")
        .bind(label_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Visual event label not found".to_string(),
        ));
    }
    Ok(())
}

pub async fn get_visual_event_label_stats(
    db: &PgPool,
    project_id: Uuid,
    label_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<VisualEventLabelStats> {
    let row: Option<(
        Uuid,
        String,
        String,
        String,
        String,
        i64,
        i64,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        "SELECT l.id, l.name, l.event_name, l.path_pattern, l.element_selector, \
                COUNT(c.*)::bigint, COUNT(DISTINCT c.visitor_id)::bigint, \
                MIN(c.created_at), MAX(c.created_at) \
         FROM visual_event_labels l \
         LEFT JOIN click_events c ON c.project_id = l.project_id \
           AND c.created_at >= $3 AND c.created_at <= $4 \
           AND c.element_selector = l.element_selector \
           AND (l.path_pattern = '*' OR c.path = l.path_pattern OR c.path LIKE REPLACE(l.path_pattern, '*', '%')) \
         WHERE l.project_id = $1 AND l.id = $2 \
         GROUP BY l.id, l.name, l.event_name, l.path_pattern, l.element_selector",
    )
    .bind(project_id)
    .bind(label_id)
    .bind(start)
    .bind(end)
    .fetch_optional(db)
    .await?;

    row.map(visual_label_stats_from_row)
        .ok_or_else(|| AppError::NotFound("Visual event label not found".to_string()))
}

pub async fn list_visual_event_label_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> AppResult<Vec<VisualEventLabelStats>> {
    let rows: Vec<(
        Uuid,
        String,
        String,
        String,
        String,
        i64,
        i64,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        "SELECT l.id, l.name, l.event_name, l.path_pattern, l.element_selector, \
                COUNT(c.*)::bigint, COUNT(DISTINCT c.visitor_id)::bigint, \
                MIN(c.created_at), MAX(c.created_at) \
         FROM visual_event_labels l \
         LEFT JOIN click_events c ON c.project_id = l.project_id \
           AND c.created_at >= $2 AND c.created_at <= $3 \
           AND c.element_selector = l.element_selector \
           AND (l.path_pattern = '*' OR c.path = l.path_pattern OR c.path LIKE REPLACE(l.path_pattern, '*', '%')) \
         WHERE l.project_id = $1 AND l.status = 'active' \
         GROUP BY l.id, l.name, l.event_name, l.path_pattern, l.element_selector \
         ORDER BY COUNT(c.*) DESC, l.created_at DESC LIMIT $4",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(limit.clamp(1, 100))
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(visual_label_stats_from_row).collect())
}

fn visual_label_stats_from_row(
    row: (
        Uuid,
        String,
        String,
        String,
        String,
        i64,
        i64,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    ),
) -> VisualEventLabelStats {
    VisualEventLabelStats {
        label_id: row.0,
        name: row.1,
        event_name: row.2,
        path_pattern: row.3,
        element_selector: row.4,
        total_clicks: row.5,
        unique_visitors: row.6,
        first_seen_at: row.7,
        last_seen_at: row.8,
    }
}

fn validate_visual_event_label_input(
    mut input: VisualEventLabelInput,
) -> AppResult<VisualEventLabelInput> {
    input.name = input.name.trim().to_string();
    input.event_name = input.event_name.trim().to_string();
    input.path_pattern = input.path_pattern.trim().to_string();
    input.element_selector = input.element_selector.trim().to_string();
    input.status = input.status.trim().to_ascii_lowercase();
    input.created_by = input
        .created_by
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if input.name.is_empty() {
        return Err(AppError::BadRequest("Label name is required".to_string()));
    }
    if input.event_name.is_empty() {
        return Err(AppError::BadRequest("event_name is required".to_string()));
    }
    if input.path_pattern.is_empty() {
        input.path_pattern = "*".to_string();
    }
    if input.element_selector.is_empty() {
        return Err(AppError::BadRequest(
            "element_selector is required".to_string(),
        ));
    }
    if !input.properties.is_object() {
        return Err(AppError::BadRequest(
            "Label properties must be an object".to_string(),
        ));
    }
    if !matches!(input.status.as_str(), "active" | "paused" | "archived") {
        return Err(AppError::BadRequest(format!(
            "Unsupported label status: {}",
            input.status
        )));
    }
    Ok(input)
}

/// Detect rage-click clusters from click heatmap events.
///
/// A rage click is represented as at least three clicks on the same page and
/// selector by the same visitor/session inside a five-second window.
pub async fn detect_friction_signals(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    path: Option<&str>,
    limit: i64,
) -> Result<Vec<FrictionSignal>, sqlx::Error> {
    let rows: Vec<(
        String,
        Option<String>,
        String,
        Option<Uuid>,
        i64,
        DateTime<Utc>,
        DateTime<Utc>,
    )> = sqlx::query_as(
        "WITH click_windows AS (
           SELECT path, element_selector, visitor_id, session_id, created_at,
                  FLOOR(EXTRACT(EPOCH FROM created_at) / 5) AS window_id
           FROM click_events
           WHERE project_id = $1
             AND created_at >= $2
             AND created_at <= $3
             AND ($4::text IS NULL OR path = $4)
         )
         SELECT path, element_selector, visitor_id, session_id,
                COUNT(*)::bigint, MIN(created_at), MAX(created_at)
         FROM click_windows
         GROUP BY path, element_selector, visitor_id, session_id, window_id
         HAVING COUNT(*) >= 3
         ORDER BY COUNT(*) DESC, MAX(created_at) DESC
         LIMIT $5",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(path)
    .bind(limit.clamp(1, 100))
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(path, element_selector, visitor_id, session_id, occurrences, first, last)| {
                FrictionSignal {
                    signal_type: "rage_click".to_string(),
                    severity: if occurrences >= 6 {
                        "high".to_string()
                    } else {
                        "medium".to_string()
                    },
                    path,
                    element_selector,
                    visitor_id,
                    session_id,
                    occurrences,
                    first_seen_at: first,
                    last_seen_at: last,
                }
            },
        )
        .collect())
}

/// Get pages with the most clicks.
pub async fn get_click_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<PageClickStats>, sqlx::Error> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT path, COUNT(*)::bigint, COUNT(DISTINCT visitor_id)::bigint \
         FROM click_events WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         GROUP BY path ORDER BY 2 DESC LIMIT $4",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| PageClickStats {
            path: r.0,
            total_clicks: r.1,
            unique_visitors: r.2,
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::{validate_visual_event_label_input, VisualEventLabelInput};
    use serde_json::json;

    #[test]
    fn validates_visual_event_labels() {
        let label = validate_visual_event_label_input(VisualEventLabelInput {
            name: " CTA Click ".to_string(),
            event_name: " pricing_cta_click ".to_string(),
            path_pattern: " /pricing* ".to_string(),
            element_selector: " button.cta-primary ".to_string(),
            properties: json!({"area": "hero"}),
            status: "Active".to_string(),
            created_by: Some(" analyst@example.com ".to_string()),
        })
        .expect("valid label");

        assert_eq!(label.name, "CTA Click");
        assert_eq!(label.event_name, "pricing_cta_click");
        assert_eq!(label.path_pattern, "/pricing*");
        assert_eq!(label.status, "active");
        assert_eq!(label.created_by.as_deref(), Some("analyst@example.com"));
    }

    #[test]
    fn rejects_invalid_visual_event_labels() {
        assert!(validate_visual_event_label_input(VisualEventLabelInput {
            name: " ".to_string(),
            event_name: "signup".to_string(),
            path_pattern: "*".to_string(),
            element_selector: "button".to_string(),
            properties: json!({}),
            status: "active".to_string(),
            created_by: None,
        })
        .is_err());

        assert!(validate_visual_event_label_input(VisualEventLabelInput {
            name: "Signup".to_string(),
            event_name: "signup".to_string(),
            path_pattern: "*".to_string(),
            element_selector: " ".to_string(),
            properties: json!([]),
            status: "active".to_string(),
            created_by: None,
        })
        .is_err());
    }
}
