// src/models/country.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Country {
    pub country_code: String,
    pub country_name: String,
    pub sanctions_tier: SanctionsTier,
    pub us_service_access: String,
    pub cloud_access_notes: String,
    pub export_control_notes: String,
    pub last_major_legal_change: Option<String>,
    pub compute_capacity_band: ComputeBand,
    pub confidence: String,
    pub source_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SanctionsTier {
    ComprehensivelySanctioned,
    PostSanctionsLag,
    UsComputeStack,
    ResourceConstrained,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComputeBand {
    VeryLow,
    Low,
    Medium,
    High,
}
