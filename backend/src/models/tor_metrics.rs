use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorMetric {
    pub id: String,
    pub country_code: String,
    pub date: String,
    pub relay_users: Option<i64>,
    pub bridge_users: Option<i64>,
    pub bridge_relay_ratio: Option<f64>,
    pub blocking_signal: Option<String>,
    pub source: String,
}
