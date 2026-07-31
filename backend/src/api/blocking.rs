use crate::{AppState, models::technology_block::TechnologyBlock};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use rusqlite::ToSql;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockingQuery {
    pub country: Option<String>,
    pub layer: Option<String>,
}

fn row_to_block(row: &rusqlite::Row) -> rusqlite::Result<TechnologyBlock> {
    Ok(TechnologyBlock {
        country_code: row.get(0)?,
        category: row.get(1)?,
        technology: row.get(2)?,
        domain_or_url: row.get(3)?,
        test_name: row.get(4)?,
        status: row.get(5)?,
        anomaly_rate: row.get(6)?,
        measurement_count: row.get(7)?,
        checked_date: row.get(8)?,
        source_notes: row.get(9)?,
    })
}

const SELECT: &str = "SELECT country_code, category, technology, domain_or_url, test_name,
    status, anomaly_rate, measurement_count, checked_date, source_notes FROM technology_blocks";

pub async fn list_blocking(
    State(state): State<AppState>,
    Query(params): Query<BlockingQuery>,
) -> Result<Json<Vec<TechnologyBlock>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut clauses = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    if let Some(country) = &params.country {
        binds.push(country.clone());
        clauses.push(format!("country_code = ?{}", binds.len()));
    }
    if let Some(layer) = &params.layer {
        binds.push(layer.clone());
        clauses.push(format!("category = ?{}", binds.len()));
    }

    let query = if clauses.is_empty() {
        SELECT.to_string()
    } else {
        format!("{SELECT} WHERE {}", clauses.join(" AND "))
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bind_refs: Vec<&dyn ToSql> = binds.iter().map(|b| b as &dyn ToSql).collect();
    let result = stmt
        .query_map(bind_refs.as_slice(), row_to_block)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
