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
}
