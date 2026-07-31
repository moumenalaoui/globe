use crate::{AppState, models::signal::AdoptionSignal};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SignalQuery {
    pub country: Option<String>,
}

fn row_to_signal(row: &rusqlite::Row) -> rusqlite::Result<AdoptionSignal> {
    Ok(AdoptionSignal {
        signal_id: row.get(0)?,
        country_code: row.get(1)?,
        provider: row.get(2)?,
        model_or_service: row.get(3)?,
        user_segment: row.get(4)?,
        signal_type: row.get(5)?,
        value_text: row.get(6)?,
        signal_date: row.get(7)?,
        confidence: row.get(8)?,
        source_notes: row.get(9)?,
    })
}

const SELECT: &str = "SELECT signal_id, country_code, provider, model_or_service,
    user_segment, signal_type, value_text, signal_date, confidence, source_notes
    FROM adoption_signals";

pub async fn list_signals(
    State(state): State<AppState>,
    Query(params): Query<SignalQuery>,
) -> Result<Json<Vec<AdoptionSignal>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = match params.country {
        Some(code) => {
            let mut stmt = conn
                .prepare(&format!("{} WHERE country_code = ?1", SELECT))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            stmt.query_map(rusqlite::params![code], row_to_signal)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .filter_map(|r| r.ok())
                .collect()
        }
        None => {
            let mut stmt = conn
                .prepare(SELECT)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            stmt.query_map([], row_to_signal)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .filter_map(|r| r.ok())
                .collect()
        }
    };
    Ok(Json(result))
}
