use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::{error_tracking, query as qsvc};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AiQueryRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub question: String,
    pub intent: String,
    pub answer: String,
    pub result: serde_json::Value,
    pub insights: serde_json::Value,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiInsight {
    pub title: String,
    pub summary: String,
    pub severity: String,
    pub metric: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AiQueryResponse {
    pub id: Uuid,
    pub question: String,
    pub intent: String,
    pub answer: String,
    pub result: serde_json::Value,
    pub insights: Vec<AiInsight>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmTrace {
    pub id: Uuid,
    pub project_id: Uuid,
    pub trace_key: String,
    pub name: Option<String>,
    pub user_id: Option<String>,
    pub visitor_id: Option<String>,
    pub session_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmTraceInput {
    pub trace_key: String,
    pub name: Option<String>,
    pub user_id: Option<String>,
    pub visitor_id: Option<String>,
    pub session_id: Option<Uuid>,
    #[serde(default = "default_object")]
    pub metadata: serde_json::Value,
    #[serde(default = "default_trace_status")]
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmGeneration {
    pub id: Uuid,
    pub project_id: Uuid,
    pub trace_id: Option<Uuid>,
    pub trace_key: Option<String>,
    pub provider: String,
    pub model: String,
    pub operation: String,
    pub prompt: serde_json::Value,
    pub completion: serde_json::Value,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
    pub latency_ms: Option<i64>,
    pub cost_usd: f64,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmGenerationInput {
    pub trace_id: Option<Uuid>,
    pub trace_key: Option<String>,
    pub provider: String,
    pub model: String,
    #[serde(default = "default_operation")]
    pub operation: String,
    #[serde(default = "default_object")]
    pub prompt: serde_json::Value,
    #[serde(default = "default_object")]
    pub completion: serde_json::Value,
    #[serde(default)]
    pub input_tokens: i32,
    #[serde(default)]
    pub output_tokens: i32,
    pub total_tokens: Option<i32>,
    pub latency_ms: Option<i64>,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default = "default_generation_status")]
    pub status: String,
    pub error_message: Option<String>,
    #[serde(default = "default_object")]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmEvaluation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub generation_id: Option<Uuid>,
    pub trace_id: Option<Uuid>,
    pub trace_key: Option<String>,
    pub evaluator: String,
    pub metric: String,
    pub score: Option<f64>,
    pub label: Option<String>,
    pub passed: Option<bool>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmEvaluationInput {
    pub generation_id: Option<Uuid>,
    pub trace_id: Option<Uuid>,
    pub trace_key: Option<String>,
    pub evaluator: String,
    pub metric: String,
    pub score: Option<f64>,
    pub label: Option<String>,
    pub passed: Option<bool>,
    #[serde(default = "default_object")]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct LlmStats {
    pub total_generations: i64,
    pub error_generations: i64,
    pub total_tokens: i64,
    pub avg_latency_ms: f64,
    pub total_cost_usd: f64,
    pub evaluation_count: i64,
    pub evaluation_pass_rate: f64,
}

const LLM_TRACE_COLUMNS: &str = "id, project_id, trace_key, name, user_id, visitor_id, \
    session_id, metadata, status, started_at, ended_at, duration_ms, created_at, updated_at";
const LLM_GENERATION_COLUMNS: &str = "id, project_id, trace_id, trace_key, provider, model, \
    operation, prompt, completion, input_tokens, output_tokens, total_tokens, latency_ms, \
    cost_usd, status, error_message, metadata, created_at";
const LLM_EVALUATION_COLUMNS: &str = "id, project_id, generation_id, trace_id, trace_key, \
    evaluator, metric, score, label, passed, metadata, created_at";

fn default_object() -> serde_json::Value {
    json!({})
}

fn default_trace_status() -> String {
    "success".to_string()
}

fn default_generation_status() -> String {
    "success".to_string()
}

fn default_operation() -> String {
    "chat_completion".to_string()
}

pub async fn answer_query(
    db: &PgPool,
    project_id: Uuid,
    question: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> AppResult<AiQueryResponse> {
    if question.trim().is_empty() {
        return Err(AppError::BadRequest("question cannot be empty".to_string()));
    }
    if start >= end {
        return Err(AppError::BadRequest(
            "start_at must be before end_at".to_string(),
        ));
    }

    let intent = detect_intent(question);
    let today = Utc::now().date_naive();
    let bounded_limit = limit.clamp(1, 100);

    let (answer, result, insights) = match intent.as_str() {
        "top_pages" => {
            let pages =
                qsvc::fetch_pages(db, project_id, start, end, today, bounded_limit, 0).await?;
            let answer = match pages
                .first()
                .and_then(|p| p.get("path"))
                .and_then(|v| v.as_str())
            {
                Some(path) => format!("The top page was {path}."),
                None => "No pageview data matched that time range.".to_string(),
            };
            let insights = page_insights(&pages);
            (answer, json!({ "data": pages }), insights)
        }
        "referrers" => {
            let rows =
                qsvc::fetch_referrers(db, project_id, start, end, today, bounded_limit, 0).await?;
            let answer = match rows
                .first()
                .and_then(|r| r.get("referrer_domain"))
                .and_then(|v| v.as_str())
            {
                Some(source) => format!("The top referrer was {source}."),
                None => "No referrer data matched that time range.".to_string(),
            };
            (answer, json!({ "data": rows }), Vec::new())
        }
        "events" => {
            let rows =
                qsvc::fetch_events(db, project_id, start, end, today, bounded_limit, 0).await?;
            let answer = match rows
                .first()
                .and_then(|r| r.get("event_name"))
                .and_then(|v| v.as_str())
            {
                Some(event) => format!("The most frequent event was {event}."),
                None => "No custom events matched that time range.".to_string(),
            };
            (answer, json!({ "data": rows }), Vec::new())
        }
        "devices" => {
            let rows =
                qsvc::fetch_devices(db, project_id, start, end, today, bounded_limit, 0).await?;
            (
                "Device and browser breakdown is returned in the result data.".to_string(),
                json!({ "data": rows }),
                Vec::new(),
            )
        }
        "geo" => {
            let rows = qsvc::fetch_geo(db, project_id, start, end, today, bounded_limit, 0).await?;
            let answer = match rows
                .first()
                .and_then(|r| r.get("country"))
                .and_then(|v| v.as_str())
            {
                Some(country) => format!("The top country was {country}."),
                None => "No geography data matched that time range.".to_string(),
            };
            (answer, json!({ "data": rows }), Vec::new())
        }
        "traffic_trend" => {
            let data = qsvc::fetch_timeseries(db, project_id, start, end, today).await?;
            let answer =
                "Traffic trend data is returned as daily pageviews, visitors, and sessions."
                    .to_string();
            (answer, json!({ "data": data }), Vec::new())
        }
        "errors" => {
            let stats = error_tracking::get_error_stats(db, project_id, start, end).await?;
            let answer = format!(
                "There were {} errors across {} unique error groups affecting {} visitors.",
                stats.total_errors, stats.unique_errors, stats.affected_visitors
            );
            let insight = if stats.total_errors > 0 {
                vec![AiInsight {
                    title: "Errors detected".to_string(),
                    summary: "Review top error groups and releases before evaluating funnel or conversion changes."
                        .to_string(),
                    severity: "warning".to_string(),
                    metric: "errors".to_string(),
                    evidence: json!({
                        "total_errors": stats.total_errors,
                        "unique_errors": stats.unique_errors,
                        "affected_visitors": stats.affected_visitors,
                    }),
                }]
            } else {
                Vec::new()
            };
            (
                answer,
                serde_json::to_value(stats).unwrap_or_else(|_| json!({})),
                insight,
            )
        }
        _ => overview(db, project_id, start, end).await?,
    };

    let run = insert_query_run(
        db,
        project_id,
        question,
        &intent,
        &answer,
        result.clone(),
        serde_json::to_value(&insights).unwrap_or_else(|_| json!([])),
        start,
        end,
    )
    .await?;

    Ok(AiQueryResponse {
        id: run.id,
        question: question.to_string(),
        intent,
        answer,
        result,
        insights,
        start_at: start,
        end_at: end,
    })
}

pub async fn generate_insights(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<Vec<AiInsight>> {
    let (_, _, insights) = overview(db, project_id, start, end).await?;
    Ok(insights)
}

pub async fn list_query_runs(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AiQueryRun>> {
    let runs = sqlx::query_as(
        "SELECT id, project_id, question, intent, answer, result, insights, start_at, end_at, created_at \
         FROM ai_query_runs WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(runs)
}

pub async fn record_llm_trace(
    db: &PgPool,
    project_id: Uuid,
    input: LlmTraceInput,
) -> AppResult<LlmTrace> {
    let input = validate_llm_trace_input(input)?;
    let trace = sqlx::query_as(&format!(
        "INSERT INTO llm_traces \
         (project_id, trace_key, name, user_id, visitor_id, session_id, metadata, status, started_at, ended_at, duration_ms) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, COALESCE($9, NOW()), $10, $11) \
         ON CONFLICT (project_id, trace_key) DO UPDATE SET \
           name = EXCLUDED.name, user_id = EXCLUDED.user_id, visitor_id = EXCLUDED.visitor_id, \
           session_id = EXCLUDED.session_id, metadata = EXCLUDED.metadata, status = EXCLUDED.status, \
           started_at = EXCLUDED.started_at, ended_at = EXCLUDED.ended_at, duration_ms = EXCLUDED.duration_ms, \
           updated_at = NOW() \
         RETURNING {LLM_TRACE_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&input.trace_key)
    .bind(&input.name)
    .bind(&input.user_id)
    .bind(&input.visitor_id)
    .bind(input.session_id)
    .bind(&input.metadata)
    .bind(&input.status)
    .bind(input.started_at)
    .bind(input.ended_at)
    .bind(input.duration_ms)
    .fetch_one(db)
    .await?;
    Ok(trace)
}

pub async fn list_llm_traces(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<LlmTrace>> {
    let traces = sqlx::query_as(&format!(
        "SELECT {LLM_TRACE_COLUMNS} FROM llm_traces \
         WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(traces)
}

pub async fn get_llm_trace(db: &PgPool, project_id: Uuid, trace_id: Uuid) -> AppResult<LlmTrace> {
    let trace = sqlx::query_as(&format!(
        "SELECT {LLM_TRACE_COLUMNS} FROM llm_traces \
         WHERE id = $1 AND project_id = $2"
    ))
    .bind(trace_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("LLM trace not found".to_string()))?;
    Ok(trace)
}

pub async fn record_llm_generation(
    db: &PgPool,
    project_id: Uuid,
    input: LlmGenerationInput,
) -> AppResult<LlmGeneration> {
    let input = validate_llm_generation_input(input)?;
    let trace_id =
        resolve_trace_id(db, project_id, input.trace_id, input.trace_key.as_deref()).await?;
    let total_tokens = input
        .total_tokens
        .unwrap_or_else(|| input.input_tokens + input.output_tokens);

    let generation = sqlx::query_as(&format!(
        "INSERT INTO llm_generations \
         (project_id, trace_id, trace_key, provider, model, operation, prompt, completion, \
          input_tokens, output_tokens, total_tokens, latency_ms, cost_usd, status, error_message, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) \
         RETURNING {LLM_GENERATION_COLUMNS}"
    ))
    .bind(project_id)
    .bind(trace_id)
    .bind(&input.trace_key)
    .bind(&input.provider)
    .bind(&input.model)
    .bind(&input.operation)
    .bind(&input.prompt)
    .bind(&input.completion)
    .bind(input.input_tokens)
    .bind(input.output_tokens)
    .bind(total_tokens)
    .bind(input.latency_ms)
    .bind(input.cost_usd)
    .bind(&input.status)
    .bind(&input.error_message)
    .bind(&input.metadata)
    .fetch_one(db)
    .await?;
    Ok(generation)
}

pub async fn list_llm_generations(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<LlmGeneration>> {
    let generations = sqlx::query_as(&format!(
        "SELECT {LLM_GENERATION_COLUMNS} FROM llm_generations \
         WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(generations)
}

pub async fn record_llm_evaluation(
    db: &PgPool,
    project_id: Uuid,
    input: LlmEvaluationInput,
) -> AppResult<LlmEvaluation> {
    let input = validate_llm_evaluation_input(input)?;
    let (trace_id, trace_key) = resolve_evaluation_links(
        db,
        project_id,
        input.generation_id,
        input.trace_id,
        input.trace_key,
    )
    .await?;

    let evaluation = sqlx::query_as(&format!(
        "INSERT INTO llm_evaluations \
         (project_id, generation_id, trace_id, trace_key, evaluator, metric, score, label, passed, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING {LLM_EVALUATION_COLUMNS}"
    ))
    .bind(project_id)
    .bind(input.generation_id)
    .bind(trace_id)
    .bind(&trace_key)
    .bind(&input.evaluator)
    .bind(&input.metric)
    .bind(input.score)
    .bind(&input.label)
    .bind(input.passed)
    .bind(&input.metadata)
    .fetch_one(db)
    .await?;
    Ok(evaluation)
}

pub async fn list_llm_evaluations(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<LlmEvaluation>> {
    let evaluations = sqlx::query_as(&format!(
        "SELECT {LLM_EVALUATION_COLUMNS} FROM llm_evaluations \
         WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    ))
    .bind(project_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(evaluations)
}

pub async fn get_llm_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<LlmStats> {
    if start >= end {
        return Err(AppError::BadRequest(
            "start_at must be before end_at".to_string(),
        ));
    }

    let (total_generations, error_generations, total_tokens, avg_latency_ms, total_cost_usd): (
        i64,
        i64,
        i64,
        f64,
        f64,
    ) = sqlx::query_as(
        "SELECT \
           COUNT(*)::bigint, \
           COUNT(*) FILTER (WHERE status = 'error')::bigint, \
           COALESCE(SUM(total_tokens), 0)::bigint, \
           COALESCE(AVG(latency_ms), 0)::double precision, \
           COALESCE(SUM(cost_usd), 0)::double precision \
         FROM llm_generations \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    let (evaluation_count, evaluation_pass_rate): (i64, f64) = sqlx::query_as(
        "SELECT \
           COUNT(*)::bigint, \
           COALESCE(AVG(CASE WHEN passed THEN 1.0 ELSE 0.0 END) FILTER (WHERE passed IS NOT NULL), 0)::double precision * 100.0 \
         FROM llm_evaluations \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    Ok(LlmStats {
        total_generations,
        error_generations,
        total_tokens,
        avg_latency_ms,
        total_cost_usd,
        evaluation_count,
        evaluation_pass_rate,
    })
}

async fn overview(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<(String, serde_json::Value, Vec<AiInsight>)> {
    let today = Utc::now().date_naive();
    let duration = end - start;
    let previous_start = start - duration;
    let previous_end = start;
    let current = qsvc::fetch_stats(db, project_id, start, end, today).await?;
    let previous = qsvc::fetch_stats(db, project_id, previous_start, previous_end, today).await?;
    let events_current = qsvc::fetch_events_count(db, project_id, start, end, today).await?;
    let pages = qsvc::fetch_pages(db, project_id, start, end, today, 5, 0).await?;

    let bounce_rate = percent(current.3, current.2);
    let avg_duration = if current.2 > 0 {
        (current.4 as f64) / (current.2 as f64) / 1000.0
    } else {
        0.0
    };
    let pageview_change = percent_change(current.0, previous.0);
    let visitor_change = percent_change(current.1, previous.1);

    let mut insights = Vec::new();
    if pageview_change.abs() >= 20.0 && previous.0 > 0 {
        insights.push(AiInsight {
            title: if pageview_change > 0.0 {
                "Traffic increased".to_string()
            } else {
                "Traffic declined".to_string()
            },
            summary: format!("Pageviews changed by {:.1}% versus the previous comparable period.", pageview_change),
            severity: if pageview_change > 0.0 { "info" } else { "warning" }.to_string(),
            metric: "pageviews".to_string(),
            evidence: json!({ "current": current.0, "previous": previous.0, "change_percent": pageview_change }),
        });
    }
    if bounce_rate >= 60.0 && current.2 >= 20 {
        insights.push(AiInsight {
            title: "Bounce rate is elevated".to_string(),
            summary: format!(
                "Bounce rate is {:.1}% across {} sessions.",
                bounce_rate, current.2
            ),
            severity: "warning".to_string(),
            metric: "bounce_rate".to_string(),
            evidence: json!({ "bounce_rate": bounce_rate, "sessions": current.2 }),
        });
    }
    insights.extend(page_insights(&pages));

    let answer = format!(
        "Pulse saw {} pageviews, {} visitors, {} sessions, and {} custom events. Pageviews changed by {:.1}% and visitors changed by {:.1}% versus the previous comparable period.",
        current.0, current.1, current.2, events_current, pageview_change, visitor_change
    );
    let result = json!({
        "stats": {
            "pageviews": current.0,
            "visitors": current.1,
            "sessions": current.2,
            "events": events_current,
            "bounce_rate": bounce_rate,
            "avg_duration": avg_duration,
            "pageview_change_percent": pageview_change,
            "visitor_change_percent": visitor_change,
        },
        "top_pages": pages,
    });

    Ok((answer, result, insights))
}

async fn insert_query_run(
    db: &PgPool,
    project_id: Uuid,
    question: &str,
    intent: &str,
    answer: &str,
    result: serde_json::Value,
    insights: serde_json::Value,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<AiQueryRun> {
    let run = sqlx::query_as(
        "INSERT INTO ai_query_runs \
         (project_id, question, intent, answer, result, insights, start_at, end_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id, project_id, question, intent, answer, result, insights, start_at, end_at, created_at",
    )
    .bind(project_id)
    .bind(question.trim())
    .bind(intent)
    .bind(answer)
    .bind(result)
    .bind(insights)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;
    Ok(run)
}

async fn resolve_trace_id(
    db: &PgPool,
    project_id: Uuid,
    trace_id: Option<Uuid>,
    trace_key: Option<&str>,
) -> AppResult<Option<Uuid>> {
    if let Some(trace_id) = trace_id {
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM llm_traces WHERE id = $1 AND project_id = $2")
                .bind(trace_id)
                .bind(project_id)
                .fetch_optional(db)
                .await?;
        if exists.is_none() {
            return Err(AppError::NotFound("LLM trace not found".to_string()));
        }
        return Ok(Some(trace_id));
    }

    let Some(trace_key) = trace_key else {
        return Ok(None);
    };
    let trace: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM llm_traces WHERE project_id = $1 AND trace_key = $2")
            .bind(project_id)
            .bind(trace_key)
            .fetch_optional(db)
            .await?;
    Ok(trace.map(|row| row.0))
}

async fn resolve_evaluation_links(
    db: &PgPool,
    project_id: Uuid,
    generation_id: Option<Uuid>,
    input_trace_id: Option<Uuid>,
    input_trace_key: Option<String>,
) -> AppResult<(Option<Uuid>, Option<String>)> {
    let mut trace_id = input_trace_id;
    let mut trace_key = input_trace_key;

    if let Some(generation_id) = generation_id {
        let generation: Option<(Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT trace_id, trace_key FROM llm_generations WHERE id = $1 AND project_id = $2",
        )
        .bind(generation_id)
        .bind(project_id)
        .fetch_optional(db)
        .await?;
        let generation =
            generation.ok_or_else(|| AppError::NotFound("LLM generation not found".to_string()))?;
        trace_id = trace_id.or(generation.0);
        trace_key = trace_key.or(generation.1);
    }

    if let Some(id) = trace_id {
        resolve_trace_id(db, project_id, Some(id), None).await?;
    }

    Ok((trace_id, trace_key))
}

fn validate_llm_trace_input(mut input: LlmTraceInput) -> AppResult<LlmTraceInput> {
    input.trace_key = input.trace_key.trim().to_string();
    input.name = normalized(input.name);
    input.user_id = normalized(input.user_id);
    input.visitor_id = normalized(input.visitor_id);
    input.status = input.status.trim().to_ascii_lowercase();

    if input.trace_key.is_empty() {
        return Err(AppError::BadRequest("trace_key is required".to_string()));
    }
    if !matches!(
        input.status.as_str(),
        "started" | "success" | "error" | "cancelled"
    ) {
        return Err(AppError::BadRequest(format!(
            "Unsupported LLM trace status: {}",
            input.status
        )));
    }
    validate_metadata_object(&input.metadata, "trace metadata")?;
    if matches!(input.duration_ms, Some(value) if value < 0) {
        return Err(AppError::BadRequest(
            "duration_ms must be non-negative".to_string(),
        ));
    }
    if let (Some(started_at), Some(ended_at)) = (input.started_at, input.ended_at) {
        if started_at > ended_at {
            return Err(AppError::BadRequest(
                "started_at must be before ended_at".to_string(),
            ));
        }
    }
    Ok(input)
}

fn validate_llm_generation_input(mut input: LlmGenerationInput) -> AppResult<LlmGenerationInput> {
    input.trace_key = normalized(input.trace_key);
    input.provider = input.provider.trim().to_string();
    input.model = input.model.trim().to_string();
    input.operation = input.operation.trim().to_string();
    input.status = input.status.trim().to_ascii_lowercase();
    input.error_message = normalized(input.error_message);

    if input.provider.is_empty() {
        return Err(AppError::BadRequest("provider is required".to_string()));
    }
    if input.model.is_empty() {
        return Err(AppError::BadRequest("model is required".to_string()));
    }
    if input.operation.is_empty() {
        return Err(AppError::BadRequest("operation is required".to_string()));
    }
    if !matches!(input.status.as_str(), "success" | "error" | "cancelled") {
        return Err(AppError::BadRequest(format!(
            "Unsupported LLM generation status: {}",
            input.status
        )));
    }
    if input.input_tokens < 0
        || input.output_tokens < 0
        || input.total_tokens.is_some_and(|tokens| tokens < 0)
    {
        return Err(AppError::BadRequest(
            "Token counts must be non-negative".to_string(),
        ));
    }
    if matches!(input.latency_ms, Some(value) if value < 0) {
        return Err(AppError::BadRequest(
            "latency_ms must be non-negative".to_string(),
        ));
    }
    if !input.cost_usd.is_finite() || input.cost_usd < 0.0 {
        return Err(AppError::BadRequest(
            "cost_usd must be a non-negative number".to_string(),
        ));
    }
    validate_metadata_object(&input.metadata, "generation metadata")?;
    Ok(input)
}

fn validate_llm_evaluation_input(mut input: LlmEvaluationInput) -> AppResult<LlmEvaluationInput> {
    input.trace_key = normalized(input.trace_key);
    input.evaluator = input.evaluator.trim().to_string();
    input.metric = input.metric.trim().to_string();
    input.label = normalized(input.label);

    if input.evaluator.is_empty() {
        return Err(AppError::BadRequest("evaluator is required".to_string()));
    }
    if input.metric.is_empty() {
        return Err(AppError::BadRequest("metric is required".to_string()));
    }
    if input.score.is_some_and(|score| !score.is_finite()) {
        return Err(AppError::BadRequest(
            "score must be a finite number".to_string(),
        ));
    }
    validate_metadata_object(&input.metadata, "evaluation metadata")?;
    Ok(input)
}

fn validate_metadata_object(value: &serde_json::Value, label: &str) -> AppResult<()> {
    if !value.is_object() {
        return Err(AppError::BadRequest(format!("{label} must be an object")));
    }
    Ok(())
}

fn normalized(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn detect_intent(question: &str) -> String {
    let q = question.to_ascii_lowercase();
    if any(&q, &["error", "bug", "exception", "crash"]) {
        "errors"
    } else if any(&q, &["page", "url", "landing"]) {
        "top_pages"
    } else if any(&q, &["referrer", "source", "utm", "campaign", "channel"]) {
        "referrers"
    } else if any(&q, &["event", "conversion", "click", "signup", "purchase"]) {
        "events"
    } else if any(&q, &["device", "browser", "os", "mobile", "desktop"]) {
        "devices"
    } else if any(&q, &["country", "geo", "region", "city", "location"]) {
        "geo"
    } else if any(&q, &["trend", "over time", "timeseries", "daily"]) {
        "traffic_trend"
    } else {
        "overview"
    }
    .to_string()
}

fn page_insights(pages: &[serde_json::Value]) -> Vec<AiInsight> {
    let total_views: i64 = pages
        .iter()
        .filter_map(|p| p.get("views").and_then(|v| v.as_i64()))
        .sum();
    if total_views <= 0 {
        return Vec::new();
    }
    let Some(top) = pages.first() else {
        return Vec::new();
    };
    let top_views = top.get("views").and_then(|v| v.as_i64()).unwrap_or(0);
    let share = (top_views as f64) / (total_views as f64) * 100.0;
    if share < 50.0 {
        return Vec::new();
    }
    vec![AiInsight {
        title: "Traffic is concentrated".to_string(),
        summary: format!(
            "The top page accounts for {:.1}% of views among the returned top pages.",
            share
        ),
        severity: "info".to_string(),
        metric: "pages".to_string(),
        evidence: json!({ "top_page": top, "share_percent": share }),
    }]
}

fn percent(numerator: i64, denominator: i64) -> f64 {
    if denominator > 0 {
        (numerator as f64) / (denominator as f64) * 100.0
    } else {
        0.0
    }
}

fn percent_change(current: i64, previous: i64) -> f64 {
    if previous > 0 {
        ((current - previous) as f64) / (previous as f64) * 100.0
    } else if current > 0 {
        100.0
    } else {
        0.0
    }
}

fn any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{
        detect_intent, page_insights, validate_llm_evaluation_input, validate_llm_generation_input,
        validate_llm_trace_input, LlmEvaluationInput, LlmGenerationInput, LlmTraceInput,
    };
    use serde_json::json;

    #[test]
    fn detects_common_query_intents() {
        assert_eq!(detect_intent("What are my top pages?"), "top_pages");
        assert_eq!(detect_intent("Show traffic by browser"), "devices");
        assert_eq!(detect_intent("Any crashes this week?"), "errors");
        assert_eq!(
            detect_intent("How did traffic trend daily?"),
            "traffic_trend"
        );
    }

    #[test]
    fn flags_concentrated_page_traffic() {
        let insights = page_insights(&[
            json!({"path": "/", "views": 90}),
            json!({"path": "/pricing", "views": 10}),
        ]);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].metric, "pages");
    }

    #[test]
    fn validates_llm_trace_inputs() {
        let trace = validate_llm_trace_input(LlmTraceInput {
            trace_key: " trace-1 ".to_string(),
            name: Some(" Checkout agent ".to_string()),
            user_id: Some(" ".to_string()),
            visitor_id: None,
            session_id: None,
            metadata: json!({"workflow": "checkout"}),
            status: "Started".to_string(),
            started_at: None,
            ended_at: None,
            duration_ms: Some(15),
        })
        .expect("valid trace");

        assert_eq!(trace.trace_key, "trace-1");
        assert_eq!(trace.name.as_deref(), Some("Checkout agent"));
        assert!(trace.user_id.is_none());
        assert_eq!(trace.status, "started");

        assert!(validate_llm_trace_input(LlmTraceInput {
            trace_key: " ".to_string(),
            name: None,
            user_id: None,
            visitor_id: None,
            session_id: None,
            metadata: json!({}),
            status: "success".to_string(),
            started_at: None,
            ended_at: None,
            duration_ms: None,
        })
        .is_err());
    }

    #[test]
    fn validates_llm_generation_inputs() {
        let generation = validate_llm_generation_input(LlmGenerationInput {
            trace_id: None,
            trace_key: Some("trace-1".to_string()),
            provider: " openai ".to_string(),
            model: " gpt-4.1 ".to_string(),
            operation: " chat ".to_string(),
            prompt: json!({"messages": []}),
            completion: json!({"text": "ok"}),
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: None,
            latency_ms: Some(1200),
            cost_usd: 0.01,
            status: "Success".to_string(),
            error_message: Some(" ".to_string()),
            metadata: json!({}),
        })
        .expect("valid generation");

        assert_eq!(generation.provider, "openai");
        assert_eq!(generation.model, "gpt-4.1");
        assert_eq!(generation.status, "success");
        assert!(generation.error_message.is_none());

        assert!(validate_llm_generation_input(LlmGenerationInput {
            trace_id: None,
            trace_key: None,
            provider: "".to_string(),
            model: "gpt".to_string(),
            operation: "chat".to_string(),
            prompt: json!({}),
            completion: json!({}),
            input_tokens: -1,
            output_tokens: 0,
            total_tokens: None,
            latency_ms: None,
            cost_usd: 0.0,
            status: "success".to_string(),
            error_message: None,
            metadata: json!({}),
        })
        .is_err());
    }

    #[test]
    fn validates_llm_evaluation_inputs() {
        let evaluation = validate_llm_evaluation_input(LlmEvaluationInput {
            generation_id: None,
            trace_id: None,
            trace_key: Some(" trace-1 ".to_string()),
            evaluator: " exact_match ".to_string(),
            metric: " correctness ".to_string(),
            score: Some(0.9),
            label: Some(" pass ".to_string()),
            passed: Some(true),
            metadata: json!({}),
        })
        .expect("valid evaluation");

        assert_eq!(evaluation.trace_key.as_deref(), Some("trace-1"));
        assert_eq!(evaluation.evaluator, "exact_match");
        assert_eq!(evaluation.metric, "correctness");

        assert!(validate_llm_evaluation_input(LlmEvaluationInput {
            generation_id: None,
            trace_id: None,
            trace_key: None,
            evaluator: "judge".to_string(),
            metric: "".to_string(),
            score: None,
            label: None,
            passed: None,
            metadata: json!([]),
        })
        .is_err());
    }
}
