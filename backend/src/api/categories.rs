use crate::{AppState, models::category_block::CategoryBlock};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CategoriesQuery {
    pub country: String,
}

fn row_to_category(row: &rusqlite::Row) -> rusqlite::Result<CategoryBlock> {
    Ok(CategoryBlock {
        country_code: row.get(0)?,
        category_code: row.get(1)?,
        category_label: row.get(2)?,
        status: row.get(3)?,
        anomaly_rate: row.get(4)?,
        anomaly_count: row.get(5)?,
        confirmed_count: row.get(6)?,
        measurement_count: row.get(7)?,
        checked_date: row.get(8)?,
        source_notes: row.get(9)?,
    })
}

// Ordered by anomaly rate so the most-censored categories come first — the
// frontend renders them top-down as a severity ranking.
const SELECT: &str = "SELECT country_code, category_code, category_label, status,
    anomaly_rate, anomaly_count, confirmed_count, measurement_count, checked_date, source_notes
    FROM category_blocks WHERE country_code = ?1 ORDER BY anomaly_rate DESC";

pub async fn list_categories(
    State(state): State<AppState>,
    Query(params): Query<CategoriesQuery>,
) -> Result<Json<Vec<CategoryBlock>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map(rusqlite::params![params.country], row_to_category)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
