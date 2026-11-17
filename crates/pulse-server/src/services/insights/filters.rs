//! Property filters (PostHog `PropertyFilter` / `PropertyGroupFilter`) compiled
//! to SQL predicates over the virtual event stream (`ev`), its session (`s`)
//! and the visitor's profile (`up`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder};

use super::{bad_request, PropertyType, SESSION_PROPERTIES};
use crate::error::AppResult;
use crate::services::groups::MAX_GROUP_TYPES;

/// Maximum nesting depth of filter groups.
const MAX_GROUP_DEPTH: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Exact,
    IsNot,
    Icontains,
    NotIcontains,
    Regex,
    NotRegex,
    Gt,
    Gte,
    Lt,
    Lte,
    IsSet,
    IsNotSet,
    IsDateBefore,
    IsDateAfter,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PropertyFilter {
    pub key: String,
    #[serde(rename = "type", default)]
    pub property_type: PropertyType,
    #[serde(default = "default_operator")]
    pub operator: Operator,
    /// String, number, bool, or an array of them (for `exact` / `is_not`).
    #[serde(default)]
    pub value: serde_json::Value,
    /// Group type for `type: "group"` filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_type_index: Option<i16>,
}

fn default_operator() -> Operator {
    Operator::Exact
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum GroupType {
    #[default]
    And,
    Or,
}

/// A filter node: either a single property filter or a nested AND/OR group.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FilterNode {
    Group(FilterGroup),
    Property(PropertyFilter),
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FilterGroup {
    #[serde(rename = "type", default)]
    pub group_type: GroupType,
    pub values: Vec<FilterNode>,
}

/// Which joins a query needs, discovered before SQL is written.
#[derive(Debug, Default, Clone, Copy)]
pub struct Joins {
    pub session: bool,
    pub person: bool,
    /// A cohort filter is used (needs the `cohorts` module).
    pub cohort: bool,
    /// Bit `i` set: group type `i` is joined as `g<i>`.
    pub groups: u8,
}

/// Which protected data a query reads, for module authorization.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DataNeeds {
    /// Person (profile) properties: `identity` module.
    pub person: bool,
    /// Cohort membership: `cohorts` module.
    pub cohorts: bool,
}

impl From<Joins> for DataNeeds {
    fn from(j: Joins) -> Self {
        Self {
            // Group properties are governed like person properties.
            person: j.person || j.groups != 0,
            cohorts: j.cohort,
        }
    }
}

impl Joins {
    pub fn add(&mut self, t: PropertyType, group: Option<i16>) {
        match t {
            PropertyType::Event => {}
            PropertyType::Group => {
                if let Some(i) = group.filter(|i| (0..MAX_GROUP_TYPES).contains(i)) {
                    self.groups |= 1 << i;
                }
            }
            PropertyType::Cohort => self.cohort = true,
            PropertyType::Session => self.session = true,
            PropertyType::Person => self.person = true,
        }
    }

    pub fn add_filters(&mut self, filters: &[PropertyFilter]) {
        for f in filters {
            self.add(f.property_type, f.group_type_index);
        }
    }

    pub fn add_group(&mut self, group: &FilterGroup) {
        for node in &group.values {
            match node {
                FilterNode::Property(f) => self.add(f.property_type, f.group_type_index),
                FilterNode::Group(g) => self.add_group(g),
            }
        }
    }
}

/// Validate a property reference before any SQL is built.
pub fn validate_property(key: &str, t: PropertyType, group: Option<i16>) -> AppResult<()> {
    if t == PropertyType::Cohort {
        return Err(bad_request(
            "cohorts can only be used as filters (type \"cohort\", operator exact or is_not)",
        ));
    }
    if key.is_empty() || key.len() > 200 {
        return Err(bad_request("property key must be 1-200 characters"));
    }
    if t == PropertyType::Group && !group.is_some_and(|i| (0..MAX_GROUP_TYPES).contains(&i)) {
        return Err(bad_request(format!(
            "group properties need group_type_index 0-{}",
            MAX_GROUP_TYPES - 1
        )));
    }
    if t == PropertyType::Session && !SESSION_PROPERTIES.contains(&key) {
        return Err(bad_request(format!(
            "unknown session property '{key}' (allowed: {})",
            SESSION_PROPERTIES.join(", ")
        )));
    }
    Ok(())
}

/// Push the text-valued SQL expression for a property.
pub fn push_property_expr(
    qb: &mut QueryBuilder<'_, Postgres>,
    key: &str,
    t: PropertyType,
    group: Option<i16>,
) {
    match t {
        PropertyType::Group => {
            // The index is checked by validate_property; `g<i>` is joined by push_event_source.
            let i = group.unwrap_or_default();
            debug_assert!((0..MAX_GROUP_TYPES).contains(&i));
            qb.push(format!("(g{i}.properties ->> "));
            qb.push_bind(key.to_string());
            qb.push(")");
        }
        PropertyType::Event => {
            qb.push("(ev.properties ->> ");
            qb.push_bind(key.to_string());
            qb.push(")");
        }
        PropertyType::Session => {
            // `key` is checked against SESSION_PROPERTIES by validate_property.
            debug_assert!(SESSION_PROPERTIES.contains(&key));
            qb.push(format!("(s.{key})::text"));
        }
        PropertyType::Cohort => unreachable!("validate_property rejects cohort expressions"),
        PropertyType::Person => {
            if key == "user_id" {
                qb.push("up.user_id");
            } else {
                qb.push("(up.traits ->> ");
                qb.push_bind(key.to_string());
                qb.push(")");
            }
        }
    }
}

/// Push a numeric expression for a property; non-numeric values become NULL.
pub fn push_numeric_expr(
    qb: &mut QueryBuilder<'_, Postgres>,
    key: &str,
    t: PropertyType,
    group: Option<i16>,
) {
    qb.push("(CASE WHEN ");
    push_property_expr(qb, key, t, group);
    qb.push(" ~ '^\\s*-?[0-9]+(\\.[0-9]+)?([eE][-+]?[0-9]+)?\\s*$' THEN (");
    push_property_expr(qb, key, t, group);
    qb.push(")::double precision END)");
}

fn scalar_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn values_as_strings(v: &serde_json::Value) -> AppResult<Vec<String>> {
    match v {
        serde_json::Value::Array(items) => items
            .iter()
            .map(|i| {
                scalar_to_string(i).ok_or_else(|| bad_request("filter values must be scalars"))
            })
            .collect(),
        other => scalar_to_string(other)
            .map(|s| vec![s])
            .ok_or_else(|| bad_request("filter value is required")),
    }
}

fn value_as_f64(v: &serde_json::Value) -> AppResult<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .ok_or_else(|| bad_request("numeric operators need a number value"))
}

fn value_as_datetime(v: &serde_json::Value) -> AppResult<DateTime<Utc>> {
    let s = v
        .as_str()
        .ok_or_else(|| bad_request("date operators need an ISO-8601 string value"))?;
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight").and_utc())
        .map_err(|_| bad_request(format!("invalid date '{s}'")))
}

/// Escape `%`, `_` and `\` so a value is matched literally by ILIKE.
fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Push `ev.visitor_id [NOT] IN <cohort members>`. The cohort must belong to
/// the same project as the event row, so ids from other projects match nothing.
fn push_cohort_filter(qb: &mut QueryBuilder<'_, Postgres>, f: &PropertyFilter) -> AppResult<()> {
    let id: uuid::Uuid = f
        .key
        .parse()
        .map_err(|_| bad_request("cohort filter key must be a cohort id"))?;
    let negate = match f.operator {
        Operator::Exact => false,
        Operator::IsNot => true,
        _ => {
            return Err(bad_request(
                "cohort filters support operator exact or is_not",
            ))
        }
    };
    qb.push(if negate { "NOT " } else { "" });
    qb.push(
        "EXISTS (SELECT 1 FROM cohort_people cp JOIN person_cohorts c ON c.id = cp.cohort_id \
         WHERE cp.cohort_id = ",
    );
    qb.push_bind(id);
    qb.push(
        " AND cp.visitor_id = ev.visitor_id AND c.project_id = ev.project_id AND NOT c.deleted)",
    );
    Ok(())
}

/// Push one property predicate.
pub fn push_filter(qb: &mut QueryBuilder<'_, Postgres>, f: &PropertyFilter) -> AppResult<()> {
    if f.property_type == PropertyType::Cohort {
        return push_cohort_filter(qb, f);
    }
    validate_property(&f.key, f.property_type, f.group_type_index)?;
    let g = f.group_type_index;
    let expr =
        |qb: &mut QueryBuilder<'_, Postgres>| push_property_expr(qb, &f.key, f.property_type, g);
    match f.operator {
        Operator::Exact => {
            expr(qb);
            qb.push(" = ANY(");
            qb.push_bind(values_as_strings(&f.value)?);
            qb.push(")");
        }
        Operator::IsNot => {
            qb.push("(");
            expr(qb);
            qb.push(" IS NULL OR NOT (");
            expr(qb);
            qb.push(" = ANY(");
            qb.push_bind(values_as_strings(&f.value)?);
            qb.push(")))");
        }
        Operator::Icontains | Operator::NotIcontains => {
            let needle = values_as_strings(&f.value)?
                .into_iter()
                .next()
                .unwrap_or_default();
            let pattern = format!("%{}%", escape_like(&needle));
            if f.operator == Operator::NotIcontains {
                qb.push("(");
                expr(qb);
                qb.push(" IS NULL OR ");
                expr(qb);
                qb.push(" NOT ILIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            } else {
                expr(qb);
                qb.push(" ILIKE ");
                qb.push_bind(pattern);
            }
        }
        Operator::Regex | Operator::NotRegex => {
            let pattern = f
                .value
                .as_str()
                .ok_or_else(|| bad_request("regex operators need a string value"))?
                .to_string();
            if f.operator == Operator::NotRegex {
                qb.push("(");
                expr(qb);
                qb.push(" IS NULL OR ");
                expr(qb);
                qb.push(" !~ ");
                qb.push_bind(pattern);
                qb.push(")");
            } else {
                expr(qb);
                qb.push(" ~ ");
                qb.push_bind(pattern);
            }
        }
        Operator::Gt | Operator::Gte | Operator::Lt | Operator::Lte => {
            let op = match f.operator {
                Operator::Gt => " > ",
                Operator::Gte => " >= ",
                Operator::Lt => " < ",
                _ => " <= ",
            };
            push_numeric_expr(qb, &f.key, f.property_type, f.group_type_index);
            qb.push(op);
            qb.push_bind(value_as_f64(&f.value)?);
        }
        Operator::IsSet => {
            expr(qb);
            qb.push(" IS NOT NULL");
        }
        Operator::IsNotSet => {
            expr(qb);
            qb.push(" IS NULL");
        }
        Operator::IsDateBefore | Operator::IsDateAfter => {
            let op = if f.operator == Operator::IsDateBefore {
                " < "
            } else {
                " > "
            };
            qb.push("(CASE WHEN ");
            expr(qb);
            qb.push(" ~ '^\\d{4}-\\d{2}-\\d{2}' THEN (");
            expr(qb);
            qb.push(")::timestamptz END)");
            qb.push(op);
            qb.push_bind(value_as_datetime(&f.value)?);
        }
    }
    Ok(())
}

/// Push `AND <f1> AND <f2> …` for a flat filter list.
pub fn push_and_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    filters: &[PropertyFilter],
) -> AppResult<()> {
    for f in filters {
        qb.push(" AND ");
        push_filter(qb, f)?;
    }
    Ok(())
}

/// Push ` AND (<group>)` for a nested group (no-op when empty).
pub fn push_group(qb: &mut QueryBuilder<'_, Postgres>, group: &FilterGroup) -> AppResult<()> {
    if group.values.is_empty() {
        return Ok(());
    }
    qb.push(" AND ");
    push_group_inner(qb, group, 0)
}

fn push_group_inner(
    qb: &mut QueryBuilder<'_, Postgres>,
    group: &FilterGroup,
    depth: usize,
) -> AppResult<()> {
    if depth >= MAX_GROUP_DEPTH {
        return Err(bad_request(format!(
            "filter groups can be nested at most {MAX_GROUP_DEPTH} levels"
        )));
    }
    if group.values.is_empty() {
        qb.push("TRUE");
        return Ok(());
    }
    let joiner = match group.group_type {
        GroupType::And => " AND ",
        GroupType::Or => " OR ",
    };
    qb.push("(");
    for (i, node) in group.values.iter().enumerate() {
        if i > 0 {
            qb.push(joiner);
        }
        match node {
            FilterNode::Property(f) => push_filter(qb, f)?,
            FilterNode::Group(g) => push_group_inner(qb, g, depth + 1)?,
        }
    }
    qb.push(")");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sql_for(f: PropertyFilter) -> String {
        let mut qb = QueryBuilder::<Postgres>::new("SELECT 1 WHERE TRUE");
        push_and_filters(&mut qb, &[f]).unwrap();
        qb.sql().to_string()
    }

    fn filter(v: serde_json::Value) -> PropertyFilter {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn event_property_key_is_bound_not_interpolated() {
        let sql = sql_for(filter(
            json!({"key": "plan'; DROP TABLE x;--", "value": "pro"}),
        ));
        assert!(!sql.contains("DROP TABLE"));
        assert!(sql.contains("(ev.properties ->> $1) = ANY($2)"));
    }

    #[test]
    fn session_property_must_be_whitelisted() {
        let mut qb = QueryBuilder::<Postgres>::new("");
        let bad = filter(json!({"key": "id; --", "type": "session", "value": "x"}));
        assert!(push_filter(&mut qb, &bad).is_err());
        let ok = sql_for(filter(
            json!({"key": "country", "type": "session", "value": ["US", "DE"]}),
        ));
        assert!(ok.contains("(s.country)::text = ANY($1)"));
    }

    #[test]
    fn icontains_escapes_like_wildcards() {
        assert_eq!(escape_like("50%_off\\"), "50\\%\\_off\\\\");
        let sql = sql_for(filter(
            json!({"key": "path", "operator": "icontains", "value": "x"}),
        ));
        assert!(sql.contains("ILIKE $2"));
    }

    #[test]
    fn numeric_operator_casts_safely() {
        let sql = sql_for(filter(
            json!({"key": "amount", "operator": "gte", "value": 10}),
        ));
        assert!(sql.contains("::double precision END) >= $3"));
        let mut qb = QueryBuilder::<Postgres>::new("");
        let bad = filter(json!({"key": "amount", "operator": "gt", "value": "abc"}));
        assert!(push_filter(&mut qb, &bad).is_err());
    }

    #[test]
    fn nested_groups_compile_and_depth_is_limited() {
        let group: FilterGroup = serde_json::from_value(json!({
            "type": "OR",
            "values": [
                {"key": "plan", "value": "pro"},
                {"type": "AND", "values": [
                    {"key": "country", "type": "session", "value": "US"},
                    {"key": "email", "type": "person", "operator": "is_set"}
                ]}
            ]
        }))
        .unwrap();
        let mut qb = QueryBuilder::<Postgres>::new("WHERE TRUE");
        push_group(&mut qb, &group).unwrap();
        let sql = qb.sql().to_string();
        assert!(
            sql.contains(" OR ((s.country)::text = ANY($3) AND (up.traits ->> $4) IS NOT NULL))")
        );

        let mut deep = json!({"key": "a", "value": "b"});
        for _ in 0..6 {
            deep = json!({"type": "AND", "values": [deep]});
        }
        let deep: FilterGroup = serde_json::from_value(deep).unwrap();
        let mut qb = QueryBuilder::<Postgres>::new("");
        assert!(push_group(&mut qb, &deep).is_err());
    }

    #[test]
    fn cohort_filter_is_project_scoped() {
        let sql = sql_for(filter(json!({
            "key": "6f1c1c1e-0000-0000-0000-000000000001", "type": "cohort", "value": true
        })));
        assert!(sql.contains("EXISTS (SELECT 1 FROM cohort_people cp JOIN person_cohorts c"));
        assert!(sql.contains("c.project_id = ev.project_id"));
        let sql = sql_for(filter(json!({
            "key": "6f1c1c1e-0000-0000-0000-000000000001", "type": "cohort", "operator": "is_not"
        })));
        assert!(sql.contains("NOT EXISTS"));
        let mut qb = QueryBuilder::<Postgres>::new("");
        assert!(push_filter(&mut qb, &filter(json!({"key": "nope", "type": "cohort"}))).is_err());
        assert!(validate_property("x", PropertyType::Cohort, None).is_err());
    }

    #[test]
    fn joins_are_detected() {
        let group: FilterGroup = serde_json::from_value(json!({
            "values": [{"type": "OR", "values": [{"key": "plan", "type": "person", "value": "x"}]}]
        }))
        .unwrap();
        let mut j = Joins::default();
        j.add_group(&group);
        assert!(j.person && !j.session);
    }

    #[test]
    fn group_filters_join_by_index() {
        let f = filter(
            json!({"key": "industry", "type": "group", "group_type_index": 1, "value": "saas"}),
        );
        let sql = sql_for(f.clone());
        assert!(sql.contains("(g1.properties ->> $1) = ANY($2)"));
        let mut j = Joins::default();
        j.add_filters(&[f]);
        assert_eq!(j.groups, 0b10);
        assert!(DataNeeds::from(j).person);
        let mut qb = QueryBuilder::<Postgres>::new("");
        for bad in [
            json!({"key": "x", "type": "group", "value": "y"}),
            json!({"key": "x", "type": "group", "group_type_index": 5, "value": "y"}),
        ] {
            assert!(push_filter(&mut qb, &filter(bad)).is_err());
        }
    }
}
