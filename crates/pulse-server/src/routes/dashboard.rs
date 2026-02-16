use askama::Template;
use axum::extract::{Path, Query};
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
#[template(path = "dashboard/visitors.html")]
struct VisitorsTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/visitor_detail.html")]
struct VisitorDetailTemplate {
    project_name: String,
    active_page: String,
    visitor_id: String,
    range: String,
    start_at: String,
    end_at: String,
}

#[derive(Template)]
#[template(path = "dashboard/pricing.html")]
struct PricingTemplate {
    project_name: String,
    active_page: String,
    range: String,
    start_at: String,
    end_at: String,
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

#[derive(Debug, Deserialize)]
pub struct VisitorTableParams {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FunnelParams {
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub steps: Option<String>,
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

// ── Visitors page routes ──

pub async fn visitors_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(VisitorsTemplate {
        project_name: session.project_name,
        active_page: "visitors".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn visitor_detail_page(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Path(visitor_id): Path<String>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(VisitorDetailTemplate {
        project_name: session.project_name,
        active_page: "visitors".to_string(),
        visitor_id,
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

pub async fn pricing_page(Extension(state): Extension<SharedState>, jar: CookieJar) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Redirect::to("/dashboard/login").into_response();
    };
    let (start, end) = default_date_range();
    render_template(PricingTemplate {
        project_name: session.project_name,
        active_page: "pricing".to_string(),
        range: "30d".to_string(),
        start_at: start.to_rfc3339(),
        end_at: end.to_rfc3339(),
    })
}

// ── Visitors HTMX partials ──

pub async fn htmx_visitors_live_count(
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
    let dot_color = if active > 0 { "bg-emerald-400" } else { "bg-gray-300" };
    let html = format!(
        r#"<div class="flex items-center gap-2 bg-white border border-gray-200 rounded-full px-4 py-1.5">
            <span class="w-2 h-2 rounded-full {dot_color} animate-pulse"></span>
            <span class="text-sm font-semibold text-gray-700">{active}</span>
            <span class="text-xs text-gray-500">online now</span>
        </div>"#
    );
    Html(html).into_response()
}

pub async fn htmx_visitors_activity_feed(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let data = match qsvc::fetch_recent_activity(&state.db, session.project_id, 20).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    if data.is_empty() {
        return Html(r#"<div class="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">No recent activity</div>"#.to_string()).into_response();
    }

    let mut html = String::from(
        r#"<div class="bg-white rounded-xl border border-gray-200 divide-y divide-gray-50 max-h-64 overflow-y-auto">"#,
    );

    for item in &data {
        let activity_type = item["activity_type"].as_str().unwrap_or("pageview");
        let visitor = item["visitor_id"].as_str().unwrap_or("-");
        let visitor_short = &visitor[..visitor.len().min(12)];
        let detail = item["detail"].as_str().unwrap_or("-");
        let event_name = item["event_name"].as_str();
        let created = item["created_at"].as_str().unwrap_or("");

        let (icon, icon_color, label) = if activity_type == "event" {
            let name = event_name.unwrap_or("event");
            (
                r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>"#,
                "text-violet-500",
                format!("<span class=\"font-medium text-violet-600\">{name}</span> on {detail}"),
            )
        } else {
            (
                r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>"#,
                "text-indigo-500",
                format!("viewed <span class=\"font-medium text-gray-700\">{detail}</span>"),
            )
        };

        html.push_str(&format!(
            r#"<div class="flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors">
                <svg class="w-4 h-4 {icon_color} shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">{icon}</svg>
                <a href="/dashboard/visitors/{visitor}" class="text-xs font-mono text-indigo-600 hover:underline shrink-0">{visitor_short}</a>
                <span class="text-xs text-gray-500 truncate">{label}</span>
                <span class="text-[10px] text-gray-400 ml-auto shrink-0 tabular-nums">{created}</span>
            </div>"#
        ));
    }

    html.push_str("</div>");
    Html(html).into_response()
}

pub async fn htmx_visitors_table(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<VisitorTableParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (default_start, default_end) = default_date_range();
    let start = params.start_at.unwrap_or(default_start);
    let end = params.end_at.unwrap_or(default_end);
    let search = params.search.as_deref().filter(|s| !s.is_empty());

    let data = match qsvc::fetch_visitors_list(&state.db, session.project_id, start, end, 50, 0, search).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let mut html = String::from(
        r#"<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
        <table class="w-full text-sm">
            <thead><tr class="border-b border-gray-100 bg-gray-50">
                <th class="text-left px-4 py-3 font-medium text-gray-600">Visitor</th>
                <th class="text-right px-4 py-3 font-medium text-gray-600">Sessions</th>
                <th class="text-right px-4 py-3 font-medium text-gray-600">Pageviews</th>
                <th class="text-right px-4 py-3 font-medium text-gray-600">Events</th>
                <th class="text-left px-4 py-3 font-medium text-gray-600">Country</th>
                <th class="text-left px-4 py-3 font-medium text-gray-600">Device</th>
                <th class="text-right px-4 py-3 font-medium text-gray-600">Last Seen</th>
            </tr></thead><tbody>"#,
    );

    for item in &data {
        let vid = item["visitor_id"].as_str().unwrap_or("-");
        let vid_short = &vid[..vid.len().min(12)];
        let sessions = item["session_count"].as_i64().unwrap_or(0);
        let pvs = item["total_pageviews"].as_i64().unwrap_or(0);
        let evts = item["total_events"].as_i64().unwrap_or(0);
        let country = item["country"].as_str().unwrap_or("-");
        let device = item["device"].as_str().unwrap_or("-");
        let last_seen = item["last_seen"].as_str().unwrap_or("-");
        let last_short = &last_seen[..last_seen.len().min(10)];

        html.push_str(&format!(
            r#"<tr class="border-b border-gray-50 hover:bg-indigo-50/50 cursor-pointer transition-colors" onclick="window.location='/dashboard/visitors/{vid}'">
                <td class="px-4 py-2.5">
                    <span class="font-mono text-xs text-indigo-600">{vid_short}...</span>
                </td>
                <td class="px-4 py-2.5 text-right text-gray-900 tabular-nums">{sessions}</td>
                <td class="px-4 py-2.5 text-right text-gray-900 tabular-nums">{pvs}</td>
                <td class="px-4 py-2.5 text-right text-gray-700 tabular-nums">{evts}</td>
                <td class="px-4 py-2.5 text-gray-700">{country}</td>
                <td class="px-4 py-2.5 text-gray-700 capitalize">{device}</td>
                <td class="px-4 py-2.5 text-right text-gray-500 text-xs tabular-nums">{last_short}</td>
            </tr>"#
        ));
    }

    if data.is_empty() {
        html.push_str(r#"<tr><td colspan="7" class="px-4 py-8 text-center text-gray-400">No visitors found</td></tr>"#);
    }

    html.push_str("</tbody></table></div>");
    Html(html).into_response()
}

// ── Visitor detail HTMX partials ──

pub async fn htmx_visitor_summary(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Path(visitor_id): Path<String>,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);

    let data = match qsvc::fetch_visitor_summary(&state.db, session.project_id, &visitor_id, start, end).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let sessions = data["session_count"].as_i64().unwrap_or(0);
    let pvs = data["total_pageviews"].as_i64().unwrap_or(0);
    let evts = data["total_events"].as_i64().unwrap_or(0);
    let dur_ms = data["total_duration_ms"].as_i64().unwrap_or(0);
    let pricing = data["pricing_views"].as_i64().unwrap_or(0);
    let dur_display = if dur_ms > 60_000 {
        format!("{:.1}m", dur_ms as f64 / 60_000.0)
    } else {
        format!("{:.1}s", dur_ms as f64 / 1000.0)
    };

    let country = data["country"].as_str().unwrap_or("-");
    let browser = data["browser"].as_str().unwrap_or("-");
    let os = data["os"].as_str().unwrap_or("-");
    let device = data["device"].as_str().unwrap_or("-");

    let html = format!(
        r#"<div class="grid grid-cols-2 lg:grid-cols-5 gap-4 mb-2">
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Sessions</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{sessions}</p>
            </div>
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Pageviews</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{pvs}</p>
            </div>
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Events</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{evts}</p>
            </div>
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Total Time</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{dur_display}</p>
            </div>
            <div class="bg-gradient-to-br from-indigo-50 to-violet-50 rounded-xl border border-indigo-200 p-4">
                <p class="text-[10px] font-semibold text-indigo-500 uppercase tracking-wide">Pricing Views</p>
                <p class="text-2xl font-bold text-indigo-700 mt-1">{pricing}</p>
            </div>
        </div>
        <div class="flex items-center gap-4 text-xs text-gray-500">
            <span>{country}</span>
            <span class="text-gray-300">|</span>
            <span>{browser} / {os}</span>
            <span class="text-gray-300">|</span>
            <span class="capitalize">{device}</span>
        </div>"#
    );
    Html(html).into_response()
}

pub async fn htmx_visitor_sessions(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Path(visitor_id): Path<String>,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);

    let data = match qsvc::fetch_visitor_sessions(&state.db, session.project_id, &visitor_id, start, end).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    if data.is_empty() {
        return Html(r#"<div class="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">No sessions found</div>"#.to_string()).into_response();
    }

    let mut html = String::from(r#"<div class="space-y-3">"#);

    for item in &data {
        let sid = item["id"].as_str().unwrap_or("-");
        let first_at = item["first_at"].as_str().unwrap_or("-");
        let first_short = &first_at[..first_at.len().min(16)];
        let pv_count = item["pageview_count"].as_i64().unwrap_or(0);
        let ev_count = item["event_count"].as_i64().unwrap_or(0);
        let dur_ms = item["duration_ms"].as_i64().unwrap_or(0);
        let dur_s = dur_ms as f64 / 1000.0;
        let is_bounce = item["is_bounce"].as_bool().unwrap_or(false);
        let entry = item["entry_page"].as_str().unwrap_or("-");
        let exit = item["exit_page"].as_str().unwrap_or("-");
        let browser = item["browser"].as_str().unwrap_or("-");
        let country = item["country"].as_str().unwrap_or("-");
        let bounce_badge = if is_bounce {
            r#"<span class="px-1.5 py-0.5 text-[10px] font-medium bg-amber-100 text-amber-700 rounded">Bounce</span>"#
        } else {
            ""
        };

        html.push_str(&format!(
            r#"<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
                <div class="px-4 py-3 flex items-center justify-between cursor-pointer hover:bg-gray-50 transition-colors"
                     hx-get="/dashboard/api/visitor/{visitor_id}/session/{sid}/detail"
                     hx-target="#session-detail-{sid}"
                     hx-swap="innerHTML"
                     hx-trigger="click once">
                    <div class="flex items-center gap-4">
                        <div class="flex items-center gap-2">
                            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                            <span class="text-sm font-medium text-gray-900">{first_short}</span>
                        </div>
                        <span class="text-xs text-gray-500">{pv_count} pages</span>
                        <span class="text-xs text-gray-500">{ev_count} events</span>
                        <span class="text-xs text-gray-500">{dur_s:.0}s</span>
                        {bounce_badge}
                    </div>
                    <div class="flex items-center gap-3 text-xs text-gray-400">
                        <span>{browser}</span>
                        <span>{country}</span>
                        <svg class="w-4 h-4 transition-transform" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>
                    </div>
                </div>
                <div class="text-xs text-gray-400 px-4 pb-2 flex gap-2">
                    <span>{entry}</span>
                    <span>→</span>
                    <span>{exit}</span>
                </div>
                <div id="session-detail-{sid}"></div>
            </div>"#
        ));
    }

    html.push_str("</div>");
    Html(html).into_response()
}

pub async fn htmx_visitor_session_detail(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Path((visitor_id, session_id)): Path<(String, String)>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let Ok(sid) = Uuid::parse_str(&session_id) else {
        return Html("Invalid session ID".to_string()).into_response();
    };
    let _ = &visitor_id; // used for route matching

    let (pageviews, events) =
        match qsvc::fetch_session_detail(&state.db, session.project_id, sid).await {
            Ok(d) => d,
            Err(e) => return Html(format!("Error: {e}")).into_response(),
        };

    let mut html = String::from(
        r#"<div class="border-t border-gray-100 px-4 py-3 bg-gray-50/50">"#,
    );

    if pageviews.is_empty() && events.is_empty() {
        html.push_str(r#"<p class="text-xs text-gray-400 text-center py-2">No details available</p>"#);
    } else {
        html.push_str(r#"<div class="space-y-1.5">"#);

        for pv in &pageviews {
            let path = pv["path"].as_str().unwrap_or("-");
            let title = pv["title"].as_str().unwrap_or("");
            let dur = pv["duration_ms"].as_i64().unwrap_or(0);
            let created = pv["created_at"].as_str().unwrap_or("");
            let time_part = &created[created.len().saturating_sub(14)..created.len().saturating_sub(5)];
            let dur_s = dur as f64 / 1000.0;

            html.push_str(&format!(
                r#"<div class="flex items-center gap-3 text-xs">
                    <span class="text-gray-400 tabular-nums w-16 shrink-0">{time_part}</span>
                    <span class="w-1.5 h-1.5 rounded-full bg-indigo-400 shrink-0"></span>
                    <span class="text-gray-700 font-mono truncate">{path}</span>
                    <span class="text-gray-400 truncate hidden lg:inline">{title}</span>
                    <span class="text-gray-400 ml-auto shrink-0">{dur_s:.0}s</span>
                </div>"#
            ));
        }

        for ev in &events {
            let name = ev["event_name"].as_str().unwrap_or("-");
            let path = ev["path"].as_str().unwrap_or("");
            let created = ev["created_at"].as_str().unwrap_or("");
            let time_part = &created[created.len().saturating_sub(14)..created.len().saturating_sub(5)];

            html.push_str(&format!(
                r#"<div class="flex items-center gap-3 text-xs">
                    <span class="text-gray-400 tabular-nums w-16 shrink-0">{time_part}</span>
                    <span class="w-1.5 h-1.5 rounded-full bg-violet-400 shrink-0"></span>
                    <span class="text-violet-600 font-medium">{name}</span>
                    <span class="text-gray-400 font-mono truncate">{path}</span>
                </div>"#
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</div>");
    Html(html).into_response()
}

pub async fn htmx_visitor_activity_chart(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Path(visitor_id): Path<String>,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);

    let data = match qsvc::fetch_visitor_daily_activity(&state.db, session.project_id, &visitor_id, start, end).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let chart_json = serde_json::to_string(&data).unwrap_or_else(|_| "[]".to_string());

    let html = format!(
        r#"<div class="bg-white rounded-xl border border-gray-200 p-6">
            <h3 class="text-sm font-medium text-gray-700 mb-4">Daily Activity</h3>
            <canvas id="visitorActivityChart" height="120"></canvas>
            <div id="visitor-activity-data" class="hidden" data-chart='{chart_json}'></div>
        </div>
        <script>
        (function() {{
            const dataEl = document.getElementById('visitor-activity-data');
            if (!dataEl) return;
            const data = JSON.parse(dataEl.getAttribute('data-chart'));
            const ctx = document.getElementById('visitorActivityChart');
            if (!ctx) return;
            if (window._vaChart) window._vaChart.destroy();
            window._vaChart = new Chart(ctx, {{
                type: 'bar',
                data: {{
                    labels: data.map(d => d.date),
                    datasets: [{{
                        label: 'Pageviews',
                        data: data.map(d => d.pageviews),
                        backgroundColor: function(context) {{
                            const chart = context.chart;
                            const {{ctx: c, chartArea}} = chart;
                            if (!chartArea) return 'rgba(99,102,241,0.7)';
                            const gradient = c.createLinearGradient(0, chartArea.bottom, 0, chartArea.top);
                            gradient.addColorStop(0, 'rgba(99,102,241,0.4)');
                            gradient.addColorStop(1, 'rgba(139,92,246,0.9)');
                            return gradient;
                        }},
                        borderRadius: 6,
                        borderSkipped: false,
                        barPercentage: 0.7,
                    }}]
                }},
                options: {{
                    responsive: true,
                    plugins: {{
                        legend: {{ display: false }},
                        tooltip: {{
                            backgroundColor: '#1f2937',
                            titleFont: {{ size: 12 }},
                            bodyFont: {{ size: 11 }},
                            padding: 10,
                            cornerRadius: 8,
                            displayColors: false,
                        }}
                    }},
                    scales: {{
                        x: {{ grid: {{ display: false }}, ticks: {{ font: {{ size: 10 }}, maxRotation: 0 }} }},
                        y: {{ beginAtZero: true, grid: {{ color: '#f3f4f6' }}, ticks: {{ font: {{ size: 10 }}, stepSize: 1 }} }}
                    }},
                    animation: {{ duration: 800, easing: 'easeOutQuart' }}
                }}
            }});
        }})();
        </script>"#
    );
    Html(html).into_response()
}

pub async fn htmx_visitor_events_breakdown(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Path(visitor_id): Path<String>,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);

    let data = match qsvc::fetch_visitor_event_breakdown(&state.db, session.project_id, &visitor_id, start, end).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    if data.is_empty() {
        return Html(
            r#"<div class="bg-white rounded-xl border border-gray-200 p-6">
                <h3 class="text-sm font-medium text-gray-700 mb-4">Event Breakdown</h3>
                <p class="text-sm text-gray-400 text-center py-8">No events recorded</p>
            </div>"#.to_string(),
        ).into_response();
    }

    let chart_json = serde_json::to_string(&data).unwrap_or_else(|_| "[]".to_string());

    let html = format!(
        r#"<div class="bg-white rounded-xl border border-gray-200 p-6">
            <h3 class="text-sm font-medium text-gray-700 mb-4">Event Breakdown</h3>
            <canvas id="visitorEventsChart" height="120"></canvas>
            <div id="visitor-events-data" class="hidden" data-chart='{chart_json}'></div>
        </div>
        <script>
        (function() {{
            const dataEl = document.getElementById('visitor-events-data');
            if (!dataEl) return;
            const data = JSON.parse(dataEl.getAttribute('data-chart'));
            const ctx = document.getElementById('visitorEventsChart');
            if (!ctx) return;
            if (window._veChart) window._veChart.destroy();
            const colors = ['#6366f1','#8b5cf6','#a78bfa','#c4b5fd','#ddd6fe','#818cf8','#7c3aed','#6d28d9','#5b21b6','#4c1d95'];
            window._veChart = new Chart(ctx, {{
                type: 'doughnut',
                data: {{
                    labels: data.map(d => d.event_name),
                    datasets: [{{
                        data: data.map(d => d.count),
                        backgroundColor: colors.slice(0, data.length),
                        borderWidth: 0,
                        hoverOffset: 8,
                    }}]
                }},
                options: {{
                    responsive: true,
                    cutout: '65%',
                    plugins: {{
                        legend: {{
                            position: 'right',
                            labels: {{ usePointStyle: true, pointStyle: 'circle', boxWidth: 8, padding: 12, font: {{ size: 11 }} }}
                        }},
                        tooltip: {{ backgroundColor: '#1f2937', padding: 10, cornerRadius: 8 }}
                    }},
                    animation: {{ animateRotate: true, duration: 1000 }}
                }}
            }});
        }})();
        </script>"#
    );
    Html(html).into_response()
}

// ── Pricing HTMX partials ──

pub async fn htmx_pricing_stats(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();

    let (views, visitors, avg_dur_ms, bounce_rate) =
        match qsvc::fetch_pricing_stats(&state.db, session.project_id, start, end, today).await {
            Ok(s) => s,
            Err(e) => return Html(format!("Error: {e}")).into_response(),
        };

    let avg_dur_s = avg_dur_ms / 1000.0;

    let html = format!(
        r#"<div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Pricing Views</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{}</p>
            </div>
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Unique Visitors</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{}</p>
            </div>
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Avg. Time on Page</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{avg_dur_s:.1}s</p>
            </div>
            <div class="bg-white rounded-xl border border-gray-200 p-4">
                <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-wide">Bounce Rate</p>
                <p class="text-2xl font-bold text-gray-900 mt-1">{bounce_rate:.1}%</p>
            </div>
        </div>"#,
        format_number(views),
        format_number(visitors),
    );
    Html(html).into_response()
}

pub async fn htmx_pricing_timeseries(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);
    let today = Utc::now().date_naive();

    let data = match qsvc::fetch_pricing_timeseries(&state.db, session.project_id, start, end, today).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let chart_json = serde_json::to_string(&data).unwrap_or_else(|_| "[]".to_string());

    let html = format!(
        r#"<div class="bg-white rounded-xl border border-gray-200 p-6">
            <h3 class="text-sm font-medium text-gray-700 mb-4">Pricing Page Traffic</h3>
            <canvas id="pricingTimeseriesChart" height="80"></canvas>
            <div id="pricing-ts-data" class="hidden" data-chart='{chart_json}'></div>
        </div>
        <script>
        (function() {{
            const dataEl = document.getElementById('pricing-ts-data');
            if (!dataEl) return;
            const data = JSON.parse(dataEl.getAttribute('data-chart'));
            const ctx = document.getElementById('pricingTimeseriesChart');
            if (!ctx) return;
            if (window._ptChart) window._ptChart.destroy();
            window._ptChart = new Chart(ctx, {{
                type: 'line',
                data: {{
                    labels: data.map(d => d.date),
                    datasets: [{{
                        label: 'Pricing Views',
                        data: data.map(d => d.views),
                        borderColor: '#6366f1',
                        borderWidth: 2.5,
                        backgroundColor: function(context) {{
                            const chart = context.chart;
                            const {{ctx: c, chartArea}} = chart;
                            if (!chartArea) return 'rgba(99,102,241,0.1)';
                            const gradient = c.createLinearGradient(0, chartArea.bottom, 0, chartArea.top);
                            gradient.addColorStop(0, 'rgba(99,102,241,0.02)');
                            gradient.addColorStop(0.5, 'rgba(99,102,241,0.08)');
                            gradient.addColorStop(1, 'rgba(99,102,241,0.25)');
                            return gradient;
                        }},
                        fill: true,
                        tension: 0.4,
                        pointRadius: 0,
                        pointHoverRadius: 6,
                        pointHoverBackgroundColor: '#6366f1',
                        pointHoverBorderColor: '#fff',
                        pointHoverBorderWidth: 2,
                    }}, {{
                        label: 'Unique Visitors',
                        data: data.map(d => d.unique_views),
                        borderColor: '#8b5cf6',
                        borderWidth: 2,
                        borderDash: [5, 5],
                        backgroundColor: 'transparent',
                        fill: false,
                        tension: 0.4,
                        pointRadius: 0,
                        pointHoverRadius: 5,
                    }}]
                }},
                options: {{
                    responsive: true,
                    interaction: {{ intersect: false, mode: 'index' }},
                    plugins: {{
                        legend: {{ position: 'bottom', labels: {{ usePointStyle: true, boxWidth: 6, padding: 16 }} }},
                        tooltip: {{
                            backgroundColor: '#1f2937',
                            titleFont: {{ size: 12, weight: '600' }},
                            bodyFont: {{ size: 11 }},
                            padding: 12,
                            cornerRadius: 8,
                            displayColors: true,
                            usePointStyle: true,
                        }}
                    }},
                    scales: {{
                        x: {{ grid: {{ display: false }}, ticks: {{ font: {{ size: 11 }}, maxRotation: 0 }} }},
                        y: {{ beginAtZero: true, grid: {{ color: '#f9fafb' }}, ticks: {{ font: {{ size: 11 }} }} }}
                    }},
                    animation: {{ duration: 1000, easing: 'easeOutCubic' }}
                }}
            }});
        }})();
        </script>"#
    );
    Html(html).into_response()
}

pub async fn htmx_pricing_frequency(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);

    let data = match qsvc::fetch_pricing_frequency(&state.db, session.project_id, start, end).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let chart_json = serde_json::to_string(&data).unwrap_or_else(|_| "[]".to_string());

    let html = format!(
        r#"<div class="bg-white rounded-xl border border-gray-200 p-6">
            <h3 class="text-sm font-medium text-gray-700 mb-4">Visit Frequency</h3>
            <canvas id="pricingFreqChart" height="120"></canvas>
            <div id="pricing-freq-data" class="hidden" data-chart='{chart_json}'></div>
        </div>
        <script>
        (function() {{
            const dataEl = document.getElementById('pricing-freq-data');
            if (!dataEl) return;
            const data = JSON.parse(dataEl.getAttribute('data-chart'));
            const ctx = document.getElementById('pricingFreqChart');
            if (!ctx) return;
            if (window._pfChart) window._pfChart.destroy();
            const labels = data.map(d => d.visits + 'x');
            const values = data.map(d => d.visitor_count);
            const maxVal = Math.max(...values, 1);
            window._pfChart = new Chart(ctx, {{
                type: 'bar',
                data: {{
                    labels: labels,
                    datasets: [{{
                        data: values,
                        backgroundColor: values.map(v => {{
                            const intensity = 0.3 + (v / maxVal) * 0.7;
                            return 'rgba(99,102,241,' + intensity + ')';
                        }}),
                        borderRadius: 4,
                        borderSkipped: false,
                        barPercentage: 0.6,
                    }}]
                }},
                options: {{
                    indexAxis: 'y',
                    responsive: true,
                    plugins: {{
                        legend: {{ display: false }},
                        tooltip: {{ backgroundColor: '#1f2937', cornerRadius: 8, padding: 10 }}
                    }},
                    scales: {{
                        x: {{ beginAtZero: true, grid: {{ color: '#f3f4f6' }}, ticks: {{ font: {{ size: 11 }} }},
                             title: {{ display: true, text: 'Number of visitors', font: {{ size: 11, weight: '500' }}, color: '#6b7280' }} }},
                        y: {{ grid: {{ display: false }}, ticks: {{ font: {{ size: 11, weight: '500' }} }},
                             title: {{ display: true, text: 'Visit frequency', font: {{ size: 11, weight: '500' }}, color: '#6b7280' }} }}
                    }},
                    animation: {{ duration: 800, easing: 'easeOutQuart' }}
                }}
            }});
        }})();
        </script>"#
    );
    Html(html).into_response()
}

pub async fn htmx_pricing_referrers(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);

    let data = match qsvc::fetch_pricing_referrers(&state.db, session.project_id, start, end, 20).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let mut html = String::from(
        r#"<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
            <h3 class="text-sm font-medium text-gray-700 px-4 pt-4 pb-2">Top Referrers to Pricing</h3>
            <table class="w-full text-sm">
                <thead><tr class="border-b border-gray-100 bg-gray-50">
                    <th class="text-left px-4 py-2 font-medium text-gray-600">Referrer</th>
                    <th class="text-right px-4 py-2 font-medium text-gray-600">Visits</th>
                </tr></thead><tbody>"#,
    );

    for item in &data {
        let domain = item["referrer_domain"].as_str().unwrap_or("-");
        let visits = item["visits"].as_i64().unwrap_or(0);
        html.push_str(&format!(
            r#"<tr class="border-b border-gray-50 hover:bg-gray-50">
                <td class="px-4 py-2 text-gray-900 font-mono text-xs">{domain}</td>
                <td class="px-4 py-2 text-right text-gray-900 font-medium tabular-nums">{visits}</td>
            </tr>"#
        ));
    }

    if data.is_empty() {
        html.push_str(r#"<tr><td colspan="2" class="px-4 py-6 text-center text-gray-400">No data</td></tr>"#);
    }

    html.push_str("</tbody></table></div>");
    Html(html).into_response()
}

pub async fn htmx_pricing_heatmap(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<DateParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (start, end) = parse_dates(&params);

    let data = match qsvc::fetch_pricing_heatmap(&state.db, session.project_id, start, end).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    // Build a 7x24 grid
    let mut grid = [[0i64; 24]; 7];
    let mut max_val = 1i64;
    for item in &data {
        let dow = item["day_of_week"].as_i64().unwrap_or(0) as usize;
        let hour = item["hour_of_day"].as_i64().unwrap_or(0) as usize;
        let views = item["views"].as_i64().unwrap_or(0);
        if dow < 7 && hour < 24 {
            grid[dow][hour] = views;
            if views > max_val {
                max_val = views;
            }
        }
    }

    let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    let mut html = String::from(
        r#"<div class="bg-white rounded-xl border border-gray-200 p-6">
            <h3 class="text-sm font-medium text-gray-700 mb-4">Activity by Time</h3>
            <div class="overflow-x-auto">
                <div class="inline-grid gap-1" style="grid-template-columns: 40px repeat(24, 24px);">
                    <div></div>"#,
    );

    // Header row: hours
    for h in 0..24 {
        html.push_str(&format!(
            r#"<div class="text-[9px] text-gray-400 text-center">{h}</div>"#
        ));
    }

    // Data rows
    for dow in 0..7 {
        html.push_str(&format!(
            r#"<div class="text-xs text-gray-500 leading-6 pr-1 text-right">{}</div>"#,
            day_names[dow]
        ));
        for hour in 0..24 {
            let val = grid[dow][hour];
            let opacity = if val == 0 {
                0.05
            } else {
                0.15 + (val as f64 / max_val as f64) * 0.85
            };
            html.push_str(&format!(
                r#"<div class="w-6 h-6 rounded-sm cursor-default" style="background-color: rgba(99,102,241,{opacity:.2});" title="{val} views"></div>"#
            ));
        }
    }

    html.push_str(r#"</div></div>
        <div class="flex items-center justify-end gap-2 mt-3 text-[10px] text-gray-400">
            <span>Less</span>
            <div class="flex gap-0.5">
                <div class="w-3 h-3 rounded-sm" style="background-color: rgba(99,102,241,0.1);"></div>
                <div class="w-3 h-3 rounded-sm" style="background-color: rgba(99,102,241,0.3);"></div>
                <div class="w-3 h-3 rounded-sm" style="background-color: rgba(99,102,241,0.5);"></div>
                <div class="w-3 h-3 rounded-sm" style="background-color: rgba(99,102,241,0.7);"></div>
                <div class="w-3 h-3 rounded-sm" style="background-color: rgba(99,102,241,0.9);"></div>
            </div>
            <span>More</span>
        </div>
    </div>"#);
    Html(html).into_response()
}

pub async fn htmx_pricing_funnel(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<FunnelParams>,
) -> Response {
    let Some(session) = get_session(&jar, &state) else {
        return Html("Unauthorized".to_string()).into_response();
    };
    let (default_start, default_end) = default_date_range();
    let start = params.start_at.unwrap_or(default_start);
    let end = params.end_at.unwrap_or(default_end);

    let steps_str = params.steps.as_deref().unwrap_or("/,/pricing,/signup");
    let steps: Vec<String> = steps_str.split(',').map(|s| s.trim().to_string()).collect();

    let data = match qsvc::fetch_funnel(&state.db, session.project_id, start, end, &steps).await {
        Ok(d) => d,
        Err(e) => return Html(format!("Error: {e}")).into_response(),
    };

    let first_count = data
        .first()
        .and_then(|d| d["visitors"].as_i64())
        .unwrap_or(1)
        .max(1);

    let mut html = String::from(
        r#"<div class="bg-white rounded-xl border border-gray-200 p-6">
            <h3 class="text-sm font-medium text-gray-700 mb-4">Conversion Funnel</h3>
            <div class="space-y-3">"#,
    );

    let mut prev_count = first_count;
    for (i, item) in data.iter().enumerate() {
        let step = item["step"].as_str().unwrap_or("-");
        let count = item["visitors"].as_i64().unwrap_or(0);
        let pct = (count as f64 / first_count as f64 * 100.0).min(100.0);
        let width = pct.max(2.0); // minimum width for visibility

        html.push_str(&format!(
            r#"<div>
                <div class="flex items-center justify-between mb-1">
                    <span class="text-sm font-medium text-gray-700 font-mono">{step}</span>
                    <span class="text-sm text-gray-500 tabular-nums">{count} visitors</span>
                </div>
                <div class="w-full bg-gray-100 rounded-full h-8 overflow-hidden">
                    <div class="h-full rounded-full flex items-center px-3"
                         style="width: {width:.1}%; background: linear-gradient(90deg, #6366f1 0%, #8b5cf6 100%);">
                        <span class="text-white text-xs font-medium">{pct:.1}%</span>
                    </div>
                </div>"#
        ));

        if i > 0 && prev_count > 0 {
            let drop = ((prev_count - count) as f64 / prev_count as f64 * 100.0).max(0.0);
            html.push_str(&format!(
                r#"<div class="flex items-center gap-1 mt-1 ml-2">
                    <svg class="w-3 h-3 text-red-400" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"/></svg>
                    <span class="text-xs text-red-400">{drop:.1}% drop-off</span>
                </div>"#
            ));
        }

        html.push_str("</div>");
        prev_count = count;
    }

    if data.is_empty() {
        html.push_str(r#"<p class="text-sm text-gray-400 text-center py-4">No funnel data</p>"#);
    }

    html.push_str("</div></div>");
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
