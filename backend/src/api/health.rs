//! Staleness-aware liveness check.
//!
//! This used to return the literal string "ok" unconditionally, which meant an
//! uptime monitor stayed green over a database that had not been refreshed in
//! weeks. "The process is up" and "the data is current" are different claims,
//! and only the second one matters for a dashboard whose whole job is to show
//! current data.
//!
//! Exposes only aggregate freshness — a date and an age — never row content,
//! so it stays safe to point an external uptime monitor at.

use crate::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

/// Age past which the data is considered stale and `/health` starts failing.
/// Overridable because the right threshold depends on how often the fetchers
/// are actually run; the default assumes at least one refresh every other day.
const DEFAULT_MAX_AGE_DAYS: i64 = 2;

#[derive(Serialize)]
pub struct Health {
    /// "ok" or "stale" — the process is up either way; this describes the data.
    status: &'static str,
    /// Most recent `last_updated` across the fetched tables, `YYYY-MM-DD`.
    newest_data: Option<String>,
    /// Whole days between `newest_data` and today, UTC.
    age_days: Option<i64>,
    max_age_days: i64,
}

/// 200 while the freshest row is within the threshold, 503 once it is not (or
/// when there is no fetched data at all), so a plain HTTP monitor catches it.
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let max_age_days = std::env::var("HEALTH_MAX_AGE_DAYS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_MAX_AGE_DAYS);

    let newest = newest_data(&state);
    let age_days = newest.as_deref().and_then(days_since);

    let healthy = matches!(age_days, Some(age) if age <= max_age_days);
    let body = Health {
        status: if healthy { "ok" } else { "stale" },
        newest_data: newest,
        age_days,
        max_age_days,
    };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}

/// Freshest `last_updated` across the tables that carry one. A DB error here
/// reads as "no data", which fails the check — the right direction to fail.
fn newest_data(state: &AppState) -> Option<String> {
    let conn = state.lock().ok()?;
    conn.query_row(
        "SELECT MAX(d) FROM (
             SELECT MAX(last_updated) AS d FROM country_scores
             UNION ALL
             SELECT MAX(last_updated) AS d FROM outage_events
         )",
        [],
        |row| row.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
}

/// Whole days from a `YYYY-MM-DD` date to today, UTC. None if unparseable.
fn days_since(date: &str) -> Option<i64> {
    Some(crate::util::date::days_between(date, &crate::util::date::today_iso())?.max(0))
}
