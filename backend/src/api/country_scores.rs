use crate::{AppState, models::country_score::CountryScore};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CountryScoresQuery {
    pub country: String,
}

fn row_to_score(row: &rusqlite::Row) -> rusqlite::Result<CountryScore> {
    Ok(CountryScore {
        id: row.get(0)?,
        country_code: row.get(1)?,
        source: row.get(2)?,
        year: row.get(3)?,
        score_overall: row.get(4)?,
        score_access: row.get(5)?,
        score_content: row.get(6)?,
        score_rights: row.get(7)?,
        classification: row.get(8)?,
        last_updated: row.get(9)?,
    })
}

const SELECT: &str = "SELECT id, country_code, source, year, score_overall, score_access,
    score_content, score_rights, classification, last_updated FROM country_scores
    WHERE country_code = ?1 ORDER BY year DESC, source ASC";

pub async fn list_country_scores(
    State(state): State<AppState>,
    Query(params): Query<CountryScoresQuery>,
) -> Result<Json<Vec<CountryScore>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map(rusqlite::params![params.country], row_to_score)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
