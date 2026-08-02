use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryScore {
    pub id: String,
    pub country_code: String,
    pub source: String,
    pub year: i64,
    pub score_overall: Option<f64>,
    pub score_access: Option<f64>,
    pub score_content: Option<f64>,
    pub score_rights: Option<f64>,
    pub classification: Option<String>,
    pub last_updated: Option<String>,
    // V-Dem internet-censorship sub-scores, present on V_DEM rows only and
    // null everywhere else. Normalised 0-100 higher = freer, same direction as
    // score_overall. Each carries V-Dem's credible interval on the same scale;
    // the panel shows the point and keeps the interval on hover.
    pub score_vdem_filtering: Option<f64>,
    pub score_vdem_filtering_low: Option<f64>,
    pub score_vdem_filtering_high: Option<f64>,
    pub score_vdem_shutdown: Option<f64>,
    pub score_vdem_shutdown_low: Option<f64>,
    pub score_vdem_shutdown_high: Option<f64>,
    pub score_vdem_censorship: Option<f64>,
    pub score_vdem_censorship_low: Option<f64>,
    pub score_vdem_censorship_high: Option<f64>,
}
