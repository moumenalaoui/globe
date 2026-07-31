use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologyBlock {
    pub country_code: String,
    pub category: String,
    pub technology: String,
    pub domain_or_url: String,
    pub test_name: String,
    pub status: String,
    pub anomaly_rate: f64,
    pub measurement_count: i64,
    pub checked_date: String,
    pub source_notes: String,
}
