use crate::models::{
    country::{Country, SanctionsTier},
    deployment::{
        BlockedPath, DeploymentAssessment, DeploymentPath, EvalRequest, OrgType, SensitivityLevel,
    },
};

pub fn evaluate(country: &Country, req: &EvalRequest) -> DeploymentAssessment {
    match country.sanctions_tier {
        SanctionsTier::ComprehensivelySanctioned => evaluate_sanctioned(country, req),
        SanctionsTier::PostSanctionsLag => evaluate_post_sanctions(country, req),
        SanctionsTier::UsComputeStack => evaluate_us_stack(country, req),
        SanctionsTier::ResourceConstrained => evaluate_resource_constrained(country, req),
    }
}

// ── Tier 1: Comprehensively sanctioned (Iran) ─────────────────────────────

fn evaluate_sanctioned(country: &Country, req: &EvalRequest) -> DeploymentAssessment {
    let feasible = vec![path_local_open_weight()];
    let mut warnings = vec![
        "Self-hosted open-weight models carry low telemetry risk when weights run fully locally.".to_string(),
        "The DeepSeek consumer app routes prompts to Chinese-controlled servers — not equivalent to self-hosting the weights.".to_string(),
        "OFAC GL 31 CFR § 560.540 covers enumerated communications categories but does not explicitly address generative AI chatbots. The policy gap is unresolved.".to_string(),
    ];

    if req.sensitivity == SensitivityLevel::Classified {
        warnings.push("Classified workloads require fully offline, air-gapped deployment. No external network connectivity permitted at inference time.".to_string());
    }

    if req.org_type == OrgType::Government {
        warnings.push("Government deployments must confirm physical control of inference hardware. No foreign-operated cloud path exists for this jurisdiction.".to_string());
    }

    let blocked = vec![
        BlockedPath {
            path_id: "us-cloud".to_string(),
            label: "US hyperscaler cloud".to_string(),
            reason: "OFAC ITSR prohibits US persons from providing services to Iran. No general license covers cloud-hosted AI inference.".to_string(),
            legal_basis: "OFAC Iran Transactions and Sanctions Regulations 31 CFR § 560".to_string(),
        },
        BlockedPath {
            path_id: "closed-model-api".to_string(),
            label: "US closed model APIs (OpenAI, Anthropic, Google)".to_string(),
            reason: "US-headquartered providers are prohibited from serving Iranian users under ITSR. Enforced via geo-blocking at the provider level.".to_string(),
            legal_basis: "OFAC ITSR 31 CFR § 560; provider terms of service".to_string(),
        },
        BlockedPath {
            path_id: "sovereign-vpc".to_string(),
            label: "Sovereign VPC on US infrastructure".to_string(),
            reason: "No US hyperscaler operates in Iran. No bilateral compute arrangement exists or is legally possible under current sanctions.".to_string(),
            legal_basis: "OFAC ITSR 31 CFR § 560; EAR bilateral deal framework".to_string(),
        },
    ];

    DeploymentAssessment {
        country_code: country.country_code.clone(),
        feasible_paths: feasible,
        blocked_paths: blocked,
        privacy_warnings: warnings,
        policy_brief: brief_sanctioned(country, req),
        confidence: "HIGH".to_string(),
    }
}

// ── Tier 2: Post-sanctions transition (Syria) ─────────────────────────────

fn evaluate_post_sanctions(country: &Country, req: &EvalRequest) -> DeploymentAssessment {
    let mut feasible = vec![path_local_open_weight()];
    let mut warnings = vec![
        "Formal sanctions relief (July 2025 EO) has not produced practical AI access parity. Lab geo-blocking, payment friction, and infrastructure gaps persist.".to_string(),
        "EAR Part 746 licensing requirements remain for controlled items even after OFAC revocation. Published open-weight weights are outside ECCN 4E091 scope by design.".to_string(),
    ];

    if req.sensitivity == SensitivityLevel::Classified {
        warnings.push("Classified workloads should not rely on the sovereign VPC path during the post-sanctions transition given EAR licensing uncertainty and infrastructure immaturity.".to_string());
    } else {
        // Non-classified: sovereign VPC is cautionary but feasible
        feasible.push(path_sovereign_vpc());
    }

    if req.sensitivity == SensitivityLevel::Public || req.sensitivity == SensitivityLevel::Internal
    {
        feasible.push(path_us_cloud_cautionary());
        warnings.push("US cloud APIs are legally unblocked but provider geo-blocking policies have not all updated since the July 2025 EO. Verify with each provider before depending on this path.".to_string());
    }

    let blocked = vec![
        BlockedPath {
            path_id: "frontier-cloud-reliable".to_string(),
            label: "US frontier cloud (reliable production use)".to_string(),
            reason: "Legally unblocked since July 2025 but provider-level geo-blocking, payment infrastructure gaps, and EAR residual controls make this path unreliable for production use.".to_string(),
            legal_basis: "White House EO on Revocation of Syria Sanctions July 2025; EAR Part 746 residual controls".to_string(),
        },
    ];

    DeploymentAssessment {
        country_code: country.country_code.clone(),
        feasible_paths: feasible,
        blocked_paths: blocked,
        privacy_warnings: warnings,
        policy_brief: brief_post_sanctions(country, req),
        confidence: "HIGH".to_string(),
    }
}

// ── Tier 3: US compute stack (UAE, Saudi Arabia) ──────────────────────────

fn evaluate_us_stack(country: &Country, req: &EvalRequest) -> DeploymentAssessment {
    let mut feasible = vec![
        path_local_open_weight(),
        path_sovereign_vpc(),
        path_us_cloud(),
    ];
    let mut warnings = vec![
        "Data residency in-country does not eliminate US legal process exposure. CLOUD Act and US subpoena authority reach foreign-located servers operated by US-headquartered entities.".to_string(),
        "Gulf states hedge at the model layer even within the US compute stack — open-weight models (Jais, Qwen3) are used alongside closed US APIs for cost and sovereignty reasons. This is not misalignment.".to_string(),
    ];

    if req.sensitivity == SensitivityLevel::Classified && req.org_type == OrgType::Government {
        feasible.retain(|p| p.path_id != "us-cloud");
        warnings.push("Classified government workloads require locally-operated inference. Sovereign VPC on US infrastructure does not fully satisfy foreign legal exposure requirements under CLOUD Act.".to_string());
    }

    if req.sensitivity == SensitivityLevel::Sensitive {
        warnings.push("Sensitive workloads on US cloud infrastructure carry residual foreign-operator risk. Sovereign VPC or local deployment recommended for prompts containing non-public government information.".to_string());
    }

    DeploymentAssessment {
        country_code: country.country_code.clone(),
        feasible_paths: feasible,
        blocked_paths: vec![],
        privacy_warnings: warnings,
        policy_brief: brief_us_stack(country, req),
        confidence: "HIGH".to_string(),
    }
}

// ── Tier 4: Resource constrained (Iraq) ───────────────────────────────────

fn evaluate_resource_constrained(country: &Country, req: &EvalRequest) -> DeploymentAssessment {
    let mut feasible = vec![path_local_small_model()];
    let mut warnings = vec![
        "Cost is the primary constraint, not legal status. No country-program OFAC sanctions apply.".to_string(),
        "No in-country hyperscaler infrastructure. Cloud access routes through regional hubs, adding latency and cost.".to_string(),
        "Chinese open-weight models (Qwen3, DeepSeek distilled variants) are the most cost-accessible path for local deployment — a function of price and deployability, not geopolitical alignment.".to_string(),
    ];

    if req.sensitivity != SensitivityLevel::Classified {
        feasible.push(path_selective_cloud_fallback());
    }

    if req.sensitivity == SensitivityLevel::Classified {
        warnings.push("Classified workloads require local deployment only. No cloud fallback path is appropriate regardless of provider or cost.".to_string());
    }

    if req.org_type == OrgType::Government {
        warnings.push("No bilateral compute diplomacy pathway exists at the scale available to Gulf states. The bilateral deal model was built for strategic partners with leverage to trade — it has no equivalent instrument for resource-constrained states.".to_string());
    }

    let blocked = vec![
        BlockedPath {
            path_id: "frontier-cloud-at-scale".to_string(),
            label: "Frontier cloud APIs at scale".to_string(),
            reason: "Cost and payment infrastructure make sustained frontier API use impractical for resource-constrained public-sector actors. No legal block applies.".to_string(),
            legal_basis: "No legal constraint. Practical constraint: cost, FX access, procurement process.".to_string(),
        },
    ];

    DeploymentAssessment {
        country_code: country.country_code.clone(),
        feasible_paths: feasible,
        blocked_paths: blocked,
        privacy_warnings: warnings,
        policy_brief: brief_resource_constrained(country, req),
        confidence: "MEDIUM".to_string(),
    }
}

// ── Path constructors ─────────────────────────────────────────────────────

fn path_local_open_weight() -> DeploymentPath {
    DeploymentPath {
        path_id: "local-open-weight".to_string(),
        label: "Fully local open-weight deployment".to_string(),
        description: "Self-hosted open-weight model on controlled infrastructure. No external network required for inference. Published weights fall outside ECCN 4E091 export control scope by design.".to_string(),
        requires_internet: false,
        foreign_operator_risk: "NONE".to_string(),
        recommended_models: vec!["deepseek-r1".to_string(), "qwen3-32b".to_string(), "mistral-7b".to_string()],
    }
}

fn path_local_small_model() -> DeploymentPath {
    DeploymentPath {
        path_id: "local-small-model".to_string(),
        label: "Local small model on workstation hardware".to_string(),
        description: "Lightweight open-weight model deployable on workstation-class hardware without GPU cluster. Prioritises cost and deployability over frontier capability.".to_string(),
        requires_internet: false,
        foreign_operator_risk: "NONE".to_string(),
        recommended_models: vec!["mistral-7b".to_string(), "qwen3-32b".to_string()],
    }
}

fn path_sovereign_vpc() -> DeploymentPath {
    DeploymentPath {
        path_id: "sovereign-vpc".to_string(),
        label: "Sovereign VPC with open-weight model".to_string(),
        description: "Open-weight model hosted in a country-controlled virtual private cloud. Improves data residency but does not eliminate foreign legal exposure if the infrastructure operator is foreign-headquartered.".to_string(),
        requires_internet: true,
        foreign_operator_risk: "LOW_TO_MEDIUM".to_string(),
        recommended_models: vec!["qwen3-32b".to_string(), "jais-30b".to_string(), "deepseek-r1".to_string()],
    }
}

fn path_us_cloud() -> DeploymentPath {
    DeploymentPath {
        path_id: "us-cloud".to_string(),
        label: "US hyperscaler cloud or closed frontier API".to_string(),
        description: "Full access to US frontier models via API or hyperscaler deployment. Data residency options available but CLOUD Act authority reaches foreign-located servers operated by US entities.".to_string(),
        requires_internet: true,
        foreign_operator_risk: "MEDIUM".to_string(),
        recommended_models: vec!["gpt-4o".to_string(), "gemini-pro".to_string()],
    }
}

fn path_us_cloud_cautionary() -> DeploymentPath {
    DeploymentPath {
        path_id: "us-cloud-cautionary".to_string(),
        label: "US cloud APIs (legally available, practically uncertain)".to_string(),
        description: "Legally unblocked since July 2025 EO but provider geo-blocking, payment infrastructure gaps, and EAR residual controls mean access is not guaranteed. Verify provider policy before depending on this path.".to_string(),
        requires_internet: true,
        foreign_operator_risk: "MEDIUM".to_string(),
        recommended_models: vec!["gpt-4o".to_string()],
    }
}

fn path_selective_cloud_fallback() -> DeploymentPath {
    DeploymentPath {
        path_id: "selective-cloud-fallback".to_string(),
        label: "Selective cloud fallback for non-sensitive tasks".to_string(),
        description: "Limited use of cloud APIs for public-facing or non-sensitive workloads where cost of local deployment outweighs sensitivity concerns. Not appropriate for internal or classified data.".to_string(),
        requires_internet: true,
        foreign_operator_risk: "MEDIUM".to_string(),
        recommended_models: vec!["gpt-4o".to_string(), "gemini-pro".to_string()],
    }
}

// ── Policy brief generators ───────────────────────────────────────────────

fn brief_sanctioned(country: &Country, req: &EvalRequest) -> String {
    format!(
        "{} is comprehensively sanctioned under OFAC ITSR, blocking all US-origin cloud, API, and managed AI services. Published open-weight weights fall outside ECCN 4E091 by design, making self-hosted deployment the only legally unambiguous path. For {:?} workloads at {:?} sensitivity, fully local inference on controlled hardware is the recommended architecture. The sharpest unresolved policy question is whether Treasury's existing internet-freedom general license (31 CFR § 560.540) covers generative AI chatbot access, or whether the license requires updating to reflect developments since its last major revision in 2022.",
        country.country_name, req.org_type, req.sensitivity
    )
}

fn brief_post_sanctions(country: &Country, req: &EvalRequest) -> String {
    format!(
        "{} formal sanctions were revoked by EO effective July 2025, but practical AI access has not reached parity with legal status. EAR Part 746 residual licensing, provider geo-blocking inertia, and payment infrastructure gaps are the active constraints — legal relief does not by itself restore effective access. For {:?} workloads at {:?} sensitivity, local or sovereign-VPC open-weight deployment is the recommended architecture until provider policies demonstrably update.",
        country.country_name, req.org_type, req.sensitivity
    )
}

fn brief_us_stack(country: &Country, req: &EvalRequest) -> String {
    format!(
        "{} is deeply integrated into the US compute stack following the November 2025 GB300 chip approvals and associated bilateral partnerships. Full access to US frontier models and hyperscaler cloud is available. The country hedges at the model layer — open-weight models including Gulf-origin Arabic models are used alongside closed US APIs for sovereignty, localisation, and cost reasons. For {:?} workloads at {:?} sensitivity, US-stack cloud is appropriate for public workloads; sovereign VPC or local open-weight is recommended for sensitive or classified government use. Data residency in-country does not eliminate US legal process exposure under the CLOUD Act if the infrastructure operator is US-headquartered.",
        country.country_name, req.org_type, req.sensitivity
    )
}

fn brief_resource_constrained(country: &Country, req: &EvalRequest) -> String {
    format!(
        "{} faces no country-program OFAC sanctions, but cost, infrastructure gaps, and the absence of a bilateral compute partnership create practical access constraints that approximate sanctions for frontier AI services. The bilateral deal model that delivered GB300 chips to the Gulf was built for strategic partners with leverage to trade — it has no equivalent instrument for resource-constrained states. Chinese open-weight models fill this gap by default, not by design. For {:?} workloads at {:?} sensitivity, small local open-weight models are the most accessible and sovereignty-preserving path available without a Gulf-scale infrastructure investment.",
        country.country_name, req.org_type, req.sensitivity
    )
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use crate::models::{
        country::{ComputeBand, Country, SanctionsTier},
        deployment::{EvalRequest, OrgType, SensitivityLevel},
    };
    use insta::assert_json_snapshot;
    use serde::Serialize;

    #[derive(Serialize)]
    struct ScenarioSnapshot {
        country_code: String,
        org_type: OrgType,
        sensitivity: SensitivityLevel,
        assessment: crate::models::deployment::DeploymentAssessment,
    }

    fn sample_country(
        country_code: &str,
        country_name: &str,
        sanctions_tier: SanctionsTier,
    ) -> Country {
        Country {
            country_code: country_code.to_string(),
            country_name: country_name.to_string(),
            sanctions_tier,
            us_service_access: "seeded".to_string(),
            cloud_access_notes: "seeded".to_string(),
            export_control_notes: "seeded".to_string(),
            last_major_legal_change: Some("2025-01-01".to_string()),
            compute_capacity_band: ComputeBand::High,
            confidence: "HIGH".to_string(),
            source_notes: "seeded".to_string(),
        }
    }

    #[test]
    fn snapshot_all_supported_country_scenarios() {
        let countries = [
            sample_country("IR", "Iran", SanctionsTier::ComprehensivelySanctioned),
            sample_country("SY", "Syria", SanctionsTier::PostSanctionsLag),
            sample_country("AE", "United Arab Emirates", SanctionsTier::UsComputeStack),
            sample_country("SA", "Saudi Arabia", SanctionsTier::UsComputeStack),
            sample_country("IQ", "Iraq", SanctionsTier::ResourceConstrained),
        ];
        let org_types = [
            OrgType::Government,
            OrgType::Ngo,
            OrgType::Private,
            OrgType::Academic,
        ];
        let sensitivities = [
            SensitivityLevel::Public,
            SensitivityLevel::Internal,
            SensitivityLevel::Sensitive,
            SensitivityLevel::Classified,
        ];

        let mut snapshots = Vec::new();
        for country in countries {
            for org_type in &org_types {
                for sensitivity in &sensitivities {
                    let request = EvalRequest {
                        country_code: country.country_code.clone(),
                        org_type: org_type.clone(),
                        sensitivity: sensitivity.clone(),
                    };
                    snapshots.push(ScenarioSnapshot {
                        country_code: country.country_code.clone(),
                        org_type: org_type.clone(),
                        sensitivity: sensitivity.clone(),
                        assessment: evaluate(&country, &request),
                    });
                }
            }
        }

        assert_json_snapshot!("rules_all_supported_country_scenarios", snapshots);
    }
}
