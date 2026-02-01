use askama::Template;
use axum::extract::Query;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Extension;
use axum::Form;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::services::query as qsvc;
use crate::state::SharedState;

type HmacSha256 = Hmac<Sha256>;

const COOKIE_NAME: &str = "pulse_session";

// ── Template structs ──

#[derive(Template)]
#[template(path = "dashboard/login.html")]
struct LoginTemplate {
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard/overview.html")]
struct OverviewTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/pages.html")]
struct PagesTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/referrers.html")]
struct ReferrersTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/events.html")]
struct EventsTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/devices.html")]
struct DevicesTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/geo.html")]
struct GeoTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/realtime.html")]
struct RealtimeTemplate {
    project_name: String,
    active_page: String,
}

#[derive(Template)]
#[template(path = "partials/stats_cards.html")]
struct StatsCardsPartial {
    cards: Vec<StatCard>,
}

#[derive(Template)]
#[template(path = "partials/timeseries.html")]
struct TimeseriesPartial {
    chart_json: String,
}

struct StatCard {
    label: String,
    display_value: String,
    change: String,
    positive: bool,
}

// ── Helper types ──

#[derive(Deserialize)]
pub struct LoginForm {
    api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct DateParams {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

struct SessionInfo {
    project_id: Uuid,
    project_name: String,
}

fn sign_cookie_value(value: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid key");
    mac.update(value.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    format!("{value}.{sig}")
}

fn verify_cookie_value(signed: &str, secret: &str) -> Option<String> {
    let dot_pos = signed.rfind('.')?;
    let value = &signed[..dot_pos];
    let sig = &signed[dot_pos + 1..];

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid key");
    mac.update(value.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if sig == expected {
        Some(value.to_string())
    } else {
        None
    }
}

fn get_session(jar: &CookieJar, state: &SharedState) -> Option<SessionInfo> {
    let cookie = jar.get(COOKIE_NAME)?;
    let value = verify_cookie_value(cookie.value(), &state.config.cookie_secret)?;
    let parts: Vec<&str> = value.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    let project_id = Uuid::parse_str(parts[0]).ok()?;
    let project_name = parts[1].to_string();
    Some(SessionInfo {
        project_id,
        project_name,
    })
}

fn default_date_range() -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();
    let start = now - Duration::days(30);
    (start, now)
}

fn parse_dates(params: &DateParams) -> (DateTime<Utc>, DateTime<Utc>) {
    let (default_start, default_end) = default_date_range();
    (
        params.start_at.unwrap_or(default_start),
        params.end_at.unwrap_or(default_end),
    )
}

fn format_change(current: f64, prev: f64, invert: bool) -> (String, bool) {
    if prev == 0.0 {
        return ("".to_string(), true);
    }
    let pct = (current - prev) / prev * 100.0;
    let sign = if pct >= 0.0 { "+" } else { "" };
    let positive = if invert { pct <= 0.0 } else { pct >= 0.0 };
    (format!("{sign}{:.1}%", pct), positive)
}

fn render_template<T: Template>(tmpl: T) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template render error: {e}");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
        }
    }
}

// ── Full page routes ──

pub async fn dashboard_index(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
) -> Response {
    if get_session(&jar, &state).is_some() {
        Redirect::to("/dashboard/overview").into_response()
    } else {
        Redirect::to("/dashboard/login").into_response()
    }
}

pub async fn login_page() -> Response {
    render_template(LoginTemplate { error: None })
}

pub async fn login_submit(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Form(input): Form<LoginForm>,
) -> Response {
    let key = &input.api_key;
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let result: Option<(Uuid, String, Vec<String>)> = sqlx::query_as(
        "SELECT ak.project_id, p.name, ak.scopes \
         FROM api_keys ak JOIN projects p ON p.id = ak.project_id \
         WHERE ak.key_hash = $1 AND ak.is_active = true",
    )
    .bind(&key_hash)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    match result {
        Some((project_id, project_name, scopes)) if scopes.contains(&"query".to_string()) => {
            let value = format!("{project_id}:{project_name}");
            let signed = sign_cookie_value(&value, &state.config.cookie_secret);
            let cookie = Cookie::build((COOKIE_NAME, signed))
                .path("/dashboard")
                .http_only(true)
                .same_site(axum_extra::extract::cookie::SameSite::Lax)
                .build();

            let jar = jar.add(cookie);
            (jar, Redirect::to("/dashboard/overview")).into_response()
        }
        Some(_) => render_template(LoginTemplate {
            error: Some("API key does not have 'query' scope".to_string()),
        }),
        None => render_template(LoginTemplate {
            error: Some("Invalid API key".to_string()),
        }),
    }
}

pub async fn logout(jar: CookieJar) -> Response {
    let jar = jar.remove(Cookie::from(COOKIE_NAME));
    (jar, Redirect::to("/dashboard/login")).into_response()
}

pub async fn overview_page(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(OverviewTemplate {
        project_name: session.project_name,
        active_page: "overview".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn pages_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(PagesTemplate {
        project_name: session.project_name,
        active_page: "pages".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn referrers_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(ReferrersTemplate {
        project_name: session.project_name,
        active_page: "referrers".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn events_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(EventsTemplate {
        project_name: session.project_name,
        active_page: "events".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn devices_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(DevicesTemplate {
        project_name: session.project_name,
        active_page: "devices".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn geo_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(GeoTemplate {
        project_name: session.project_name,
        active_page: "geo".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn realtime_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    render_template(RealtimeTemplate {
        project_name: session.project_name,
        active_page: "realtime".to_string(),
    })
}

// ── HTMX partial endpoints ──

pub async fn htmx_stats_cards(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();
    let duration = end - start;
    let prev_start = start - duration;

    let current = match qsvc::fetch_stats(&state.db, session.project_id, start, end, today).await {
        Ok(s) => s,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };
    let previous = match qsvc::fetch_stats(&state.db, session.project_id, prev_start, start, today).await {
        Ok(s) => s,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let bounce_cur = if current.2 > 0 { current.3 as f64 / current.2 as f64 * 100.0 } else { 0.0 };
    let bounce_prev = if previous.2 > 0 { previous.3 as f64 / previous.2 as f64 * 100.0 } else { 0.0 };
    let dur_cur = if current.2 > 0 { current.4 as f64 / current.2 as f64 / 1000.0 } else { 0.0 };
    let dur_prev = if previous.2 > 0 { previous.4 as f64 / previous.2 as f64 / 1000.0 } else { 0.0 };

    let mk = |label: &str, cur: f64, prev: f64, display: String, invert: bool| {
        let (change, positive) = format_change(cur, prev, invert);
        StatCard { label: label.to_string(), display_value: display, change, positive }
    };

    let cards = vec![
        mk("Pageviews", current.0 as f64, previous.0 as f64, format_number(current.0), false),
        mk("Visitors", current.1 as f64, previous.1 as f64, format_number(current.1), false),
        mk("Bounce Rate", bounce_cur, bounce_prev, format!("{:.1}%", bounce_cur), true),
        mk("Avg Duration", dur_cur, dur_prev, format!("{:.1}s", dur_cur), false),
    ];

    render_template(StatsCardsPartial { cards })
}

pub async fn htmx_timeseries(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();

    let data = match qsvc::fetch_timeseries(&state.db, session.project_id, start, end, today).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let chart_json = serde_json::to_string(&data).unwrap_or_else(|_| "[]".to_string());
    render_template(TimeseriesPartial { chart_json })
}

pub async fn htmx_pages_table(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();
    let data = match qsvc::fetch_pages(&state.db, session.project_id, start, end, today, 50, 0).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };
    render_two_col_table("Page", "path", "Views", "views", &data)
}

pub async fn htmx_referrers_table(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();
    let data = match qsvc::fetch_referrers(&state.db, session.project_id, start, end, today, 50, 0).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };
    render_two_col_table("Referrer", "referrer_domain", "Visits", "visits", &data)
}

pub async fn htmx_events_table(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();
    let data = match qsvc::fetch_events(&state.db, session.project_id, start, end, today, 50, 0).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };
    render_two_col_table("Event", "event_name", "Count", "count", &data)
}

pub async fn htmx_devices_table(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();
    let data = match qsvc::fetch_devices(&state.db, session.project_id, start, end, today, 50, 0).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let mut html = String::from(r#"<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
        <table class="w-full text-sm">
            <thead><tr class="border-b border-gray-100 bg-gray-50">
                <th class="text-left px-4 py-3 font-medium text-gray-600">Browser</th>
                <th class="text-left px-4 py-3 font-medium text-gray-600">OS</th>
                <th class="text-left px-4 py-3 font-medium text-gray-600">Device</th>
                <th class="text-right px-4 py-3 font-medium text-gray-600">Visitors</th>
            </tr></thead><tbody>"#);

    for item in &data {
        let browser = item["browser"].as_str().unwrap_or("-");
        let os = item["os"].as_str().unwrap_or("-");
        let device = item["device"].as_str().unwrap_or("-");
        let visitors = item["visitors"].as_i64().unwrap_or(0);
        html.push_str(&format!(
            r#"<tr class="border-b border-gray-50 hover:bg-gray-50">
                <td class="px-4 py-2.5 text-gray-900">{browser}</td>
                <td class="px-4 py-2.5 text-gray-700">{os}</td>
                <td class="px-4 py-2.5 text-gray-700">{device}</td>
                <td class="px-4 py-2.5 text-right text-gray-900 font-medium">{visitors}</td>
            </tr>"#
        ));
    }

    if data.is_empty() {
        html.push_str(r#"<tr><td colspan="4" class="px-4 py-8 text-center text-gray-400">No data for this period</td></tr>"#);
    }

    html.push_str("</tbody></table></div>");
    Html(html).into_response()
}

pub async fn htmx_geo_table(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();
    let data = match qsvc::fetch_geo(&state.db, session.project_id, start, end, today, 50, 0).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };
    render_two_col_table("Country", "country", "Visitors", "visitors", &data)
}

pub async fn htmx_realtime(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };

    let active = match qsvc::fetch_realtime(&state, session.project_id).await {
        Ok(n) => n,
        Err(_) => 0,
    };

    let dot_color = if active > 0 { "bg-emerald-400" } else { "bg-gray-400" };
    let text_color = if active > 0 { "text-emerald-600" } else { "text-gray-500" };

    let html = format!(
        r#"<div class="bg-white rounded-xl border border-gray-200 p-12 text-center">
            <div class="inline-flex items-center gap-3 mb-2">
                <span class="w-3 h-3 rounded-full {dot_color} animate-pulse"></span>
                <span class="text-5xl font-bold text-gray-900">{active}</span>
            </div>
            <p class="text-sm {text_color} font-medium">active visitor{}</p>
            <p class="text-xs text-gray-400 mt-2">Auto-refreshes every 5 seconds</p>
        </div>"#,
        if active == 1 { "" } else { "s" }
    );

    Html(html).into_response()
}

// ── Helpers ──

fn format_number(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn render_two_col_table(
    label_header: &str,
    label_key: &str,
    value_header: &str,
    value_key: &str,
    data: &[serde_json::Value],
) -> Response {
    let mut html = format!(
        r#"<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
        <table class="w-full text-sm">
            <thead><tr class="border-b border-gray-100 bg-gray-50">
                <th class="text-left px-4 py-3 font-medium text-gray-600">{label_header}</th>
                <th class="text-right px-4 py-3 font-medium text-gray-600">{value_header}</th>
            </tr></thead><tbody>"#
    );

    for item in data {
        let label = item[label_key].as_str().unwrap_or("-");
        let value = item[value_key].as_i64().unwrap_or(0);
        html.push_str(&format!(
            r#"<tr class="border-b border-gray-50 hover:bg-gray-50">
                <td class="px-4 py-2.5 text-gray-900 font-mono text-xs">{label}</td>
                <td class="px-4 py-2.5 text-right text-gray-900 font-medium">{value}</td>
            </tr>"#
        ));
    }

    if data.is_empty() {
        html.push_str(&format!(
            r#"<tr><td colspan="2" class="px-4 py-8 text-center text-gray-400">No data for this period</td></tr>"#
        ));
    }

    html.push_str("</tbody></table></div>");
    Html(html).into_response()
}
