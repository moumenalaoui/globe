use crate::{AppState, models::blocking_timeline::BlockingTimelinePoint};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TimelineQuery {
    pub country: String,
    pub technology: String,
}

fn row_to_point(row: &rusqlite::Row) -> rusqlite::Result<BlockingTimelinePoint> {
    Ok(BlockingTimelinePoint {
        country_code: row.get(0)?,
        technology: row.get(1)?,
        measurement_date: row.get(2)?,
        anomaly_count: row.get(3)?,
        confirmed_count: row.get(4)?,
        measurement_count: row.get(5)?,
        ok_count: row.get(6)?,
    })
}

const SELECT: &str = "SELECT country_code, technology, measurement_date, anomaly_count,
    confirmed_count, measurement_count, ok_count FROM blocking_timeline
    WHERE country_code = ?1 AND technology = ?2
    ORDER BY measurement_date ASC";

pub async fn list_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineQuery>,
) -> Result<Json<Vec<BlockingTimelinePoint>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map(
            rusqlite::params![params.country, params.technology],
            row_to_point,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
