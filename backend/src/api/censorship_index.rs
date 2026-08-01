use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use std::collections::HashMap;

// A transparent composite: blend the freedom indices we hold per country into a
// single 0–100 score. Weights favour V-Dem (current, best coverage) over RSF.
// Missing components are dropped and the remaining weights renormalised, so a
// country with only V-Dem still gets a score. Both inputs are already 0–100
// "higher = freer", so the blend is too; the map colours by its inverse
// (censorship = 100 − freedom).
const W_VDEM: f64 = 0.5;
const W_RSF: f64 = 0.3;

#[derive(Default)]
struct Components {
    vdem: Option<f64>,
    rsf: Option<f64>,
}

#[derive(Serialize)]
pub struct CensorshipIndexEntry {
    pub country_code: String,
    /// 0–100, higher = more censored (what the choropleth colours by).
    pub censorship_score: f64,
    /// 0–100, higher = more free (the composite before inversion).
    pub freedom_score: f64,
    pub vdem: Option<f64>,
    pub rsf: Option<f64>,
    pub component_count: i64,
}

/// Composite censorship index for every country that has at least one freedom
/// index. Computed on the fly from `country_scores` so it always reflects the
/// latest fetched V-Dem / RSF values.
pub async fn list_censorship_index(
    State(state): State<AppState>,
) -> Result<Json<Vec<CensorshipIndexEntry>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut stmt = conn
        .prepare(
            "SELECT country_code, source, score_overall FROM country_scores
             WHERE source IN ('V_DEM','RSF') AND score_overall IS NOT NULL",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok());

    let mut by_country: HashMap<String, Components> = HashMap::new();
    for (code, source, score) in rows {
        let entry = by_country.entry(code).or_default();
        match source.as_str() {
            "V_DEM" => entry.vdem = Some(score),
            "RSF" => entry.rsf = Some(score),
            _ => {}
        }
    }

    let mut result: Vec<CensorshipIndexEntry> = by_country
        .into_iter()
        .filter_map(|(code, c)| compute(&code, &c))
        .collect();
    // Most-censored first — a stable, useful default order for any consumer.
    result.sort_by(|a, b| b.censorship_score.partial_cmp(&a.censorship_score).unwrap());

    Ok(Json(result))
}

fn compute(code: &str, c: &Components) -> Option<CensorshipIndexEntry> {
    let mut weighted = 0.0;
    let mut weight = 0.0;
    let mut count = 0i64;

    for (value, w) in [(c.vdem, W_VDEM), (c.rsf, W_RSF)] {
        if let Some(v) = value {
            weighted += v * w;
            weight += w;
            count += 1;
        }
    }
    if weight == 0.0 {
        return None;
    }

    let freedom = (weighted / weight).clamp(0.0, 100.0);
    let round1 = |x: f64| (x * 10.0).round() / 10.0;

    Some(CensorshipIndexEntry {
        country_code: code.to_string(),
        censorship_score: round1(100.0 - freedom),
        freedom_score: round1(freedom),
        vdem: c.vdem,
        rsf: c.rsf,
        component_count: count,
    })
}
