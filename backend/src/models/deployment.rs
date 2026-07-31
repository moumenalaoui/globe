// src/models/deployment.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRequest {
    pub country_code: String,
    pub sensitivity: SensitivityLevel,
    pub org_type: OrgType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentAssessment {
    pub country_code: String,
    pub feasible_paths: Vec<DeploymentPath>,
    pub blocked_paths: Vec<BlockedPath>,
    pub privacy_warnings: Vec<String>,
    pub policy_brief: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPath {
    pub path_id: String,
    pub label: String,
    pub description: String,
    pub requires_internet: bool,
    pub foreign_operator_risk: String,
    pub recommended_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedPath {
    pub path_id: String,
    pub label: String,
    pub reason: String,
    pub legal_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensitivityLevel {
    Public,
    Internal,
    Sensitive,
    Classified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrgType {
    Government,
    Ngo,
    Private,
    Academic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Recommended,
    AllowedWithCaveats,
    Fragile,
    PracticallyUnavailable,
    LegallyBlocked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateResponse {
    pub scenario: Scenario,
    pub summary: Summary,
    pub paths: Vec<PathAssessment>,
    pub constraints: Vec<ConstraintAssessment>,
    pub evidence_items: Vec<EvidenceItem>,
    pub policy_levers: Vec<PolicyLever>,
    pub decision_trace: Vec<String>,
    pub diff_from_baseline: Option<DiffFromBaseline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub country_code: String,
    pub org_type: OrgType,
    pub sensitivity: SensitivityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub recommended_path_id: String,
    pub overall_posture: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathAssessment {
    pub path_id: String,
    pub label: String,
    pub status: Status,
    pub description: String,
    pub why: Vec<String>,
    pub dependencies: Vec<PathDependencyRef>,
    pub risks: Vec<Risk>,
    pub evidence_refs: Vec<String>,
    pub recommended_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathDependencyRef {
    pub dependency_type: String,
    pub dependency_target: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Risk {
    #[serde(rename = "type")]
    pub risk_type: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintAssessment {
    pub constraint_id: String,
    #[serde(rename = "type")]
    pub constraint_type: String,
    pub status: Status,
    pub target_type: String,
    pub target_id: String,
    pub summary: String,
    pub confidence: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyLever {
    pub lever: String,
    pub effect: String,
    pub affects_constraint_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFromBaseline {
    pub baseline: BaselineScenario,
    pub changed_paths: Vec<ChangedPath>,
    pub changed_constraints: Vec<ChangedConstraint>,
    pub changed_risks: Vec<ChangedRisk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineScenario {
    pub org_type: OrgType,
    pub sensitivity: SensitivityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedPath {
    pub path_id: String,
    pub from: Status,
    pub to: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedConstraint {
    pub constraint_id: String,
    pub from: Status,
    pub to: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedRisk {
    pub message: String,
    pub before_present: bool,
    pub after_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryConstraintRow {
    pub constraint_id: String,
    pub country_code: String,
    pub target_type: String,
    pub target_id: String,
    pub constraint_type: String,
    pub status: Status,
    pub summary: String,
    pub applies_to_org_type: Option<String>,
    pub applies_to_sensitivity: Option<String>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub evidence_id: String,
    pub source_type: String,
    pub title: String,
    pub publisher: Option<String>,
    pub url: Option<String>,
    pub observed_at: Option<String>,
    pub claim_text: String,
    pub confidence: String,
}
