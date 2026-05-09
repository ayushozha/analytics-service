use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct ChannelStat {
    pub channel: String,
    pub visitors: i64,
    pub sessions: i64,
    pub pageviews: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttributionRow {
    pub model: String,
    pub channel: String,
    pub source: Option<String>,
    pub campaign: Option<String>,
    pub conversions: f64,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevenueByCurrency {
    pub currency: String,
    pub orders: i64,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductRevenue {
    pub product_id: String,
    pub product_name: String,
    pub orders: i64,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EcommerceReport {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub orders: i64,
    pub revenue: f64,
    pub average_order_value: f64,
    pub currency_breakdown: Vec<RevenueByCurrency>,
    pub top_products: Vec<ProductRevenue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiReferrerStat {
    pub referrer_domain: String,
    pub provider: String,
    pub visitors: i64,
    pub sessions: i64,
    pub pageviews: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketingImport {
    pub id: Uuid,
    pub project_id: Uuid,
    pub provider: String,
    pub name: String,
    pub row_count: i32,
    pub imported_by: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketingImportRow {
    pub id: i64,
    pub import_id: Uuid,
    pub project_id: Uuid,
    pub row_number: i32,
    pub row_date: Option<NaiveDate>,
    pub dimensions: serde_json::Value,
    pub metrics: serde_json::Value,
    pub raw_row: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarketingImportInput {
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub rows: Vec<MarketingImportRowInput>,
    pub imported_by: Option<String>,
    #[serde(default = "empty_object")]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarketingImportRowInput {
    pub date: Option<NaiveDate>,
    #[serde(default = "empty_object")]
    pub dimensions: serde_json::Value,
    #[serde(default = "empty_object")]
    pub metrics: serde_json::Value,
    #[serde(default = "empty_object")]
    pub raw_row: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketingImportSummary {
    pub provider: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub rows: i64,
    pub impressions: f64,
    pub clicks: f64,
    pub cost: f64,
    pub conversions: f64,
    pub revenue: f64,
    pub sessions: f64,
    pub users: f64,
}

#[derive(Debug, Default)]
struct ChannelAccumulator {
    visitors: i64,
    sessions: i64,
    pageviews: i64,
}

#[derive(Debug, Default)]
struct AttributionAccumulator {
    conversions: f64,
    revenue: f64,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

pub async fn get_channel_groups(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> AppResult<Vec<ChannelStat>> {
    validate_range(start_at, end_at)?;

    let rows: Vec<(
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i64,
        i64,
    )> = sqlx::query_as(
        "SELECT NULLIF(utm_source, '') AS source, NULLIF(utm_medium, '') AS medium, \
                NULLIF(referrer_domain, '') AS referrer_domain, \
                COUNT(DISTINCT visitor_id)::bigint AS visitors, \
                COUNT(DISTINCT session_id)::bigint AS sessions, \
                COUNT(*)::bigint AS pageviews \
         FROM pageviews \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
         GROUP BY NULLIF(utm_source, ''), NULLIF(utm_medium, ''), NULLIF(referrer_domain, '')",
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_all(db)
    .await?;

    let mut channels: HashMap<String, ChannelAccumulator> = HashMap::new();
    let mut total_pageviews = 0;
    for (source, medium, referrer, visitors, sessions, pageviews) in rows {
        let channel = classify_channel(source.as_deref(), medium.as_deref(), referrer.as_deref());
        let entry = channels.entry(channel).or_default();
        entry.visitors += visitors;
        entry.sessions += sessions;
        entry.pageviews += pageviews;
        total_pageviews += pageviews;
    }

    let mut stats: Vec<ChannelStat> = channels
        .into_iter()
        .map(|(channel, acc)| ChannelStat {
            channel,
            visitors: acc.visitors,
            sessions: acc.sessions,
            pageviews: acc.pageviews,
            percentage: percent(acc.pageviews as f64, total_pageviews as f64),
        })
        .collect();
    stats.sort_by(|a, b| b.pageviews.cmp(&a.pageviews));
    Ok(stats)
}

pub async fn get_attribution(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    model: &str,
) -> AppResult<Vec<AttributionRow>> {
    validate_range(start_at, end_at)?;
    let model = validate_attribution_model(model)?;
    let rows = if model == "linear" {
        fetch_linear_attribution(db, project_id, start_at, end_at).await?
    } else {
        fetch_touch_attribution(db, project_id, start_at, end_at, model).await?
    };

    let mut grouped: HashMap<(String, Option<String>, Option<String>), AttributionAccumulator> =
        HashMap::new();
    for (source, medium, campaign, referrer, conversions, revenue) in rows {
        let channel = classify_channel(source.as_deref(), medium.as_deref(), referrer.as_deref());
        let key = (channel, source, campaign);
        let entry = grouped.entry(key).or_default();
        entry.conversions += conversions;
        entry.revenue += revenue;
    }

    let mut result: Vec<AttributionRow> = grouped
        .into_iter()
        .map(|((channel, source, campaign), acc)| AttributionRow {
            model: model.to_string(),
            channel,
            source,
            campaign,
            conversions: acc.conversions,
            revenue: acc.revenue,
        })
        .collect();
    result.sort_by(|a, b| {
        b.revenue
            .partial_cmp(&a.revenue)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(result)
}

pub async fn get_ecommerce_report(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> AppResult<EcommerceReport> {
    validate_range(start_at, end_at)?;

    let totals: (i64, f64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, COALESCE(SUM(revenue_amount), 0)::float8 \
         FROM events \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
           AND revenue_amount IS NOT NULL",
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_one(db)
    .await?;

    let currency_rows: Vec<(String, i64, f64)> = sqlx::query_as(
        "SELECT COALESCE(revenue_currency, 'USD') AS currency, COUNT(*)::bigint, \
                COALESCE(SUM(revenue_amount), 0)::float8 \
         FROM events \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
           AND revenue_amount IS NOT NULL \
         GROUP BY COALESCE(revenue_currency, 'USD') \
         ORDER BY 3 DESC",
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_all(db)
    .await?;

    let product_rows: Vec<(String, String, i64, f64)> = sqlx::query_as(
        "SELECT COALESCE(event_data->>'product_id', 'unknown') AS product_id, \
                COALESCE(event_data->>'product_name', event_data->>'name', event_name, 'Unknown') AS product_name, \
                COUNT(*)::bigint AS orders, COALESCE(SUM(revenue_amount), 0)::float8 AS revenue \
         FROM events \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
           AND revenue_amount IS NOT NULL \
         GROUP BY 1, 2 \
         ORDER BY revenue DESC \
         LIMIT 25",
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_all(db)
    .await?;

    Ok(EcommerceReport {
        start_at,
        end_at,
        orders: totals.0,
        revenue: totals.1,
        average_order_value: if totals.0 > 0 {
            totals.1 / totals.0 as f64
        } else {
            0.0
        },
        currency_breakdown: currency_rows
            .into_iter()
            .map(|(currency, orders, revenue)| RevenueByCurrency {
                currency,
                orders,
                revenue,
            })
            .collect(),
        top_products: product_rows
            .into_iter()
            .map(
                |(product_id, product_name, orders, revenue)| ProductRevenue {
                    product_id,
                    product_name,
                    orders,
                    revenue,
                },
            )
            .collect(),
    })
}

pub async fn get_ai_referrers(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> AppResult<Vec<AiReferrerStat>> {
    validate_range(start_at, end_at)?;

    let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT referrer_domain, COUNT(DISTINCT visitor_id)::bigint, \
                COUNT(DISTINCT session_id)::bigint, COUNT(*)::bigint \
         FROM pageviews \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
           AND referrer_domain IS NOT NULL AND referrer_domain <> '' \
         GROUP BY referrer_domain \
         ORDER BY 4 DESC",
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(referrer_domain, visitors, sessions, pageviews)| {
            ai_provider(&referrer_domain).map(|provider| AiReferrerStat {
                referrer_domain,
                provider: provider.to_string(),
                visitors,
                sessions,
                pageviews,
            })
        })
        .collect())
}

pub async fn list_imports(
    db: &PgPool,
    project_id: Uuid,
    provider: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<MarketingImport>> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);
    let imports = if let Some(provider) = provider {
        let provider = validate_import_provider(provider)?;
        sqlx::query_as(
            "SELECT id, project_id, provider, name, row_count, imported_by, metadata, created_at, updated_at \
             FROM marketing_imports \
             WHERE project_id = $1 AND provider = $2 \
             ORDER BY created_at DESC LIMIT $3 OFFSET $4",
        )
        .bind(project_id)
        .bind(provider)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, project_id, provider, name, row_count, imported_by, metadata, created_at, updated_at \
             FROM marketing_imports \
             WHERE project_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?
    };
    Ok(imports)
}

pub async fn create_import(
    db: &PgPool,
    project_id: Uuid,
    input: MarketingImportInput,
) -> AppResult<MarketingImport> {
    let input = validate_import_input(input)?;
    let mut tx = db.begin().await?;
    let import: MarketingImport = sqlx::query_as(
        "INSERT INTO marketing_imports (project_id, provider, name, row_count, imported_by, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, project_id, provider, name, row_count, imported_by, metadata, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&input.provider)
    .bind(&input.name)
    .bind(input.rows.len() as i32)
    .bind(&input.imported_by)
    .bind(&input.metadata)
    .fetch_one(&mut *tx)
    .await?;

    for (idx, row) in input.rows.iter().enumerate() {
        sqlx::query(
            "INSERT INTO marketing_import_rows \
             (import_id, project_id, row_number, row_date, dimensions, metrics, raw_row) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(import.id)
        .bind(project_id)
        .bind((idx + 1) as i32)
        .bind(row.date)
        .bind(&row.dimensions)
        .bind(&row.metrics)
        .bind(&row.raw_row)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(import)
}

pub async fn get_import_rows(
    db: &PgPool,
    project_id: Uuid,
    import_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<MarketingImportRow>> {
    let exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM marketing_imports WHERE id = $1 AND project_id = $2")
            .bind(import_id)
            .bind(project_id)
            .fetch_optional(db)
            .await?;
    if exists.is_none() {
        return Err(AppError::NotFound("Marketing import not found".to_string()));
    }
    let rows = sqlx::query_as(
        "SELECT id, import_id, project_id, row_number, row_date, dimensions, metrics, raw_row, created_at \
         FROM marketing_import_rows \
         WHERE project_id = $1 AND import_id = $2 \
         ORDER BY row_number ASC LIMIT $3 OFFSET $4",
    )
    .bind(project_id)
    .bind(import_id)
    .bind(limit.clamp(1, 1000))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn delete_import(db: &PgPool, project_id: Uuid, import_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM marketing_imports WHERE id = $1 AND project_id = $2")
        .bind(import_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Marketing import not found".to_string()));
    }
    Ok(())
}

pub async fn get_import_summary(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    provider: Option<&str>,
) -> AppResult<MarketingImportSummary> {
    validate_range(start_at, end_at)?;
    let start_date = start_at.date_naive();
    let end_date = end_at.date_naive();
    let provider = provider.map(validate_import_provider).transpose()?;
    let row = if let Some(provider) = provider {
        sqlx::query_as::<_, ImportSummaryRow>(
            "SELECT COUNT(*)::bigint AS rows, \
                    COALESCE(SUM((r.metrics->>'impressions')::float8), 0)::float8 AS impressions, \
                    COALESCE(SUM((r.metrics->>'clicks')::float8), 0)::float8 AS clicks, \
                    COALESCE(SUM((r.metrics->>'cost')::float8), 0)::float8 AS cost, \
                    COALESCE(SUM((r.metrics->>'conversions')::float8), 0)::float8 AS conversions, \
                    COALESCE(SUM((r.metrics->>'revenue')::float8), 0)::float8 AS revenue, \
                    COALESCE(SUM((r.metrics->>'sessions')::float8), 0)::float8 AS sessions, \
                    COALESCE(SUM((r.metrics->>'users')::float8), 0)::float8 AS users \
             FROM marketing_import_rows r \
             JOIN marketing_imports i ON i.id = r.import_id \
             WHERE r.project_id = $1 AND i.provider = $2 \
               AND r.row_date >= $3 AND r.row_date <= $4",
        )
        .bind(project_id)
        .bind(provider)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(db)
        .await?
    } else {
        sqlx::query_as::<_, ImportSummaryRow>(
            "SELECT COUNT(*)::bigint AS rows, \
                    COALESCE(SUM((r.metrics->>'impressions')::float8), 0)::float8 AS impressions, \
                    COALESCE(SUM((r.metrics->>'clicks')::float8), 0)::float8 AS clicks, \
                    COALESCE(SUM((r.metrics->>'cost')::float8), 0)::float8 AS cost, \
                    COALESCE(SUM((r.metrics->>'conversions')::float8), 0)::float8 AS conversions, \
                    COALESCE(SUM((r.metrics->>'revenue')::float8), 0)::float8 AS revenue, \
                    COALESCE(SUM((r.metrics->>'sessions')::float8), 0)::float8 AS sessions, \
                    COALESCE(SUM((r.metrics->>'users')::float8), 0)::float8 AS users \
             FROM marketing_import_rows r \
             WHERE r.project_id = $1 AND r.row_date >= $2 AND r.row_date <= $3",
        )
        .bind(project_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(db)
        .await?
    };

    Ok(MarketingImportSummary {
        provider: provider.map(str::to_string),
        start_date,
        end_date,
        rows: row.rows,
        impressions: row.impressions,
        clicks: row.clicks,
        cost: row.cost,
        conversions: row.conversions,
        revenue: row.revenue,
        sessions: row.sessions,
        users: row.users,
    })
}

async fn fetch_touch_attribution(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    model: &str,
) -> AppResult<Vec<AttributionSourceRow>> {
    let direction = if model == "first_touch" {
        "ASC"
    } else {
        "DESC"
    };
    let sql = format!(
        "WITH attributed AS ( \
           SELECT e.id, COALESCE(e.revenue_amount, 0)::float8 AS revenue, \
                  NULLIF(t.utm_source, '') AS source, NULLIF(t.utm_medium, '') AS medium, \
                  NULLIF(t.utm_campaign, '') AS campaign, NULLIF(t.referrer_domain, '') AS referrer_domain \
           FROM events e \
           LEFT JOIN LATERAL ( \
             SELECT utm_source, utm_medium, utm_campaign, referrer_domain \
             FROM pageviews p \
             WHERE p.project_id = e.project_id \
               AND p.session_id = e.session_id \
               AND p.created_at <= e.created_at \
             ORDER BY p.created_at {direction} \
             LIMIT 1 \
           ) t ON true \
           WHERE e.project_id = $1 AND e.created_at >= $2 AND e.created_at <= $3 \
             AND e.revenue_amount IS NOT NULL \
         ) \
         SELECT source, medium, campaign, referrer_domain, COUNT(*)::float8, COALESCE(SUM(revenue), 0)::float8 \
         FROM attributed \
         GROUP BY source, medium, campaign, referrer_domain"
    );
    let rows = sqlx::query_as(&sql)
        .bind(project_id)
        .bind(start_at)
        .bind(end_at)
        .fetch_all(db)
        .await?;
    Ok(rows)
}

async fn fetch_linear_attribution(
    db: &PgPool,
    project_id: Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> AppResult<Vec<AttributionSourceRow>> {
    let rows = sqlx::query_as(
        "WITH conversion_events AS ( \
           SELECT id, project_id, session_id, created_at, COALESCE(revenue_amount, 0)::float8 AS revenue \
           FROM events \
           WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
             AND revenue_amount IS NOT NULL \
         ), touches AS ( \
           SELECT e.id, e.revenue, NULLIF(p.utm_source, '') AS source, \
                  NULLIF(p.utm_medium, '') AS medium, NULLIF(p.utm_campaign, '') AS campaign, \
                  NULLIF(p.referrer_domain, '') AS referrer_domain \
           FROM conversion_events e \
           LEFT JOIN LATERAL ( \
             SELECT DISTINCT utm_source, utm_medium, utm_campaign, referrer_domain \
             FROM pageviews p \
             WHERE p.project_id = e.project_id \
               AND p.session_id = e.session_id \
               AND p.created_at <= e.created_at \
           ) p ON true \
         ), weighted AS ( \
           SELECT *, COUNT(*) OVER (PARTITION BY id) AS touch_count FROM touches \
         ) \
         SELECT source, medium, campaign, referrer_domain, \
                COALESCE(SUM(1.0 / NULLIF(touch_count, 0)), 0)::float8 AS conversions, \
                COALESCE(SUM(revenue / NULLIF(touch_count, 0)), 0)::float8 AS revenue \
         FROM weighted \
         GROUP BY source, medium, campaign, referrer_domain",
    )
    .bind(project_id)
    .bind(start_at)
    .bind(end_at)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

type AttributionSourceRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    f64,
    f64,
);

#[derive(Debug, FromRow)]
struct ImportSummaryRow {
    rows: i64,
    impressions: f64,
    clicks: f64,
    cost: f64,
    conversions: f64,
    revenue: f64,
    sessions: f64,
    users: f64,
}

fn validate_range(start_at: DateTime<Utc>, end_at: DateTime<Utc>) -> AppResult<()> {
    if start_at >= end_at {
        return Err(AppError::BadRequest(
            "start_at must be before end_at".to_string(),
        ));
    }
    Ok(())
}

fn validate_import_input(mut input: MarketingImportInput) -> AppResult<MarketingImportInput> {
    input.provider = validate_import_provider(&input.provider)?.to_string();
    input.name = input.name.trim().to_string();
    input.imported_by = input
        .imported_by
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if input.name.is_empty() {
        return Err(AppError::BadRequest("Import name is required".to_string()));
    }
    if input.rows.is_empty() {
        return Err(AppError::BadRequest(
            "Marketing imports require at least one row".to_string(),
        ));
    }
    if input.rows.len() > 10_000 {
        return Err(AppError::BadRequest(
            "Marketing imports support at most 10000 rows per request".to_string(),
        ));
    }
    if !input.metadata.is_object() {
        return Err(AppError::BadRequest(
            "metadata must be an object".to_string(),
        ));
    }
    for row in &input.rows {
        validate_import_row(row)?;
    }
    Ok(input)
}

fn validate_import_row(row: &MarketingImportRowInput) -> AppResult<()> {
    if !row.dimensions.is_object() {
        return Err(AppError::BadRequest(
            "Marketing import row dimensions must be an object".to_string(),
        ));
    }
    let metrics = row.metrics.as_object().ok_or_else(|| {
        AppError::BadRequest("Marketing import row metrics must be an object".to_string())
    })?;
    if !row.raw_row.is_object() {
        return Err(AppError::BadRequest(
            "Marketing import row raw_row must be an object".to_string(),
        ));
    }
    for (key, value) in metrics {
        if !metric_value_is_numeric(value) {
            return Err(AppError::BadRequest(format!(
                "Marketing import metric '{key}' must be a finite number"
            )));
        }
    }
    Ok(())
}

fn metric_value_is_numeric(value: &serde_json::Value) -> bool {
    value.as_f64().is_some_and(f64::is_finite)
}

fn validate_import_provider(provider: &str) -> AppResult<&'static str> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "google_analytics" | "ga4" => Ok("google_analytics"),
        "google_ads" => Ok("google_ads"),
        "search_console" | "google_search_console" => Ok("search_console"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported marketing import provider: {other}"
        ))),
    }
}

fn validate_attribution_model(model: &str) -> AppResult<&'static str> {
    match model.trim() {
        "first_touch" => Ok("first_touch"),
        "last_touch" => Ok("last_touch"),
        "linear" => Ok("linear"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported attribution model: {other}"
        ))),
    }
}

fn classify_channel(source: Option<&str>, medium: Option<&str>, referrer: Option<&str>) -> String {
    let source = source.unwrap_or("").to_ascii_lowercase();
    let medium = medium.unwrap_or("").to_ascii_lowercase();
    let referrer = referrer.unwrap_or("").to_ascii_lowercase();
    let combined = format!("{source} {medium} {referrer}");

    if combined.trim().is_empty() || source == "direct" || referrer == "direct" {
        "Direct".to_string()
    } else if ai_provider(&combined).is_some() {
        "AI Referrals".to_string()
    } else if medium == "email" || source.contains("mail") {
        "Email".to_string()
    } else if medium.contains("affiliate") {
        "Affiliate".to_string()
    } else if medium.contains("display") || medium.contains("banner") {
        "Display".to_string()
    } else if is_social(&combined) && is_paid(&medium) {
        "Paid Social".to_string()
    } else if is_social(&combined) {
        "Organic Social".to_string()
    } else if is_search(&combined) && is_paid(&medium) {
        "Paid Search".to_string()
    } else if is_search(&combined) {
        "Organic Search".to_string()
    } else if is_paid(&medium) {
        "Paid Other".to_string()
    } else if !referrer.is_empty() {
        "Referral".to_string()
    } else {
        "Other".to_string()
    }
}

fn ai_provider(value: &str) -> Option<&'static str> {
    let value = value.to_ascii_lowercase();
    if value.contains("chatgpt") || value.contains("openai") {
        Some("OpenAI")
    } else if value.contains("perplexity") {
        Some("Perplexity")
    } else if value.contains("claude") || value.contains("anthropic") {
        Some("Anthropic")
    } else if value.contains("gemini") || value.contains("bard.google") {
        Some("Google Gemini")
    } else if value.contains("copilot") || value.contains("bing.com/chat") {
        Some("Microsoft Copilot")
    } else {
        None
    }
}

fn is_paid(medium: &str) -> bool {
    matches!(
        medium,
        "cpc" | "ppc" | "paid" | "paid_search" | "paid-social" | "paid_social" | "ads"
    )
}

fn is_search(value: &str) -> bool {
    ["google", "bing", "yahoo", "duckduckgo", "baidu", "yandex"]
        .iter()
        .any(|needle| value.contains(needle))
}

fn is_social(value: &str) -> bool {
    [
        "facebook",
        "instagram",
        "linkedin",
        "twitter",
        "x.com",
        "t.co",
        "reddit",
        "youtube",
        "tiktok",
        "threads",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn percent(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        numerator / denominator * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ai_provider, classify_channel, validate_attribution_model, validate_import_input,
        validate_import_provider, MarketingImportInput, MarketingImportRowInput,
    };
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn classifies_marketing_channels() {
        assert_eq!(
            classify_channel(Some("google"), Some("cpc"), None),
            "Paid Search"
        );
        assert_eq!(
            classify_channel(None, None, Some("chatgpt.com")),
            "AI Referrals"
        );
        assert_eq!(classify_channel(None, None, None), "Direct");
    }

    #[test]
    fn detects_ai_referrer_providers() {
        assert_eq!(ai_provider("perplexity.ai"), Some("Perplexity"));
        assert_eq!(ai_provider("example.com"), None);
    }

    #[test]
    fn validates_attribution_models() {
        assert!(validate_attribution_model("first_touch").is_ok());
        assert!(validate_attribution_model("last_touch").is_ok());
        assert!(validate_attribution_model("linear").is_ok());
        assert!(validate_attribution_model("time_decay").is_err());
    }

    #[test]
    fn validates_marketing_import_providers_and_rows() {
        assert_eq!(
            validate_import_provider("ga4").expect("ga4 alias"),
            "google_analytics"
        );
        assert_eq!(
            validate_import_provider("google_search_console").expect("gsc alias"),
            "search_console"
        );

        let input = validate_import_input(MarketingImportInput {
            provider: "GOOGLE_ADS".to_string(),
            name: " Weekly ads export ".to_string(),
            rows: vec![MarketingImportRowInput {
                date: Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
                dimensions: json!({"campaign": "brand", "source": "google"}),
                metrics: json!({"impressions": 1000, "clicks": 42, "cost": 123.45}),
                raw_row: json!({"Campaign": "brand"}),
            }],
            imported_by: Some(" analyst@example.com ".to_string()),
            metadata: json!({"source": "csv"}),
        })
        .expect("valid import");

        assert_eq!(input.provider, "google_ads");
        assert_eq!(input.name, "Weekly ads export");
        assert_eq!(input.imported_by.as_deref(), Some("analyst@example.com"));
    }

    #[test]
    fn rejects_bad_marketing_import_shapes() {
        assert!(validate_import_input(MarketingImportInput {
            provider: "unknown".to_string(),
            name: "Import".to_string(),
            rows: vec![MarketingImportRowInput {
                date: None,
                dimensions: json!({}),
                metrics: json!({"clicks": 1}),
                raw_row: json!({}),
            }],
            imported_by: None,
            metadata: json!({}),
        })
        .is_err());

        assert!(validate_import_input(MarketingImportInput {
            provider: "search_console".to_string(),
            name: "Bad import".to_string(),
            rows: vec![MarketingImportRowInput {
                date: None,
                dimensions: json!({}),
                metrics: json!({"clicks": "not-a-number"}),
                raw_row: json!({}),
            }],
            imported_by: None,
            metadata: json!({}),
        })
        .is_err());
    }
}
