use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRelease {
    pub model_id: String,
    pub provider: String,
    pub model_name: String,
    pub origin_country: String,
    pub release_date: String,
    pub weight_access: WeightAccess,
    pub license_type: String,
    pub parameter_class: String,
    pub local_deployability: LocalDeployability,
    pub telemetry_risk_default: String,
    pub confidence: String,
    #[serde(default)]
    pub huggingface_repo_id: Option<String>,
    pub source_notes: String,
    #[serde(default)]
    pub downloads_all_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WeightAccess {
    OpenWeight,
    ClosedApi,
    SourceAvailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LocalDeployability {
    LaptopOrWorkstation,
    WorkstationToServer,
    Server,
    ServerRequired,
    CloudOnly,
}
