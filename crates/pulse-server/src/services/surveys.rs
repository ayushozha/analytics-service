use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Survey {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub questions: serde_json::Value,
    pub trigger_config: serde_json::Value,
    pub appearance: serde_json::Value,
    pub status: String,
    pub response_limit: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SurveyResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub survey_id: Uuid,
    pub visitor_id: String,
    pub session_id: Option<Uuid>,
    pub answers: serde_json::Value,
    pub completed: bool,
    pub path: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SurveyStats {
    pub total_responses: i64,
    pub completed_responses: i64,
    pub completion_rate: f64,
    pub question_stats: Vec<serde_json::Value>,
}

const SURVEY_COLUMNS: &str = "id, project_id, name, questions, trigger_config, appearance, \
    status, response_limit, started_at, ended_at, created_at, updated_at";

/// Create a new survey.
pub async fn create_survey(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    questions: &serde_json::Value,
    trigger_config: &serde_json::Value,
    appearance: &serde_json::Value,
) -> Result<Survey, sqlx::Error> {
    let survey: Survey = sqlx::query_as(&format!(
        "INSERT INTO surveys (project_id, name, questions, trigger_config, appearance) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING {SURVEY_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name)
    .bind(questions)
    .bind(trigger_config)
    .bind(appearance)
    .fetch_one(db)
    .await?;

    Ok(survey)
}

/// List all surveys for a project.
pub async fn list_surveys(
    db: &PgPool,
    project_id: Uuid,
) -> Result<Vec<Survey>, sqlx::Error> {
    let surveys: Vec<Survey> = sqlx::query_as(&format!(
        "SELECT {SURVEY_COLUMNS} FROM surveys WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(surveys)
}

/// Get a single survey by ID.
pub async fn get_survey(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
) -> Result<Option<Survey>, sqlx::Error> {
    let survey: Option<Survey> = sqlx::query_as(&format!(
        "SELECT {SURVEY_COLUMNS} FROM surveys WHERE id = $1 AND project_id = $2"
    ))
    .bind(survey_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    Ok(survey)
}

/// Update survey details.
pub async fn update_survey(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
    name: &str,
    questions: &serde_json::Value,
    trigger_config: &serde_json::Value,
    appearance: &serde_json::Value,
) -> Result<Survey, sqlx::Error> {
    let survey: Survey = sqlx::query_as(&format!(
        "UPDATE surveys SET name = $1, questions = $2, trigger_config = $3, appearance = $4, \
         updated_at = NOW() \
         WHERE id = $5 AND project_id = $6 \
         RETURNING {SURVEY_COLUMNS}"
    ))
    .bind(name)
    .bind(questions)
    .bind(trigger_config)
    .bind(appearance)
    .bind(survey_id)
    .bind(project_id)
    .fetch_one(db)
    .await?;

    Ok(survey)
}

/// Delete a survey.
pub async fn delete_survey(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM surveys WHERE id = $1 AND project_id = $2")
            .bind(survey_id)
            .bind(project_id)
            .execute(db)
            .await?;

    Ok(result.rows_affected() > 0)
}

/// Update survey status (draft, active, paused, completed).
pub async fn update_survey_status(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
    status: &str,
) -> Result<Survey, sqlx::Error> {
    let now = Utc::now();

    let (started_at_expr, ended_at_expr) = match status {
        "active" => (
            "COALESCE(started_at, $4::timestamptz)",
            "NULL::timestamptz",
        ),
        "completed" => ("started_at", "$4::timestamptz"),
        _ => ("started_at", "ended_at"),
    };

    let query = format!(
        "UPDATE surveys SET status = $1, started_at = {started_at_expr}, \
         ended_at = {ended_at_expr}, updated_at = NOW() \
         WHERE id = $2 AND project_id = $3 \
         RETURNING {SURVEY_COLUMNS}"
    );

    let survey: Survey = sqlx::query_as(&query)
        .bind(status)
        .bind(survey_id)
        .bind(project_id)
        .bind(now)
        .fetch_one(db)
        .await?;

    Ok(survey)
}

/// Record a survey response.
pub async fn record_response(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
    visitor_id: &str,
    session_id: Option<Uuid>,
    answers: &serde_json::Value,
    completed: bool,
    path: Option<&str>,
) -> Result<SurveyResponse, sqlx::Error> {
    let response: SurveyResponse = sqlx::query_as(
        "INSERT INTO survey_responses (project_id, survey_id, visitor_id, session_id, answers, completed, path) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id, project_id, survey_id, visitor_id, session_id, answers, completed, path, created_at",
    )
    .bind(project_id)
    .bind(survey_id)
    .bind(visitor_id)
    .bind(session_id)
    .bind(answers)
    .bind(completed)
    .bind(path)
    .fetch_one(db)
    .await?;

    Ok(response)
}

/// Get survey responses with pagination.
pub async fn get_survey_responses(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<SurveyResponse>, sqlx::Error> {
    let responses: Vec<SurveyResponse> = sqlx::query_as(
        "SELECT id, project_id, survey_id, visitor_id, session_id, answers, completed, path, created_at \
         FROM survey_responses WHERE survey_id = $1 AND project_id = $2 \
         ORDER BY created_at DESC LIMIT $3 OFFSET $4",
    )
    .bind(survey_id)
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(responses)
}

/// Get survey statistics: total responses, completion rate, per-question aggregation.
pub async fn get_survey_stats(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
) -> Result<SurveyStats, sqlx::Error> {
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM survey_responses WHERE survey_id = $1 AND project_id = $2",
    )
    .bind(survey_id)
    .bind(project_id)
    .fetch_one(db)
    .await?;

    let completed: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM survey_responses \
         WHERE survey_id = $1 AND project_id = $2 AND completed = true",
    )
    .bind(survey_id)
    .bind(project_id)
    .fetch_one(db)
    .await?;

    let completion_rate = if total.0 > 0 {
        (completed.0 as f64 / total.0 as f64) * 100.0
    } else {
        0.0
    };

    // Per-question aggregation
    let answers: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT answers FROM survey_responses WHERE survey_id = $1 AND project_id = $2",
    )
    .bind(survey_id)
    .bind(project_id)
    .fetch_all(db)
    .await?;

    let mut question_counts: std::collections::HashMap<
        String,
        std::collections::HashMap<String, i64>,
    > = std::collections::HashMap::new();

    for (answer_data,) in &answers {
        if let Some(arr) = answer_data.as_array() {
            for item in arr {
                let q = item
                    .get("question")
                    .and_then(|q| q.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let a = item
                    .get("answer")
                    .map(|a| {
                        if let Some(s) = a.as_str() {
                            s.to_string()
                        } else {
                            a.to_string()
                        }
                    })
                    .unwrap_or_else(|| "no_answer".to_string());

                *question_counts
                    .entry(q)
                    .or_default()
                    .entry(a)
                    .or_insert(0) += 1;
            }
        }
    }

    let question_stats: Vec<serde_json::Value> = question_counts
        .into_iter()
        .map(|(question, answer_counts)| {
            serde_json::json!({
                "question": question,
                "answer_distribution": answer_counts,
            })
        })
        .collect();

    Ok(SurveyStats {
        total_responses: total.0,
        completed_responses: completed.0,
        completion_rate,
        question_stats,
    })
}

/// Get all active surveys for a project (for frontend display).
pub async fn get_active_surveys(
    db: &PgPool,
    project_id: Uuid,
) -> Result<Vec<Survey>, sqlx::Error> {
    let surveys: Vec<Survey> = sqlx::query_as(&format!(
        "SELECT {SURVEY_COLUMNS} FROM surveys WHERE project_id = $1 AND status = 'active' \
         ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(surveys)
}
