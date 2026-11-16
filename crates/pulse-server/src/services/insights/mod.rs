//! General insights query engine (PostHog-style `TrendsQuery`).
//!
//! Every pageview and custom event is exposed as one virtual event stream:
//!
//! | column       | pageviews                       | events                          |
//! |--------------|---------------------------------|---------------------------------|
//! | `event`      | `'$pageview'`                   | `event_name`                    |
//! | `properties` | path/title/referrer/utm_* JSON  | `event_data` + path/revenue     |
//!
//! Session properties come from `sessions` (joined on `session_id`) and person
//! properties from `user_profiles.traits` (joined on `visitor_id`). All user
//! input is bound as parameters; only whitelisted identifiers are interpolated.

pub mod filters;
pub mod formula;
pub mod trends;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Name of the virtual event that pageviews are exposed as.
pub const PAGEVIEW_EVENT: &str = "$pageview";
/// Label of the bucket that collects breakdown values outside the top N.
pub const BREAKDOWN_OTHER: &str = "$$_other";
/// Upper bound on buckets in one query (e.g. hourly over 7 months).
pub const MAX_BUCKETS: i64 = 5_000;
/// Upper bound on series per query.
pub const MAX_SERIES: usize = 26;
/// Default and maximum breakdown values returned (plus the "other" bucket).
pub const DEFAULT_BREAKDOWN_LIMIT: i64 = 25;
pub const MAX_BREAKDOWN_LIMIT: i64 = 500;

/// Where a property lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    /// `properties` of the event itself.
    #[default]
    Event,
    /// A column of the event's session (browser, country, entry_page, …).
    Session,
    /// A trait of the visitor's profile (set via identify).
    Person,
}

/// Session columns that can be filtered or broken down on.
pub const SESSION_PROPERTIES: &[&str] = &[
    "browser",
    "os",
    "device",
    "screen",
    "language",
    "country",
    "region",
    "city",
    "hostname",
    "entry_page",
    "exit_page",
    "is_bounce",
    "duration_ms",
    "pageview_count",
    "event_count",
];

pub(crate) fn bad_request(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}

/// Turn Postgres errors caused by user input (bad regex, bad timezone, bad
/// cast) into 400s instead of opaque 500s.
pub(crate) fn map_query_error(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &e {
        if let Some(code) = db.code() {
            // 2201B invalid_regular_expression, 22023 invalid_parameter_value
            // (unknown time zone), 22P02 invalid_text_representation,
            // 22007/22008 datetime format/overflow.
            if matches!(
                code.as_ref(),
                "2201B" | "22023" | "22P02" | "22007" | "22008"
            ) {
                return bad_request(format!("Invalid query input: {}", db.message()));
            }
        }
    }
    AppError::Database(e)
}
