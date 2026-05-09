use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

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

#[derive(Debug, Serialize)]
pub struct NpsReport {
    pub survey_id: Uuid,
    pub question_id: Option<String>,
    pub total_responses: i64,
    pub scored_responses: i64,
    pub promoters: i64,
    pub passives: i64,
    pub detractors: i64,
    pub nps_score: f64,
}

#[derive(Debug, Serialize)]
pub struct SentimentExample {
    pub response_id: Uuid,
    pub sentiment: String,
    pub score: i32,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct SentimentReport {
    pub survey_id: Uuid,
    pub question_id: Option<String>,
    pub total_text_responses: i64,
    pub positive: i64,
    pub neutral: i64,
    pub negative: i64,
    pub sentiment_score: f64,
    pub examples: Vec<SentimentExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InAppGuide {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub guide_type: String,
    pub steps: serde_json::Value,
    pub targeting: serde_json::Value,
    pub appearance: serde_json::Value,
    pub status: String,
    pub priority: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GuideEvent {
    pub id: Uuid,
    pub project_id: Uuid,
    pub guide_id: Uuid,
    pub visitor_id: String,
    pub event_type: String,
    pub step_id: Option<String>,
    pub metadata: serde_json::Value,
    pub path: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GuideInput {
    pub name: String,
    #[serde(default = "default_guide_type")]
    pub guide_type: String,
    #[serde(default = "default_array")]
    pub steps: serde_json::Value,
    #[serde(default = "default_object")]
    pub targeting: serde_json::Value,
    #[serde(default = "default_object")]
    pub appearance: serde_json::Value,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GuideEventInput {
    pub visitor_id: String,
    pub event_type: String,
    pub step_id: Option<String>,
    #[serde(default = "default_object")]
    pub metadata: serde_json::Value,
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuideStats {
    pub guide_id: Uuid,
    pub shown: i64,
    pub started: i64,
    pub completed: i64,
    pub dismissed: i64,
    pub converted: i64,
    pub completion_rate: f64,
    pub dismissal_rate: f64,
}

const SURVEY_COLUMNS: &str = "id, project_id, name, questions, trigger_config, appearance, \
    status, response_limit, started_at, ended_at, created_at, updated_at";
const GUIDE_COLUMNS: &str = "id, project_id, name, guide_type, steps, targeting, appearance, \
    status, priority, started_at, ended_at, created_at, updated_at";
const GUIDE_EVENT_COLUMNS: &str = "id, project_id, guide_id, visitor_id, event_type, step_id, \
    metadata, path, created_at";

fn default_guide_type() -> String {
    "tour".to_string()
}

fn default_object() -> serde_json::Value {
    serde_json::json!({})
}

fn default_array() -> serde_json::Value {
    serde_json::json!([])
}

/// Create a new survey.
pub async fn create_survey(
    db: &PgPool,
    project_id: Uuid,
    name: &str,
    questions: &serde_json::Value,
    trigger_config: &serde_json::Value,
    appearance: &serde_json::Value,
    response_limit: Option<i32>,
) -> Result<Survey, sqlx::Error> {
    let survey: Survey = sqlx::query_as(&format!(
        "INSERT INTO surveys (project_id, name, questions, trigger_config, appearance, response_limit) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {SURVEY_COLUMNS}"
    ))
    .bind(project_id)
    .bind(name)
    .bind(questions)
    .bind(trigger_config)
    .bind(appearance)
    .bind(response_limit)
    .fetch_one(db)
    .await?;

    Ok(survey)
}

/// List all surveys for a project.
pub async fn list_surveys(db: &PgPool, project_id: Uuid) -> Result<Vec<Survey>, sqlx::Error> {
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
    response_limit: Option<i32>,
) -> Result<Survey, sqlx::Error> {
    let survey: Survey = sqlx::query_as(&format!(
        "UPDATE surveys SET name = $1, questions = $2, trigger_config = $3, appearance = $4, \
         response_limit = $5, \
         updated_at = NOW() \
         WHERE id = $6 AND project_id = $7 \
         RETURNING {SURVEY_COLUMNS}"
    ))
    .bind(name)
    .bind(questions)
    .bind(trigger_config)
    .bind(appearance)
    .bind(response_limit)
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
    let result = sqlx::query("DELETE FROM surveys WHERE id = $1 AND project_id = $2")
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
        "active" => ("COALESCE(started_at, $4::timestamptz)", "NULL::timestamptz"),
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
         SELECT $1, s.id, $3, $4, $5, $6, $7 \
         FROM surveys s \
         WHERE s.id = $2 AND s.project_id = $1 AND s.status = 'active' \
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

                *question_counts.entry(q).or_default().entry(a).or_insert(0) += 1;
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
pub async fn get_active_surveys(db: &PgPool, project_id: Uuid) -> Result<Vec<Survey>, sqlx::Error> {
    let surveys: Vec<Survey> = sqlx::query_as(&format!(
        "SELECT {SURVEY_COLUMNS} FROM surveys WHERE project_id = $1 AND status = 'active' \
         ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;

    Ok(surveys)
}

pub async fn get_nps_report(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
    question_id: Option<&str>,
) -> AppResult<NpsReport> {
    ensure_survey_exists(db, project_id, survey_id).await?;
    let responses = response_answers(db, project_id, survey_id).await?;

    let mut promoters = 0;
    let mut passives = 0;
    let mut detractors = 0;
    let mut scored_responses = 0;

    for (_, answers) in &responses {
        if let Some(score) = nps_score_from_answers(answers, question_id) {
            scored_responses += 1;
            if score >= 9 {
                promoters += 1;
            } else if score >= 7 {
                passives += 1;
            } else {
                detractors += 1;
            }
        }
    }

    let nps_score = if scored_responses > 0 {
        ((promoters as f64 / scored_responses as f64)
            - (detractors as f64 / scored_responses as f64))
            * 100.0
    } else {
        0.0
    };

    Ok(NpsReport {
        survey_id,
        question_id: question_id.map(str::to_string),
        total_responses: responses.len() as i64,
        scored_responses,
        promoters,
        passives,
        detractors,
        nps_score,
    })
}

pub async fn get_sentiment_report(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
    question_id: Option<&str>,
) -> AppResult<SentimentReport> {
    ensure_survey_exists(db, project_id, survey_id).await?;
    let responses = response_answers(db, project_id, survey_id).await?;

    let mut positive = 0;
    let mut neutral = 0;
    let mut negative = 0;
    let mut total_score = 0;
    let mut examples = Vec::new();

    for (response_id, answers) in &responses {
        for text in text_answers(answers, question_id) {
            let score = sentiment_score(&text);
            total_score += score;
            let sentiment = if score > 0 {
                positive += 1;
                "positive"
            } else if score < 0 {
                negative += 1;
                "negative"
            } else {
                neutral += 1;
                "neutral"
            };
            if examples.len() < 10 {
                examples.push(SentimentExample {
                    response_id: *response_id,
                    sentiment: sentiment.to_string(),
                    score,
                    text: text.chars().take(500).collect(),
                });
            }
        }
    }

    let total_text_responses = positive + neutral + negative;
    let sentiment_score = if total_text_responses > 0 {
        total_score as f64 / total_text_responses as f64
    } else {
        0.0
    };

    Ok(SentimentReport {
        survey_id,
        question_id: question_id.map(str::to_string),
        total_text_responses,
        positive,
        neutral,
        negative,
        sentiment_score,
        examples,
    })
}

async fn ensure_survey_exists(db: &PgPool, project_id: Uuid, survey_id: Uuid) -> AppResult<()> {
    let exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM surveys WHERE id = $1 AND project_id = $2")
            .bind(survey_id)
            .bind(project_id)
            .fetch_optional(db)
            .await?;
    if exists.is_none() {
        return Err(AppError::NotFound("Survey not found".to_string()));
    }
    Ok(())
}

async fn response_answers(
    db: &PgPool,
    project_id: Uuid,
    survey_id: Uuid,
) -> AppResult<Vec<(Uuid, serde_json::Value)>> {
    let responses = sqlx::query_as(
        "SELECT id, answers FROM survey_responses \
         WHERE survey_id = $1 AND project_id = $2 AND completed = true \
         ORDER BY created_at DESC",
    )
    .bind(survey_id)
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(responses)
}

fn nps_score_from_answers(answers: &serde_json::Value, question_id: Option<&str>) -> Option<i64> {
    answer_items(answers)
        .into_iter()
        .filter(|item| question_matches(item, question_id))
        .find_map(numeric_answer)
        .filter(|score| (0..=10).contains(score))
}

fn text_answers(answers: &serde_json::Value, question_id: Option<&str>) -> Vec<String> {
    answer_items(answers)
        .into_iter()
        .filter(|item| question_matches(item, question_id))
        .filter_map(text_answer)
        .filter(|text| text.split_whitespace().count() >= 2)
        .collect()
}

fn answer_items(answers: &serde_json::Value) -> Vec<&serde_json::Value> {
    answers
        .as_array()
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn question_matches(item: &serde_json::Value, question_id: Option<&str>) -> bool {
    let Some(question_id) = question_id else {
        return true;
    };
    ["question_id", "question", "id"]
        .iter()
        .filter_map(|field| item.get(field).and_then(|value| value.as_str()))
        .any(|value| value == question_id)
}

fn numeric_answer(item: &serde_json::Value) -> Option<i64> {
    ["value", "answer", "score"]
        .iter()
        .find_map(|field| item.get(field))
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|value| value.round() as i64))
                .or_else(|| value.as_str().and_then(|value| value.parse::<i64>().ok()))
        })
}

fn text_answer(item: &serde_json::Value) -> Option<String> {
    ["value", "answer", "text"]
        .iter()
        .find_map(|field| item.get(field))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn sentiment_score(text: &str) -> i32 {
    let lower = text.to_ascii_lowercase();
    let positive = [
        "love",
        "great",
        "excellent",
        "easy",
        "fast",
        "helpful",
        "useful",
        "clear",
        "happy",
        "awesome",
        "better",
        "best",
    ];
    let negative = [
        "hate",
        "bad",
        "poor",
        "slow",
        "confusing",
        "broken",
        "hard",
        "difficult",
        "bug",
        "issue",
        "frustrating",
        "worse",
        "worst",
    ];

    let pos = positive
        .iter()
        .filter(|word| {
            lower
                .split(|c: char| !c.is_alphanumeric())
                .any(|part| part == **word)
        })
        .count() as i32;
    let neg = negative
        .iter()
        .filter(|word| {
            lower
                .split(|c: char| !c.is_alphanumeric())
                .any(|part| part == **word)
        })
        .count() as i32;
    pos - neg
}

pub async fn list_guides(db: &PgPool, project_id: Uuid) -> AppResult<Vec<InAppGuide>> {
    let guides = sqlx::query_as(&format!(
        "SELECT {GUIDE_COLUMNS} FROM in_app_guides \
         WHERE project_id = $1 ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(guides)
}

pub async fn get_guide(db: &PgPool, project_id: Uuid, guide_id: Uuid) -> AppResult<InAppGuide> {
    let guide = sqlx::query_as(&format!(
        "SELECT {GUIDE_COLUMNS} FROM in_app_guides \
         WHERE id = $1 AND project_id = $2"
    ))
    .bind(guide_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Guide not found".to_string()))?;
    Ok(guide)
}

pub async fn create_guide(
    db: &PgPool,
    project_id: Uuid,
    input: GuideInput,
) -> AppResult<InAppGuide> {
    let input = validate_guide_input(input)?;
    let guide = sqlx::query_as(&format!(
        "INSERT INTO in_app_guides \
         (project_id, name, guide_type, steps, targeting, appearance, priority) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING {GUIDE_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.guide_type)
    .bind(&input.steps)
    .bind(&input.targeting)
    .bind(&input.appearance)
    .bind(input.priority)
    .fetch_one(db)
    .await?;
    Ok(guide)
}

pub async fn update_guide(
    db: &PgPool,
    project_id: Uuid,
    guide_id: Uuid,
    input: GuideInput,
) -> AppResult<InAppGuide> {
    let input = validate_guide_input(input)?;
    let guide = sqlx::query_as(&format!(
        "UPDATE in_app_guides SET \
           name = $3, guide_type = $4, steps = $5, targeting = $6, appearance = $7, \
           priority = $8, updated_at = NOW() \
         WHERE id = $1 AND project_id = $2 \
         RETURNING {GUIDE_COLUMNS}"
    ))
    .bind(guide_id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.guide_type)
    .bind(&input.steps)
    .bind(&input.targeting)
    .bind(&input.appearance)
    .bind(input.priority)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Guide not found".to_string()))?;
    Ok(guide)
}

pub async fn delete_guide(db: &PgPool, project_id: Uuid, guide_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM in_app_guides WHERE id = $1 AND project_id = $2")
        .bind(guide_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Guide not found".to_string()));
    }
    Ok(())
}

pub async fn update_guide_status(
    db: &PgPool,
    project_id: Uuid,
    guide_id: Uuid,
    status: &str,
) -> AppResult<InAppGuide> {
    let status = validate_guide_status(status)?;
    let now = Utc::now();
    let query = format!(
        "UPDATE in_app_guides SET status = $1, \
         started_at = CASE WHEN $1 = 'active' THEN COALESCE(started_at, $4::timestamptz) ELSE started_at END, \
         ended_at = CASE WHEN $1 = 'active' THEN NULL::timestamptz WHEN $1 = 'archived' THEN $4::timestamptz ELSE ended_at END, \
         updated_at = NOW() \
         WHERE id = $2 AND project_id = $3 \
         RETURNING {GUIDE_COLUMNS}"
    );
    let guide = sqlx::query_as(&query)
        .bind(status)
        .bind(guide_id)
        .bind(project_id)
        .bind(now)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Guide not found".to_string()))?;
    Ok(guide)
}

pub async fn get_active_guides(db: &PgPool, project_id: Uuid) -> AppResult<Vec<InAppGuide>> {
    let guides = sqlx::query_as(&format!(
        "SELECT {GUIDE_COLUMNS} FROM in_app_guides \
         WHERE project_id = $1 AND status = 'active' \
           AND (started_at IS NULL OR started_at <= NOW()) \
           AND (ended_at IS NULL OR ended_at > NOW()) \
         ORDER BY priority DESC, created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(guides)
}

pub async fn record_guide_event(
    db: &PgPool,
    project_id: Uuid,
    guide_id: Uuid,
    input: GuideEventInput,
) -> AppResult<GuideEvent> {
    let input = validate_guide_event_input(input)?;
    let event = sqlx::query_as(&format!(
        "INSERT INTO guide_events \
         (project_id, guide_id, visitor_id, event_type, step_id, metadata, path) \
         SELECT $1, g.id, $3, $4, $5, $6, $7 \
         FROM in_app_guides g \
         WHERE g.id = $2 AND g.project_id = $1 \
         RETURNING {GUIDE_EVENT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(guide_id)
    .bind(&input.visitor_id)
    .bind(&input.event_type)
    .bind(&input.step_id)
    .bind(&input.metadata)
    .bind(&input.path)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Guide not found".to_string()))?;
    Ok(event)
}

pub async fn list_guide_events(
    db: &PgPool,
    project_id: Uuid,
    guide_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<GuideEvent>> {
    let events = sqlx::query_as(&format!(
        "SELECT {GUIDE_EVENT_COLUMNS} FROM guide_events \
         WHERE project_id = $1 AND guide_id = $2 \
         ORDER BY created_at DESC LIMIT $3 OFFSET $4"
    ))
    .bind(project_id)
    .bind(guide_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(events)
}

pub async fn get_guide_stats(
    db: &PgPool,
    project_id: Uuid,
    guide_id: Uuid,
) -> AppResult<GuideStats> {
    get_guide(db, project_id, guide_id).await?;
    let (shown, started, completed, dismissed, converted): (i64, i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT \
               COUNT(*) FILTER (WHERE event_type = 'shown')::bigint, \
               COUNT(*) FILTER (WHERE event_type = 'started')::bigint, \
               COUNT(*) FILTER (WHERE event_type = 'completed')::bigint, \
               COUNT(*) FILTER (WHERE event_type = 'dismissed')::bigint, \
               COUNT(*) FILTER (WHERE event_type = 'converted')::bigint \
             FROM guide_events \
             WHERE project_id = $1 AND guide_id = $2",
        )
        .bind(project_id)
        .bind(guide_id)
        .fetch_one(db)
        .await?;

    let denominator = started.max(shown).max(1) as f64;
    Ok(GuideStats {
        guide_id,
        shown,
        started,
        completed,
        dismissed,
        converted,
        completion_rate: (completed as f64 / denominator) * 100.0,
        dismissal_rate: (dismissed as f64 / denominator) * 100.0,
    })
}

fn validate_guide_input(mut input: GuideInput) -> AppResult<GuideInput> {
    input.name = input.name.trim().to_string();
    input.guide_type = input.guide_type.trim().to_ascii_lowercase();
    if input.name.is_empty() {
        return Err(AppError::BadRequest("Guide name is required".to_string()));
    }
    if !matches!(
        input.guide_type.as_str(),
        "tour" | "tooltip" | "onboarding" | "announcement" | "checklist"
    ) {
        return Err(AppError::BadRequest(format!(
            "Unsupported guide type: {}",
            input.guide_type
        )));
    }
    if !input.steps.is_array() {
        return Err(AppError::BadRequest(
            "Guide steps must be an array".to_string(),
        ));
    }
    if !input.targeting.is_object() {
        return Err(AppError::BadRequest(
            "Guide targeting must be an object".to_string(),
        ));
    }
    if !input.appearance.is_object() {
        return Err(AppError::BadRequest(
            "Guide appearance must be an object".to_string(),
        ));
    }
    Ok(input)
}

fn validate_guide_event_input(mut input: GuideEventInput) -> AppResult<GuideEventInput> {
    input.visitor_id = input.visitor_id.trim().to_string();
    input.event_type = input.event_type.trim().to_ascii_lowercase();
    input.step_id = input
        .step_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    input.path = input
        .path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if input.visitor_id.is_empty() {
        return Err(AppError::BadRequest("visitor_id is required".to_string()));
    }
    validate_guide_event_type(&input.event_type)?;
    if !input.metadata.is_object() {
        return Err(AppError::BadRequest(
            "Guide event metadata must be an object".to_string(),
        ));
    }
    Ok(input)
}

fn validate_guide_status(status: &str) -> AppResult<&'static str> {
    match status.trim() {
        "draft" => Ok("draft"),
        "active" => Ok("active"),
        "paused" => Ok("paused"),
        "archived" => Ok("archived"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported guide status: {other}"
        ))),
    }
}

fn validate_guide_event_type(event_type: &str) -> AppResult<&'static str> {
    match event_type {
        "shown" => Ok("shown"),
        "started" => Ok("started"),
        "step_viewed" => Ok("step_viewed"),
        "completed" => Ok("completed"),
        "dismissed" => Ok("dismissed"),
        "converted" => Ok("converted"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported guide event type: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        nps_score_from_answers, sentiment_score, text_answers, validate_guide_event_input,
        validate_guide_input, validate_guide_status, GuideEventInput, GuideInput,
    };
    use serde_json::json;

    #[test]
    fn extracts_nps_scores_from_flexible_answers() {
        let answers = json!([
            { "question_id": "nps", "value": 9 },
            { "question_id": "comment", "value": "Great product" }
        ]);
        assert_eq!(nps_score_from_answers(&answers, Some("nps")), Some(9));
        assert_eq!(nps_score_from_answers(&answers, Some("missing")), None);
    }

    #[test]
    fn extracts_text_answers_by_question() {
        let answers = json!([
            { "question_id": "nps", "value": 9 },
            { "question_id": "comment", "value": "Great product" }
        ]);
        assert_eq!(
            text_answers(&answers, Some("comment")),
            vec!["Great product"]
        );
    }

    #[test]
    fn scores_basic_sentiment() {
        assert!(sentiment_score("Love the fast dashboard") > 0);
        assert!(sentiment_score("The flow is confusing and slow") < 0);
        assert_eq!(sentiment_score("It is okay"), 0);
    }

    #[test]
    fn validates_guide_inputs() {
        let guide = validate_guide_input(GuideInput {
            name: "  Onboarding  ".to_string(),
            guide_type: "Tour".to_string(),
            steps: json!([{ "id": "welcome" }]),
            targeting: json!({ "paths": ["/app"] }),
            appearance: json!({}),
            priority: 10,
        })
        .unwrap();
        assert_eq!(guide.name, "Onboarding");
        assert_eq!(guide.guide_type, "tour");

        let invalid = validate_guide_input(GuideInput {
            name: "Broken".to_string(),
            guide_type: "modal".to_string(),
            steps: json!([]),
            targeting: json!({}),
            appearance: json!({}),
            priority: 0,
        });
        assert!(invalid.is_err());
    }

    #[test]
    fn validates_guide_status_and_events() {
        assert!(validate_guide_status("active").is_ok());
        assert!(validate_guide_status("done").is_err());

        let event = validate_guide_event_input(GuideEventInput {
            visitor_id: " visitor ".to_string(),
            event_type: "STEP_VIEWED".to_string(),
            step_id: Some(" intro ".to_string()),
            metadata: json!({}),
            path: Some(" /app ".to_string()),
        })
        .unwrap();
        assert_eq!(event.visitor_id, "visitor");
        assert_eq!(event.event_type, "step_viewed");
        assert_eq!(event.step_id.as_deref(), Some("intro"));
        assert_eq!(event.path.as_deref(), Some("/app"));
    }
}
