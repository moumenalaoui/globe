use crate::{
    AppState,
    engine::{contract, rules},
    models::{
        country::{ComputeBand, Country, SanctionsTier},
        deployment::{EvalRequest, EvaluateResponse},
    },
};
use axum::{Json, extract::State, http::StatusCode};

pub async fn evaluate(
    State(state): State<AppState>,
    Json(req): Json<EvalRequest>,
) -> Result<Json<EvaluateResponse>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let country = conn
        .query_row(
            "SELECT country_code, country_name, sanctions_tier, us_service_access,
             cloud_access_notes, export_control_notes, last_major_legal_change,
             compute_capacity_band, confidence, source_notes
             FROM countries WHERE country_code = ?1",
            rusqlite::params![req.country_code],
            |row| {
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
            },
        )
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let legacy_assessment = rules::evaluate(&country, &req);
    let (shared, evidence_items) = contract::load_shared(&conn, &country.country_code)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    contract::persist_evidence_items(&conn, &evidence_items)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut response = contract::build_response(&country, &req, &legacy_assessment, &shared);
    let evidence_refs = contract::collect_evidence_refs(&response);
    response.evidence_items = contract::load_evidence_items_by_ids(&conn, &evidence_refs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(response))
}
