use crate::{AppState, models::deal::Deal};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DealQuery {
    pub country: Option<String>,
}

fn row_to_deal(row: &rusqlite::Row) -> rusqlite::Result<Deal> {
    Ok(Deal {
        deal_id: row.get(0)?,
        country_code: row.get(1)?,
        partner_type: row.get(2)?,
        partner_name: row.get(3)?,
        deal_date: row.get(4)?,
        deal_type: row.get(5)?,
        description: row.get(6)?,
        stack_layer: row.get(7)?,
        confidence: row.get(8)?,
        source_notes: row.get(9)?,
    })
}

const SELECT: &str = "SELECT deal_id, country_code, partner_type, partner_name,
    deal_date, deal_type, description, stack_layer, confidence, source_notes
    FROM deals";

pub async fn list_deals(
    State(state): State<AppState>,
    Query(params): Query<DealQuery>,
) -> Result<Json<Vec<Deal>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = match params.country {
        Some(code) => {
            let mut stmt = conn
                .prepare(&format!("{} WHERE country_code = ?1", SELECT))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            stmt.query_map(rusqlite::params![code], row_to_deal)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .filter_map(|r| r.ok())
                .collect()
        }
        None => {
            let mut stmt = conn
                .prepare(SELECT)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            stmt.query_map([], row_to_deal)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .filter_map(|r| r.ok())
                .collect()
        }
    };
    Ok(Json(result))
}
