use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingTimelinePoint {
    pub country_code: String,
    pub technology: String,
    pub measurement_date: String,
    pub anomaly_count: i64,
    pub confirmed_count: i64,
    pub measurement_count: i64,
    pub ok_count: i64,
}
