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
    // Per-transport bridge estimates. Tor publishes these as a low/high
    // interval rather than a point value; both bounds are exposed so a
    // consumer can see the published uncertainty, and the sidebar renders
    // the midpoint. Null when the combined CSV has no row for that
    // country-date-transport (it lags the per-country CSV slightly).
    pub obfs4_low: Option<i64>,
    pub obfs4_high: Option<i64>,
    pub snowflake_low: Option<i64>,
    pub snowflake_high: Option<i64>,
    pub webtunnel_low: Option<i64>,
    pub webtunnel_high: Option<i64>,
}
