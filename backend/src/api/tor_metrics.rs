use crate::{AppState, models::tor_metric::TorMetric};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TorMetricsQuery {
    pub country: String,
}

fn row_to_metric(row: &rusqlite::Row) -> rusqlite::Result<TorMetric> {
    Ok(TorMetric {
        id: row.get(0)?,
        country_code: row.get(1)?,
        date: row.get(2)?,
        relay_users: row.get(3)?,
        bridge_users: row.get(4)?,
        bridge_relay_ratio: row.get(5)?,
        blocking_signal: row.get(6)?,
        source: row.get(7)?,
        obfs4_low: row.get(8)?,
        obfs4_high: row.get(9)?,
        snowflake_low: row.get(10)?,
        snowflake_high: row.get(11)?,
        webtunnel_low: row.get(12)?,
        webtunnel_high: row.get(13)?,
    })
}

const SELECT: &str = "SELECT id, country_code, date, relay_users, bridge_users,
    bridge_relay_ratio, blocking_signal, source,
    obfs4_low, obfs4_high, snowflake_low, snowflake_high, webtunnel_low, webtunnel_high
    FROM tor_metrics
    WHERE country_code = ?1 ORDER BY date ASC";

pub async fn list_tor_metrics(
    State(state): State<AppState>,
    Query(params): Query<TorMetricsQuery>,
) -> Result<Json<Vec<TorMetric>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map(rusqlite::params![params.country], row_to_metric)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
