use serde::{Deserialize, Serialize};

/// One internet-outage window for one country, as detected by one IODA
/// datasource. Timestamps are unix seconds (IODA's native unit); the frontend
/// decides what counts as "currently active" from `end_ts` relative to now.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutageEvent {
    pub id: String,
    pub country_code: String,
    pub start_ts: i64,
    pub duration_secs: i64,
    pub end_ts: i64,
    pub datasource: String,
    pub method: Option<String>,
    pub score: f64,
    pub source: String,
}
