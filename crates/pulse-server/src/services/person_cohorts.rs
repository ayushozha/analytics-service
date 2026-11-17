//! Person cohorts (PostHog `Cohort`).
//!
//! Dynamic cohorts are defined by nested AND/OR groups of criteria:
//! - `person`: a profile trait matches (any property operator)
//! - `behavioral`: performed an event (optionally ≥/≤/= N times), stopped,
//!   restarted, did it for the first time, or did it regularly, in a window
//! - `cohort`: member of another cohort of the same project
//!
//! Each criterion can be negated. Filters compile into SQL set operations
//! (UNION / INTERSECT / EXCEPT) over the project's known visitors and
//! materialised into `cohort_people`. Static cohorts hold explicit members.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::insights::filters::{push_filter, Operator, PropertyFilter};
use crate::services::insights::{
    load_profiles, map_query_error, push_event_source, EventSource, PropertyType,
};

/// Dynamic cohorts older than this are recalculated by the background task.
pub const RECALC_AFTER_MINUTES: i64 = 30;
pub const RECALC_INTERVAL_SECS: u64 = 300;
pub const MAX_STATIC_MEMBERS: usize = 100_000;
/// A calculation claim older than this is assumed dead (crashed instance).
pub const CLAIM_LEASE_MINUTES: i32 = 10;
const MAX_CRITERIA: usize = 50;
const MAX_NESTING: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeInterval {
    Day,
    Week,
    Month,
}

impl TimeInterval {
    fn duration(self, n: i64) -> Duration {
        match self {
            TimeInterval::Day => Duration::days(n),
            TimeInterval::Week => Duration::weeks(n),
            TimeInterval::Month => Duration::days(30 * n),
        }
    }

    fn sql(self) -> &'static str {
        match self {
            TimeInterval::Day => "day",
            TimeInterval::Week => "week",
            TimeInterval::Month => "month",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Behavior {
    /// Did the event at least once in the window.
    PerformedEvent,
    /// Did the event `count_operator` `count` times in the window.
    PerformedEventMultiple,
    /// Did it earlier in the window but not in the last `seq_time_value` units.
    StoppedPerformingEvent,
    /// Did it in the last `seq_time_value` units, not earlier in the window, but before the window.
    RestartedPerformingEvent,
    /// First-ever occurrence falls inside the window.
    PerformedEventFirstTime,
    /// Did it in at least `min_periods` of the last `total_periods` intervals.
    PerformedEventRegularly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CountOperator {
    Gte,
    Lte,
    Exact,
}

/// One cohort criterion.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Criterion {
    Person {
        key: String,
        #[serde(default = "default_op")]
        operator: Operator,
        #[serde(default)]
        value: Value,
        #[serde(default)]
        negation: bool,
    },
    Behavioral {
        value: Behavior,
        /// Event name (`$pageview` for pageviews).
        key: String,
        #[serde(default)]
        event_properties: Vec<PropertyFilter>,
        /// Window length.
        #[serde(default = "default_time_value")]
        time_value: i64,
        #[serde(default = "default_interval")]
        time_interval: TimeInterval,
        /// Stopped / restarted: the recent sub-window.
        #[serde(default)]
        seq_time_value: Option<i64>,
        #[serde(default)]
        count: Option<i64>,
        #[serde(default)]
        count_operator: Option<CountOperator>,
        /// Regularly: required active periods ...
        #[serde(default)]
        min_periods: Option<i64>,
        /// ... out of this many most recent periods.
        #[serde(default)]
        total_periods: Option<i64>,
        #[serde(default)]
        negation: bool,
    },
    Cohort {
        /// Cohort id.
        key: Uuid,
        #[serde(default)]
        negation: bool,
    },
}

fn default_op() -> Operator {
    Operator::Exact
}
fn default_time_value() -> i64 {
    30
}
fn default_interval() -> TimeInterval {
    TimeInterval::Day
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Join {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Node {
    Group(Group),
    Criterion(Criterion),
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Group {
    #[serde(rename = "type", default)]
    pub join: Join,
    pub values: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Cohort {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_static: bool,
    pub filters: Value,
    pub member_count: i32,
    pub is_calculating: bool,
    pub last_calculated_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const COLUMNS: &str = "id, project_id, name, description, is_static, filters, member_count, \
    is_calculating, last_calculated_at, last_error, created_at, updated_at";

fn bad(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}

/// Parse and structurally validate a filter tree; returns referenced cohorts.
pub fn parse_filters(v: &Value) -> AppResult<(Group, Vec<Uuid>)> {
    let g: Group = serde_json::from_value(v.clone())
        .map_err(|e| bad(format!("invalid cohort filters: {e}")))?;
    let mut refs = Vec::new();
    let mut count = 0;
    validate_group(&g, 0, &mut count, &mut refs)?;
    if count == 0 {
        return Err(bad("a dynamic cohort needs at least one criterion"));
    }
    Ok((g, refs))
}

fn validate_group(
    g: &Group,
    depth: usize,
    count: &mut usize,
    refs: &mut Vec<Uuid>,
) -> AppResult<()> {
    if depth >= MAX_NESTING {
        return Err(bad(format!(
            "cohort groups nest at most {MAX_NESTING} levels"
        )));
    }
    for n in &g.values {
        match n {
            Node::Group(inner) => validate_group(inner, depth + 1, count, refs)?,
            Node::Criterion(c) => {
                *count += 1;
                if *count > MAX_CRITERIA {
                    return Err(bad(format!("at most {MAX_CRITERIA} criteria")));
                }
                validate_criterion(c, refs)?;
            }
        }
    }
    Ok(())
}

fn validate_criterion(c: &Criterion, refs: &mut Vec<Uuid>) -> AppResult<()> {
    match c {
        Criterion::Person { key, .. } => {
            if key.is_empty() || key.len() > 200 {
                return Err(bad("person property key must be 1-200 characters"));
            }
        }
        Criterion::Behavioral {
            value,
            key,
            time_value,
            seq_time_value,
            count,
            count_operator,
            min_periods,
            total_periods,
            ..
        } => {
            if key.is_empty() || key.len() > 255 {
                return Err(bad("event name must be 1-255 characters"));
            }
            if !(1..=3650).contains(time_value) {
                return Err(bad("time_value must be 1-3650"));
            }
            match value {
                Behavior::PerformedEventMultiple => {
                    if count.is_none_or(|c| c < 0) || count_operator.is_none() {
                        return Err(bad(
                            "performed_event_multiple needs count and count_operator",
                        ));
                    }
                }
                Behavior::StoppedPerformingEvent | Behavior::RestartedPerformingEvent => {
                    if seq_time_value.is_none_or(|s| s < 1 || s >= *time_value) {
                        return Err(bad(
                            "seq_time_value must be at least 1 and smaller than time_value",
                        ));
                    }
                }
                Behavior::PerformedEventRegularly => match (min_periods, total_periods) {
                    (Some(m), Some(t)) if *m >= 1 && m <= t && *t <= 365 => {}
                    _ => return Err(bad(
                        "performed_event_regularly needs 1 <= min_periods <= total_periods <= 365",
                    )),
                },
                _ => {}
            }
        }
        Criterion::Cohort { key, .. } => refs.push(*key),
    }
    Ok(())
}

/// Referenced cohorts must exist in the project and not lead back to `self_id`.
async fn check_references(
    db: &PgPool,
    project_id: Uuid,
    self_id: Option<Uuid>,
    refs: Vec<Uuid>,
) -> AppResult<()> {
    let mut seen: HashSet<Uuid> = HashSet::new();
    let mut stack = refs;
    while let Some(id) = stack.pop() {
        if Some(id) == self_id {
            return Err(bad(
                "a cohort cannot reference itself, directly or indirectly",
            ));
        }
        if !seen.insert(id) {
            continue;
        }
        if seen.len() > 50 {
            return Err(bad("too many nested cohort references"));
        }
        let row: Option<(bool, Value)> = sqlx::query_as(
            "SELECT is_static, filters FROM person_cohorts WHERE id = $1 AND project_id = $2 AND NOT deleted",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(db)
        .await?;
        let (is_static, filters) =
            row.ok_or_else(|| AppError::NotFound(format!("Cohort {id} not found")))?;
        if !is_static {
            if let Ok((_, more)) = parse_filters(&filters) {
                stack.extend(more);
            }
        }
    }
    Ok(())
}

/// All visitors the project has seen (the universe for negation).
fn push_universe(qb: &mut QueryBuilder<'_, Postgres>, project_id: Uuid) {
    qb.push("(SELECT visitor_id FROM sessions WHERE project_id = ");
    qb.push_bind(project_id);
    qb.push(" UNION SELECT visitor_id FROM user_profiles WHERE project_id = ");
    qb.push_bind(project_id);
    qb.push(")");
}

/// Matching events for a behavioural criterion between two instants.
fn push_events(
    qb: &mut QueryBuilder<'_, Postgres>,
    project_id: Uuid,
    event: &str,
    props: &[PropertyFilter],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<()> {
    let names = [event.to_string()];
    push_event_source(
        qb,
        &EventSource {
            project_id,
            from,
            to,
            events: Some(&names),
            joins: Default::default(),
        },
    );
    qb.push(" WHERE TRUE");
    for f in props {
        if f.property_type != PropertyType::Event {
            return Err(bad("behavioral event_properties must be event properties"));
        }
        qb.push(" AND ");
        push_filter(qb, f)?;
    }
    Ok(())
}

fn push_criterion(
    qb: &mut QueryBuilder<'_, Postgres>,
    project_id: Uuid,
    c: &Criterion,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let negation = match c {
        Criterion::Person { negation, .. }
        | Criterion::Behavioral { negation, .. }
        | Criterion::Cohort { negation, .. } => *negation,
    };
    if negation {
        qb.push("(");
        push_universe(qb, project_id);
        qb.push(" EXCEPT ");
    }
    qb.push("(");
    match c {
        Criterion::Person {
            key,
            operator,
            value,
            ..
        } => {
            qb.push("SELECT up.visitor_id FROM user_profiles up WHERE up.project_id = ");
            qb.push_bind(project_id);
            qb.push(" AND ");
            push_filter(
                qb,
                &PropertyFilter {
                    key: key.clone(),
                    property_type: PropertyType::Person,
                    operator: *operator,
                    value: value.clone(),
                    group_type_index: None,
                },
            )?;
        }
        Criterion::Cohort { key, .. } => {
            qb.push(
                "SELECT cp.visitor_id FROM cohort_people cp JOIN person_cohorts c ON c.id = cp.cohort_id \
                 WHERE c.project_id = ",
            );
            qb.push_bind(project_id);
            qb.push(" AND cp.cohort_id = ");
            qb.push_bind(*key);
        }
        Criterion::Behavioral {
            value,
            key,
            event_properties,
            time_value,
            time_interval,
            seq_time_value,
            count,
            count_operator,
            min_periods,
            total_periods,
            ..
        } => {
            let start = now - time_interval.duration(*time_value);
            let epoch = DateTime::<Utc>::UNIX_EPOCH;
            match value {
                Behavior::PerformedEvent => {
                    qb.push("SELECT DISTINCT ev.visitor_id");
                    push_events(qb, project_id, key, event_properties, start, now)?;
                }
                Behavior::PerformedEventMultiple => {
                    qb.push("SELECT ev.visitor_id");
                    push_events(qb, project_id, key, event_properties, start, now)?;
                    let op = match count_operator.expect("validated") {
                        CountOperator::Gte => " >= ",
                        CountOperator::Lte => " <= ",
                        CountOperator::Exact => " = ",
                    };
                    let n = count.expect("validated");
                    qb.push(" GROUP BY ev.visitor_id HAVING COUNT(*)");
                    qb.push(op);
                    qb.push_bind(n);
                    // Visitors who never did the event in the window have a
                    // count of 0: they satisfy "<= N", "= 0" and ">= 0".
                    let include_zero = match count_operator.expect("validated") {
                        CountOperator::Lte => true,
                        CountOperator::Exact | CountOperator::Gte => n == 0,
                    };
                    if include_zero {
                        qb.push(" UNION (SELECT visitor_id FROM ");
                        push_universe(qb, project_id);
                        qb.push(" AS u EXCEPT SELECT DISTINCT ev.visitor_id");
                        push_events(qb, project_id, key, event_properties, start, now)?;
                        qb.push(")");
                    }
                }
                Behavior::StoppedPerformingEvent | Behavior::RestartedPerformingEvent => {
                    let recent = now - time_interval.duration(seq_time_value.expect("validated"));
                    let stopped = *value == Behavior::StoppedPerformingEvent;
                    // stopped: did in [start, recent) and not in [recent, now]
                    // restarted: did in [recent, now], not in [start, recent), and before start
                    // ((A) EXCEPT (B)) [INTERSECT (C)]: INTERSECT binds tighter
                    // than EXCEPT, so the difference is parenthesised.
                    qb.push("((SELECT DISTINCT ev.visitor_id");
                    if stopped {
                        push_events(qb, project_id, key, event_properties, start, recent)?;
                    } else {
                        push_events(qb, project_id, key, event_properties, recent, now)?;
                    }
                    qb.push(") EXCEPT (SELECT DISTINCT ev.visitor_id");
                    if stopped {
                        push_events(qb, project_id, key, event_properties, recent, now)?;
                    } else {
                        push_events(qb, project_id, key, event_properties, start, recent)?;
                    }
                    qb.push("))");
                    if !stopped {
                        qb.push(" INTERSECT (SELECT DISTINCT ev.visitor_id");
                        push_events(qb, project_id, key, event_properties, epoch, start)?;
                        qb.push(")");
                    }
                }
                Behavior::PerformedEventFirstTime => {
                    qb.push("SELECT ev.visitor_id");
                    push_events(qb, project_id, key, event_properties, epoch, now)?;
                    qb.push(" GROUP BY ev.visitor_id HAVING MIN(ev.created_at) >= ");
                    qb.push_bind(start);
                }
                Behavior::PerformedEventRegularly => {
                    let total = total_periods.expect("validated");
                    let from = now - time_interval.duration(total);
                    qb.push("SELECT ev.visitor_id");
                    push_events(qb, project_id, key, event_properties, from, now)?;
                    qb.push(" GROUP BY ev.visitor_id HAVING COUNT(DISTINCT date_trunc(");
                    qb.push_bind(time_interval.sql());
                    qb.push(", ev.created_at)) >= ");
                    qb.push_bind(min_periods.expect("validated"));
                }
            }
        }
    }
    qb.push(")");
    if negation {
        qb.push(")");
    }
    Ok(())
}

fn push_group(
    qb: &mut QueryBuilder<'_, Postgres>,
    project_id: Uuid,
    g: &Group,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let op = match g.join {
        Join::And => " INTERSECT ",
        Join::Or => " UNION ",
    };
    qb.push("(");
    let mut first = true;
    for n in &g.values {
        if !first {
            qb.push(op);
        }
        first = false;
        qb.push("SELECT visitor_id FROM ");
        match n {
            Node::Group(inner) => push_group(qb, project_id, inner, now)?,
            Node::Criterion(c) => push_criterion(qb, project_id, c, now)?,
        }
        qb.push(" AS s");
    }
    if first {
        // Empty group: nobody.
        qb.push("SELECT NULL::varchar AS visitor_id WHERE FALSE");
    }
    qb.push(")");
    Ok(())
}

/// SQL selecting the members of a dynamic cohort (inspected by tests).
#[cfg(test)]
fn members_sql<'a>(
    project_id: Uuid,
    g: &Group,
    now: DateTime<Utc>,
) -> AppResult<QueryBuilder<'a, Postgres>> {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT DISTINCT visitor_id FROM ");
    push_group(&mut qb, project_id, g, now)?;
    qb.push(" AS members WHERE visitor_id IS NOT NULL");
    Ok(qb)
}

// ---------------------------------------------------------------- CRUD

#[derive(Debug, Clone, Deserialize)]
pub struct CohortInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_static: bool,
    #[serde(default)]
    pub filters: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CohortPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub filters: Option<Value>,
}

fn validate_name(n: &str) -> AppResult<()> {
    if n.trim().is_empty() || n.len() > 255 {
        return Err(bad("name must be 1-255 characters"));
    }
    Ok(())
}

pub async fn create(db: &PgPool, project_id: Uuid, input: &CohortInput) -> AppResult<Cohort> {
    validate_name(&input.name)?;
    let filters = if input.is_static {
        if input.filters.is_some() {
            return Err(bad("static cohorts have no filters; add members instead"));
        }
        json!({})
    } else {
        let f = input
            .filters
            .clone()
            .ok_or_else(|| bad("dynamic cohorts need filters"))?;
        let (_, refs) = parse_filters(&f)?;
        check_references(db, project_id, None, refs).await?;
        f
    };
    let c: Cohort = sqlx::query_as(&format!(
        "INSERT INTO person_cohorts (project_id, name, description, is_static, filters) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {COLUMNS}"
    ))
    .bind(project_id)
    .bind(input.name.trim())
    .bind(&input.description)
    .bind(input.is_static)
    .bind(&filters)
    .fetch_one(db)
    .await?;
    if !c.is_static {
        return calculate(db, project_id, c.id).await;
    }
    Ok(c)
}

pub async fn get(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Cohort> {
    sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM person_cohorts WHERE id = $1 AND project_id = $2 AND NOT deleted"
    ))
    .bind(id)
    .bind(project_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Cohort not found".into()))
}

pub async fn list(db: &PgPool, project_id: Uuid) -> AppResult<Vec<Cohort>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM person_cohorts WHERE project_id = $1 AND NOT deleted ORDER BY lower(name), id"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?)
}

pub async fn update(db: &PgPool, project_id: Uuid, id: Uuid, p: &CohortPatch) -> AppResult<Cohort> {
    let cur = get(db, project_id, id).await?;
    if let Some(n) = &p.name {
        validate_name(n)?;
    }
    if let Some(f) = &p.filters {
        if cur.is_static {
            return Err(bad("static cohorts have no filters"));
        }
        let (_, refs) = parse_filters(f)?;
        check_references(db, project_id, Some(id), refs).await?;
    }
    let c: Cohort = sqlx::query_as(&format!(
        "UPDATE person_cohorts SET name = COALESCE($3, name), description = COALESCE($4, description), \
         filters = COALESCE($5, filters), updated_at = NOW() WHERE id = $1 AND project_id = $2 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(project_id)
    .bind(p.name.as_deref().map(str::trim))
    .bind(&p.description)
    .bind(&p.filters)
    .fetch_one(db)
    .await?;
    if p.filters.is_some() {
        return calculate(db, project_id, id).await;
    }
    Ok(c)
}

pub async fn delete(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<()> {
    get(db, project_id, id).await?;
    // Refuse while another cohort depends on this one.
    let dependents: Vec<(String, Value)> = sqlx::query_as(
        "SELECT name, filters FROM person_cohorts WHERE project_id = $1 AND NOT deleted AND NOT is_static AND id <> $2",
    )
    .bind(project_id)
    .bind(id)
    .fetch_all(db)
    .await?;
    for (name, f) in dependents {
        if parse_filters(&f).is_ok_and(|(_, refs)| refs.contains(&id)) {
            return Err(bad(format!(
                "cohort '{name}' uses this cohort; remove that criterion first"
            )));
        }
    }
    sqlx::query("UPDATE person_cohorts SET deleted = true, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM cohort_people WHERE cohort_id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Recalculate a dynamic cohort's members (atomic swap) and record its count.
pub async fn calculate(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Cohort> {
    let c = get(db, project_id, id).await?;
    if c.is_static {
        return Err(bad("static cohorts are not calculated"));
    }
    let (g, _) = parse_filters(&c.filters)?;
    // Claim the cohort so concurrent calculations (other instances) skip it.
    let claimed: Option<(Uuid,)> = sqlx::query_as(
        "UPDATE person_cohorts SET is_calculating = true, calc_started_at = NOW() WHERE id = $1 \
         AND (NOT is_calculating OR calc_started_at < NOW() - make_interval(mins => $2)) RETURNING id",
    )
    .bind(id)
    .bind(CLAIM_LEASE_MINUTES)
    .fetch_optional(db)
    .await?;
    if claimed.is_none() {
        return Err(AppError::BadRequest(
            "cohort is already being calculated".into(),
        ));
    }
    let result = recalc(db, project_id, id, &g).await;
    let (count, error) = match &result {
        Ok(n) => (Some(*n), None),
        Err(e) => (None, Some(e.to_string())),
    };
    sqlx::query(
        "UPDATE person_cohorts SET is_calculating = false, last_error = $2, \
         member_count = COALESCE($3, member_count), \
         last_calculated_at = CASE WHEN $3 IS NULL THEN last_calculated_at ELSE NOW() END \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&error)
    .bind(count)
    .execute(db)
    .await?;
    result?;
    get(db, project_id, id).await
}

async fn recalc(db: &PgPool, project_id: Uuid, id: Uuid, g: &Group) -> AppResult<i32> {
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM cohort_people WHERE cohort_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let mut qb =
        QueryBuilder::<Postgres>::new("INSERT INTO cohort_people (cohort_id, visitor_id) SELECT ");
    qb.push_bind(id);
    qb.push(", visitor_id FROM ");
    push_group(&mut qb, project_id, g, Utc::now())?;
    qb.push(" AS members WHERE visitor_id IS NOT NULL ON CONFLICT DO NOTHING");
    let n = qb
        .build()
        .execute(&mut *tx)
        .await
        .map_err(map_query_error)?
        .rows_affected() as i32;
    sqlx::query("INSERT INTO cohort_counts (cohort_id, member_count) VALUES ($1, $2)")
        .bind(id)
        .bind(n)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(n)
}

/// Background task: recalculate stale dynamic cohorts.
pub fn start_recalc_task(state: crate::state::SharedState) {
    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(std::time::Duration::from_secs(RECALC_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let due: Vec<(Uuid, Uuid)> = match sqlx::query_as(
                "SELECT id, project_id FROM person_cohorts WHERE NOT deleted AND NOT is_static \
                 AND (NOT is_calculating OR calc_started_at < NOW() - make_interval(mins => $2)) \
                 AND (last_calculated_at IS NULL OR last_calculated_at < NOW() - make_interval(mins => $1))",
            )
            .bind(RECALC_AFTER_MINUTES as i32)
            .bind(CLAIM_LEASE_MINUTES)
            .fetch_all(&state.db)
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("cohort recalculation listing failed: {e}");
                    continue;
                }
            };
            for (id, project_id) in due {
                if let Err(e) = calculate(&state.db, project_id, id).await {
                    tracing::warn!("cohort {id} recalculation failed: {e}");
                }
            }
        }
    });
}

#[derive(Debug, Serialize, FromRow)]
pub struct CountPoint {
    pub member_count: i32,
    pub calculated_at: DateTime<Utc>,
}

pub async fn history(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Vec<CountPoint>> {
    get(db, project_id, id).await?;
    Ok(sqlx::query_as(
        "SELECT member_count, calculated_at FROM cohort_counts WHERE cohort_id = $1 ORDER BY calculated_at DESC LIMIT 500",
    )
    .bind(id)
    .fetch_all(db)
    .await?)
}

// ---------------------------------------------------------------- members

/// Resolve identifiers (visitor ids or identified user ids) to the visitor
/// ids known in this project, as `(input id, visitor id)` pairs.
pub async fn resolve_visitors(
    db: &PgPool,
    project_id: Uuid,
    ids: &[String],
) -> AppResult<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT DISTINCT i.id::text, x.visitor_id FROM unnest($2::varchar[]) AS i(id) \
         JOIN LATERAL ( \
           SELECT visitor_id FROM user_profiles WHERE project_id = $1 AND (visitor_id = i.id OR user_id = i.id) \
           UNION SELECT visitor_id FROM user_aliases WHERE project_id = $1 AND user_id = i.id \
           UNION SELECT visitor_id FROM sessions WHERE project_id = $1 AND visitor_id = i.id \
         ) x ON TRUE",
    )
    .bind(project_id)
    .bind(ids)
    .fetch_all(db)
    .await?)
}

/// Add members to a static cohort. Returns `(added, unknown_ids)`.
pub async fn add_members(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
    ids: &[String],
) -> AppResult<(u64, usize)> {
    let c = get(db, project_id, id).await?;
    if !c.is_static {
        return Err(bad("members can only be added to static cohorts"));
    }
    if ids.len() > MAX_STATIC_MEMBERS {
        return Err(bad(format!("at most {MAX_STATIC_MEMBERS} ids per request")));
    }
    let pairs = resolve_visitors(db, project_id, ids).await?;
    let matched: HashSet<&str> = pairs.iter().map(|(input, _)| input.as_str()).collect();
    let unknown = ids.iter().filter(|i| !matched.contains(i.as_str())).count();
    let visitors: Vec<String> = pairs.iter().map(|(_, v)| v.clone()).collect();
    let added = sqlx::query(
        "INSERT INTO cohort_people (cohort_id, visitor_id) SELECT $1, unnest($2::varchar[]) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(&visitors)
    .execute(db)
    .await?
    .rows_affected();
    refresh_static_count(db, id).await?;
    Ok((added, unknown))
}

pub async fn remove_members(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
    visitor_ids: &[String],
) -> AppResult<u64> {
    let c = get(db, project_id, id).await?;
    if !c.is_static {
        return Err(bad("members can only be removed from static cohorts"));
    }
    let n = sqlx::query("DELETE FROM cohort_people WHERE cohort_id = $1 AND visitor_id = ANY($2)")
        .bind(id)
        .bind(visitor_ids)
        .execute(db)
        .await?
        .rows_affected();
    refresh_static_count(db, id).await?;
    Ok(n)
}

async fn refresh_static_count(db: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE person_cohorts SET member_count = (SELECT COUNT(*) FROM cohort_people WHERE cohort_id = $1), \
         last_calculated_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(db)
    .await?;
    sqlx::query("INSERT INTO cohort_counts (cohort_id, member_count) SELECT $1, member_count FROM person_cohorts WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Parse a CSV of ids (first column; an optional header row is skipped).
pub fn parse_csv_ids(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, line) in body.lines().enumerate() {
        let first = line
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .trim();
        if first.is_empty() {
            continue;
        }
        if i == 0
            && matches!(
                first.to_ascii_lowercase().as_str(),
                "visitor_id" | "user_id" | "distinct_id" | "id" | "email"
            )
        {
            continue;
        }
        out.push(first.to_string());
    }
    out
}

/// Duplicate any cohort's current members as a new static cohort.
pub async fn duplicate_as_static(db: &PgPool, project_id: Uuid, id: Uuid) -> AppResult<Cohort> {
    let src = get(db, project_id, id).await?;
    let name: String = format!("{} (static copy)", src.name)
        .chars()
        .take(255)
        .collect();
    let copy = create(
        db,
        project_id,
        &CohortInput {
            name,
            description: src.description,
            is_static: true,
            filters: None,
        },
    )
    .await?;
    sqlx::query("INSERT INTO cohort_people (cohort_id, visitor_id) SELECT $1, visitor_id FROM cohort_people WHERE cohort_id = $2")
        .bind(copy.id)
        .bind(id)
        .execute(db)
        .await?;
    refresh_static_count(db, copy.id).await?;
    get(db, project_id, copy.id).await
}

pub async fn members(
    db: &PgPool,
    project_id: Uuid,
    id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Value> {
    let c = get(db, project_id, id).await?;
    if !(1..=10_000).contains(&limit) || offset < 0 {
        return Err(bad("limit must be 1-10000 and offset >= 0"));
    }
    let ids: Vec<(String,)> = sqlx::query_as(
        "SELECT visitor_id FROM cohort_people WHERE cohort_id = $1 ORDER BY visitor_id LIMIT $2 OFFSET $3",
    )
    .bind(id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    let ids: Vec<String> = ids.into_iter().map(|r| r.0).collect();
    let profiles = load_profiles(db, project_id, &ids).await?;
    let data: Vec<Value> = ids
        .iter()
        .map(|v| {
            let (user_id, traits) = profiles.get(v).cloned().unwrap_or((None, json!({})));
            json!({ "visitor_id": v, "user_id": user_id, "traits": traits })
        })
        .collect();
    Ok(json!({ "data": data, "total": c.member_count, "limit": limit, "offset": offset }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(v: Value) -> Group {
        parse_filters(&v).unwrap().0
    }

    #[test]
    fn parses_and_validates_trees() {
        let (grp, refs) = parse_filters(&json!({"type": "OR", "values": [
            {"type": "person", "key": "plan", "value": "pro"},
            {"type": "AND", "values": [
                {"type": "behavioral", "value": "performed_event_multiple", "key": "purchase",
                 "count": 2, "count_operator": "gte", "time_value": 30},
                {"type": "cohort", "key": "6f1c1c1e-0000-0000-0000-000000000001", "negation": true}
            ]}
        ]}))
        .unwrap();
        assert_eq!(grp.join, Join::Or);
        assert_eq!(refs.len(), 1);
        assert!(parse_filters(&json!({"values": []})).is_err());
        assert!(parse_filters(&json!({"values": [{"type": "behavioral", "value": "performed_event_multiple", "key": "x"}]})).is_err());
        assert!(parse_filters(&json!({"values": [{"type": "behavioral", "value": "stopped_performing_event", "key": "x", "time_value": 30, "seq_time_value": 30}]})).is_err());
        assert!(parse_filters(&json!({"values": [{"type": "behavioral", "value": "performed_event_regularly", "key": "x", "min_periods": 5, "total_periods": 3}]})).is_err());
    }

    #[test]
    fn compiles_set_operations() {
        let now = Utc::now();
        let grp = g(json!({"type": "AND", "values": [
            {"type": "person", "key": "plan", "value": "pro"},
            {"type": "behavioral", "value": "performed_event", "key": "signup", "negation": true}
        ]}));
        let sql = members_sql(Uuid::nil(), &grp, now)
            .unwrap()
            .sql()
            .to_string();
        assert!(sql.contains(" INTERSECT "));
        assert!(sql.contains(" EXCEPT "));
        assert!(sql.contains("FROM user_profiles up WHERE up.project_id = "));
        let grp = g(json!({"type": "OR", "values": [
            {"type": "behavioral", "value": "stopped_performing_event", "key": "a", "time_value": 30, "seq_time_value": 7},
            {"type": "behavioral", "value": "performed_event_first_time", "key": "a", "time_value": 7}
        ]}));
        let sql = members_sql(Uuid::nil(), &grp, now)
            .unwrap()
            .sql()
            .to_string();
        assert!(sql.contains(" UNION "));
        assert!(sql.contains("HAVING MIN(ev.created_at) >= "));
    }

    #[test]
    fn csv_parsing() {
        assert_eq!(
            parse_csv_ids("visitor_id\nv1\n\"v2\",x\n\n v3 \n"),
            vec!["v1", "v2", "v3"]
        );
        assert_eq!(parse_csv_ids("u1\nu2"), vec!["u1", "u2"]);
    }
}
