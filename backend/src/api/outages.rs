use crate::{AppState, models::outage_event::OutageEvent};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use rusqlite::ToSql;
use serde::Deserialize;

// An outage counts as "active" for the globe overlay if it ended within the
// last 48h (or is still ongoing). IODA windows are historical, so a hard
// "end >= now" would almost never light anything up; a short grace keeps the
// live map showing genuinely recent disruption without going stale.
const ACTIVE_GRACE_SECS: i64 = 48 * 60 * 60;

#[derive(Deserialize)]
pub struct OutagesQuery {
    pub country: Option<String>,
    pub active: Option<bool>,
}

fn row_to_outage(row: &rusqlite::Row) -> rusqlite::Result<OutageEvent> {
    Ok(OutageEvent {
        id: row.get(0)?,
        country_code: row.get(1)?,
        start_ts: row.get(2)?,
        duration_secs: row.get(3)?,
        end_ts: row.get(4)?,
        datasource: row.get(5)?,
        method: row.get(6)?,
        score: row.get(7)?,
        source: row.get(8)?,
    })
}

const SELECT: &str = "SELECT id, country_code, start_ts, duration_secs, end_ts,
    datasource, method, score, source FROM outage_events";

pub async fn list_outages(
    State(state): State<AppState>,
    Query(params): Query<OutagesQuery>,
) -> Result<Json<Vec<OutageEvent>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut clauses: Vec<String> = Vec::new();
    let mut binds: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(country) = &params.country {
        binds.push(Box::new(country.clone()));
        clauses.push(format!("country_code = ?{}", binds.len()));
    }
    if params.active == Some(true) {
        let threshold = now_unix() - ACTIVE_GRACE_SECS;
        binds.push(Box::new(threshold));
        clauses.push(format!("end_ts >= ?{}", binds.len()));
    }

    let query = if clauses.is_empty() {
        format!("{SELECT} ORDER BY start_ts DESC")
    } else {
        format!("{SELECT} WHERE {} ORDER BY start_ts DESC", clauses.join(" AND "))
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bind_refs: Vec<&dyn ToSql> = binds.iter().map(|b| b.as_ref()).collect();
    let result = stmt
        .query_map(bind_refs.as_slice(), row_to_outage)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
