use crate::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RankingsQuery {
    /// Which index to rank by (country_scores.source). Defaults to V_DEM.
    pub source: Option<String>,
    /// "asc" (least-free / most-censored first, the default) or "desc".
    pub order: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct RankingEntry {
    pub country_code: String,
    pub country_name: String,
    pub region: Option<String>,
    pub score_overall: f64,
    pub classification: Option<String>,
    pub year: i64,
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<RankingEntry> {
    Ok(RankingEntry {
        country_code: row.get(0)?,
        country_name: row.get(1)?,
        region: row.get(2)?,
        score_overall: row.get(3)?,
        classification: row.get(4)?,
        year: row.get(5)?,
    })
}

/// Whole-world ranking for one freedom index, names joined in so the client
/// renders a leaderboard directly. `order` is whitelisted (never interpolated
/// from raw input) since it can't be a bound parameter.
pub async fn list_rankings(
    State(state): State<AppState>,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<Vec<RankingEntry>>, StatusCode> {
    let source = params.source.unwrap_or_else(|| "V_DEM".to_string());
    let direction = match params.order.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC", // least-free first — the "most censored" framing
    };
    let limit = params.limit.unwrap_or(200).clamp(1, 500);

    let sql = format!(
        "SELECT cs.country_code, cr.country_name, cr.region, cs.score_overall,
                cs.classification, cs.year
         FROM country_scores cs
         JOIN country_reference cr USING (country_code)
         WHERE cs.source = ?1 AND cs.score_overall IS NOT NULL
         ORDER BY cs.score_overall {direction}
         LIMIT ?2"
    );

    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map(rusqlite::params![source, limit], row_to_entry)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
