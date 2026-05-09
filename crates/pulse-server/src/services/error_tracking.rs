use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct ErrorGroup {
    pub fingerprint: String,
    pub message: String,
    pub count: i64,
    pub affected_visitors: i64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub last_path: Option<String>,
    pub last_browser: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub source_map_configured: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorInstance {
    pub id: i64,
    pub visitor_id: String,
    pub session_id: Uuid,
    pub message: String,
    pub stack: Option<String>,
    pub filename: Option<String>,
    pub lineno: Option<i32>,
    pub colno: Option<i32>,
    pub path: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub fingerprint: Option<String>,
    pub matched_source_map: Option<MatchedSourceMap>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ErrorTimeseriesPoint {
    pub date: NaiveDate,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ErrorStats {
    pub total_errors: i64,
    pub unique_errors: i64,
    pub affected_visitors: i64,
    pub releases: Vec<ReleaseErrorCount>,
}

#[derive(Debug, Serialize)]
pub struct ReleaseErrorCount {
    pub release: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AppRelease {
    pub id: Uuid,
    pub project_id: Uuid,
    pub version: String,
    pub environment: String,
    pub commit_sha: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SourceMapArtifact {
    pub id: Uuid,
    pub project_id: Uuid,
    pub release_id: Option<Uuid>,
    pub release_version: String,
    pub environment: String,
    pub minified_url: String,
    pub source_map_url: Option<String>,
    pub artifacts: serde_json::Value,
    pub uploaded_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchedSourceMap {
    pub id: Uuid,
    pub release_version: String,
    pub environment: String,
    pub minified_url: String,
    pub source_map_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LogEntry {
    pub id: i64,
    pub project_id: Uuid,
    pub visitor_id: Option<String>,
    pub session_id: Option<Uuid>,
    pub level: String,
    pub message: String,
    pub body: serde_json::Value,
    pub path: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LogStats {
    pub total: i64,
    pub levels: Vec<LogLevelCount>,
    pub releases: Vec<ReleaseLogCount>,
}

#[derive(Debug, Serialize)]
pub struct LogLevelCount {
    pub level: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ReleaseLogCount {
    pub release: String,
    pub count: i64,
}

#[derive(Debug, Deserialize)]
pub struct LogFilters {
    pub level: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, FromRow)]
struct ErrorInstanceRow {
    id: i64,
    visitor_id: String,
    session_id: Uuid,
    message: String,
    stack: Option<String>,
    filename: Option<String>,
    lineno: Option<i32>,
    colno: Option<i32>,
    path: Option<String>,
    browser: Option<String>,
    os: Option<String>,
    release: Option<String>,
    environment: Option<String>,
    fingerprint: Option<String>,
    source_map_id: Option<Uuid>,
    source_map_release_version: Option<String>,
    source_map_environment: Option<String>,
    source_map_minified_url: Option<String>,
    source_map_url: Option<String>,
    created_at: DateTime<Utc>,
}

pub fn error_fingerprint(
    message: &str,
    filename: Option<&str>,
    lineno: Option<i32>,
    stack: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(message.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(filename.unwrap_or("").trim().as_bytes());
    hasher.update(b"|");
    if let Some(lineno) = lineno {
        hasher.update(lineno.to_string().as_bytes());
    }
    hasher.update(b"|");
    if let Some(first_stack_line) =
        stack.and_then(|s| s.lines().find(|line| !line.trim().is_empty()))
    {
        hasher.update(first_stack_line.trim().as_bytes());
    }
    let digest = hasher.finalize();
    hex::encode(&digest[..16])
}

pub fn normalize_log_level(level: &str) -> AppResult<String> {
    let normalized = level.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "trace" | "debug" | "info" | "warn" | "warning" | "error" | "fatal" => {
            Ok(if normalized == "warning" {
                "warn".to_string()
            } else {
                normalized
            })
        }
        _ => Err(AppError::BadRequest(format!("Invalid log level: {level}"))),
    }
}

/// Get error groups: errors grouped by message with count and first/last seen.
pub async fn get_error_groups(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<ErrorGroup>> {
    let rows: Vec<(
        String,
        String,
        i64,
        i64,
        DateTime<Utc>,
        DateTime<Utc>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
    )> = sqlx::query_as(
        "SELECT COALESCE(e.fingerprint, e.message) AS fingerprint, \
             (array_agg(e.message ORDER BY e.created_at DESC))[1] AS message, \
             COUNT(*)::bigint, COUNT(DISTINCT e.visitor_id)::bigint, \
             MIN(e.created_at), MAX(e.created_at), \
             (array_agg(e.path ORDER BY e.created_at DESC))[1], \
             (array_agg(e.browser ORDER BY e.created_at DESC))[1], \
             e.release, e.environment, \
             EXISTS ( \
               SELECT 1 FROM source_maps sm \
               WHERE sm.project_id = e.project_id \
                 AND sm.release_version = e.release \
                 AND sm.environment = COALESCE(e.environment, 'production') \
             ) AS source_map_configured \
             FROM js_errors e WHERE e.project_id = $1 \
             AND e.created_at >= $2 AND e.created_at <= $3 \
             GROUP BY COALESCE(e.fingerprint, e.message), e.release, e.environment, e.project_id \
             ORDER BY 3 DESC LIMIT $4 OFFSET $5",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| ErrorGroup {
            fingerprint: r.0,
            message: r.1,
            count: r.2,
            affected_visitors: r.3,
            first_seen: r.4,
            last_seen: r.5,
            last_path: r.6,
            last_browser: r.7,
            release: r.8,
            environment: r.9,
            source_map_configured: r.10,
        })
        .collect();

    Ok(results)
}

/// Get individual error instances for a specific error message.
pub async fn get_error_detail(
    db: &PgPool,
    project_id: Uuid,
    message: Option<&str>,
    fingerprint: Option<&str>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    limit: i64,
) -> AppResult<Vec<ErrorInstance>> {
    let rows: Vec<ErrorInstanceRow> = sqlx::query_as(
        "SELECT e.id, e.visitor_id, e.session_id, e.message, e.stack, e.filename, e.lineno, e.colno, \
             e.path, e.browser, e.os, e.release, e.environment, e.fingerprint, \
             sm.id AS source_map_id, \
             sm.release_version AS source_map_release_version, \
             sm.environment AS source_map_environment, \
             sm.minified_url AS source_map_minified_url, \
             sm.source_map_url AS source_map_url, \
             e.created_at \
             FROM js_errors e \
             LEFT JOIN LATERAL ( \
               SELECT id, release_version, environment, minified_url, source_map_url \
               FROM source_maps sm \
               WHERE sm.project_id = e.project_id \
                 AND sm.release_version = e.release \
                 AND sm.environment = COALESCE(e.environment, 'production') \
                 AND e.filename IS NOT NULL \
                 AND (e.filename = sm.minified_url OR e.filename LIKE '%' || sm.minified_url) \
               ORDER BY sm.created_at DESC \
               LIMIT 1 \
             ) sm ON true \
             WHERE e.project_id = $1 \
             AND ($2::text IS NULL OR e.message = $2) \
             AND ($3::text IS NULL OR e.fingerprint = $3) \
             AND e.created_at >= $4 AND e.created_at <= $5 \
             ORDER BY e.created_at DESC LIMIT $6",
    )
    .bind(project_id)
    .bind(message)
    .bind(fingerprint)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| ErrorInstance {
            id: r.id,
            visitor_id: r.visitor_id,
            session_id: r.session_id,
            message: r.message,
            stack: r.stack,
            filename: r.filename,
            lineno: r.lineno,
            colno: r.colno,
            path: r.path,
            browser: r.browser,
            os: r.os,
            release: r.release,
            environment: r.environment,
            fingerprint: r.fingerprint,
            matched_source_map: r.source_map_id.map(|id| MatchedSourceMap {
                id,
                release_version: r.source_map_release_version.unwrap_or_default(),
                environment: r.source_map_environment.unwrap_or_default(),
                minified_url: r.source_map_minified_url.unwrap_or_default(),
                source_map_url: r.source_map_url,
            }),
            created_at: r.created_at,
        })
        .collect();

    Ok(results)
}

/// Get daily error count timeseries.
pub async fn get_error_timeseries(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<Vec<ErrorTimeseriesPoint>> {
    let rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        "SELECT created_at::date as day, COUNT(*)::bigint \
         FROM js_errors WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         GROUP BY day ORDER BY day",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?;

    let results = rows
        .into_iter()
        .map(|(date, count)| ErrorTimeseriesPoint { date, count })
        .collect();

    Ok(results)
}

/// Get aggregate error statistics.
pub async fn get_error_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<ErrorStats> {
    let row: (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, COUNT(DISTINCT COALESCE(fingerprint, message))::bigint, \
         COUNT(DISTINCT visitor_id)::bigint \
         FROM js_errors WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    let releases: Vec<ReleaseErrorCount> = sqlx::query_as::<_, (String, i64)>(
        "SELECT COALESCE(release, 'unknown') AS release, COUNT(*)::bigint \
         FROM js_errors WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         GROUP BY COALESCE(release, 'unknown') ORDER BY 2 DESC LIMIT 10",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|(release, count)| ReleaseErrorCount { release, count })
    .collect();

    Ok(ErrorStats {
        total_errors: row.0,
        unique_errors: row.1,
        affected_visitors: row.2,
        releases,
    })
}

pub async fn create_release(
    db: &PgPool,
    project_id: Uuid,
    version: &str,
    environment: &str,
    commit_sha: Option<&str>,
    deployed_at: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
) -> AppResult<AppRelease> {
    validate_non_empty(version, "release version")?;
    validate_non_empty(environment, "environment")?;

    let release = sqlx::query_as(
        "INSERT INTO app_releases \
         (project_id, version, environment, commit_sha, deployed_at, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (project_id, version, environment) DO UPDATE SET \
           commit_sha = EXCLUDED.commit_sha, \
           deployed_at = EXCLUDED.deployed_at, \
           metadata = EXCLUDED.metadata \
         RETURNING id, project_id, version, environment, commit_sha, deployed_at, metadata, created_at",
    )
    .bind(project_id)
    .bind(version.trim())
    .bind(environment.trim())
    .bind(commit_sha.map(str::trim))
    .bind(deployed_at)
    .bind(metadata)
    .fetch_one(db)
    .await?;

    Ok(release)
}

pub async fn list_releases(
    db: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AppRelease>> {
    let releases = sqlx::query_as(
        "SELECT id, project_id, version, environment, commit_sha, deployed_at, metadata, created_at \
         FROM app_releases WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    Ok(releases)
}

pub async fn delete_release(db: &PgPool, project_id: Uuid, release_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM app_releases WHERE id = $1 AND project_id = $2")
        .bind(release_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Release not found".to_string()));
    }
    Ok(())
}

pub async fn register_source_map(
    db: &PgPool,
    project_id: Uuid,
    release_version: &str,
    environment: &str,
    minified_url: &str,
    source_map_url: Option<&str>,
    artifacts: serde_json::Value,
    uploaded_by: Option<&str>,
) -> AppResult<SourceMapArtifact> {
    validate_non_empty(release_version, "release_version")?;
    validate_non_empty(environment, "environment")?;
    validate_non_empty(minified_url, "minified_url")?;

    let release_id: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM app_releases WHERE project_id = $1 AND version = $2 AND environment = $3",
    )
    .bind(project_id)
    .bind(release_version.trim())
    .bind(environment.trim())
    .fetch_optional(db)
    .await?;

    let source_map = sqlx::query_as(
        "INSERT INTO source_maps \
         (project_id, release_id, release_version, environment, minified_url, source_map_url, artifacts, uploaded_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (project_id, release_version, environment, minified_url) DO UPDATE SET \
           release_id = EXCLUDED.release_id, \
           source_map_url = EXCLUDED.source_map_url, \
           artifacts = EXCLUDED.artifacts, \
           uploaded_by = EXCLUDED.uploaded_by \
         RETURNING id, project_id, release_id, release_version, environment, minified_url, source_map_url, artifacts, uploaded_by, created_at",
    )
    .bind(project_id)
    .bind(release_id.map(|r| r.0))
    .bind(release_version.trim())
    .bind(environment.trim())
    .bind(minified_url.trim())
    .bind(source_map_url.map(str::trim))
    .bind(artifacts)
    .bind(uploaded_by.map(str::trim))
    .fetch_one(db)
    .await?;

    Ok(source_map)
}

pub async fn list_source_maps(
    db: &PgPool,
    project_id: Uuid,
    release_version: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<SourceMapArtifact>> {
    let source_maps = sqlx::query_as(
        "SELECT id, project_id, release_id, release_version, environment, minified_url, \
         source_map_url, artifacts, uploaded_by, created_at \
         FROM source_maps \
         WHERE project_id = $1 AND ($2::text IS NULL OR release_version = $2) \
         ORDER BY created_at DESC LIMIT $3 OFFSET $4",
    )
    .bind(project_id)
    .bind(release_version)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    Ok(source_maps)
}

pub async fn delete_source_map(
    db: &PgPool,
    project_id: Uuid,
    source_map_id: Uuid,
) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM source_maps WHERE id = $1 AND project_id = $2")
        .bind(source_map_id)
        .bind(project_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Source map not found".to_string()));
    }
    Ok(())
}

pub async fn list_logs(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    filters: LogFilters,
) -> AppResult<Vec<LogEntry>> {
    let level = match filters.level {
        Some(level) => Some(normalize_log_level(&level)?),
        None => None,
    };
    let search = filters.search.map(|s| format!("%{}%", s.trim()));
    let logs = sqlx::query_as(
        "SELECT id, project_id, visitor_id, session_id, level, message, body, path, release, \
         environment, browser, os, created_at \
         FROM log_entries WHERE project_id = $1 \
         AND created_at >= $2 AND created_at <= $3 \
         AND ($4::text IS NULL OR level = $4) \
         AND ($5::text IS NULL OR release = $5) \
         AND ($6::text IS NULL OR environment = $6) \
         AND ($7::text IS NULL OR message ILIKE $7) \
         ORDER BY created_at DESC LIMIT $8 OFFSET $9",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .bind(level)
    .bind(filters.release)
    .bind(filters.environment)
    .bind(search)
    .bind(filters.limit)
    .bind(filters.offset)
    .fetch_all(db)
    .await?;
    Ok(logs)
}

pub async fn get_log_stats(
    db: &PgPool,
    project_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<LogStats> {
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM log_entries \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_one(db)
    .await?;

    let levels = sqlx::query_as::<_, (String, i64)>(
        "SELECT level, COUNT(*)::bigint FROM log_entries \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
         GROUP BY level ORDER BY 2 DESC",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|(level, count)| LogLevelCount { level, count })
    .collect();

    let releases = sqlx::query_as::<_, (String, i64)>(
        "SELECT COALESCE(release, 'unknown'), COUNT(*)::bigint FROM log_entries \
         WHERE project_id = $1 AND created_at >= $2 AND created_at <= $3 \
         GROUP BY COALESCE(release, 'unknown') ORDER BY 2 DESC LIMIT 10",
    )
    .bind(project_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|(release, count)| ReleaseLogCount { release, count })
    .collect();

    Ok(LogStats {
        total: total.0,
        levels,
        releases,
    })
}

pub fn log_body(body: Option<serde_json::Value>) -> serde_json::Value {
    body.unwrap_or_else(|| json!({}))
}

fn validate_non_empty(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{error_fingerprint, normalize_log_level};

    #[test]
    fn fingerprint_is_stable_for_same_error_shape() {
        let a = error_fingerprint(
            "TypeError: missing id",
            Some("/assets/app.js"),
            Some(42),
            Some("TypeError: missing id\n at fn (/assets/app.js:42:3)"),
        );
        let b = error_fingerprint(
            "TypeError: missing id",
            Some("/assets/app.js"),
            Some(42),
            Some("TypeError: missing id\n at other (/assets/app.js:99:1)"),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn normalizes_supported_log_levels() {
        assert_eq!(normalize_log_level("WARNING").unwrap(), "warn");
        assert_eq!(normalize_log_level("error").unwrap(), "error");
        assert!(normalize_log_level("loud").is_err());
    }
}
