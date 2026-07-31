use crate::{
    AppState,
    models::country::{ComputeBand, Country, SanctionsTier},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

fn row_to_country(row: &rusqlite::Row) -> rusqlite::Result<Country> {
    let tier_str: String = row.get(2)?;
    let band_str: String = row.get(7)?;
    Ok(Country {
        country_code: row.get(0)?,
        country_name: row.get(1)?,
        sanctions_tier: serde_json::from_str(&format!("\"{}\"", tier_str))
            .unwrap_or(SanctionsTier::ResourceConstrained),
        us_service_access: row.get(3)?,
        cloud_access_notes: row.get(4)?,
        export_control_notes: row.get(5)?,
        last_major_legal_change: row.get(6)?,
        compute_capacity_band: serde_json::from_str(&format!("\"{}\"", band_str))
            .unwrap_or(ComputeBand::Low),
        confidence: row.get(8)?,
        source_notes: row.get(9)?,
    })
}

const SELECT: &str = "SELECT country_code, country_name, sanctions_tier, us_service_access,
    cloud_access_notes, export_control_notes, last_major_legal_change,
    compute_capacity_band, confidence, source_notes FROM countries";

pub async fn list_countries(
    State(state): State<AppState>,
) -> Result<Json<Vec<Country>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map([], row_to_country)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}

pub async fn get_country(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<Country>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = conn
        .query_row(
            &format!("{} WHERE country_code = ?1", SELECT),
            rusqlite::params![code],
            row_to_country,
        )
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(result))
}
