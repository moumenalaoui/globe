use serde::{Deserialize, Serialize};

/// Censorship signal for one Citizen Lab content category in one country,
/// derived from OONI web_connectivity measurements aggregated by
/// `category_code`. `anomaly_rate` is the share of measurements in the
/// category that were anomalous; `confirmed_count` corroborates with
/// OONI-confirmed blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryBlock {
    pub country_code: String,
    pub category_code: String,
    pub category_label: String,
    pub status: String,
    pub anomaly_rate: f64,
    pub anomaly_count: i64,
    pub confirmed_count: i64,
    pub measurement_count: i64,
    pub checked_date: String,
    pub source_notes: String,
}
