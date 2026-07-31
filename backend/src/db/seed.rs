use crate::models::{
    country::Country, country_reference::CountryReference, deal::Deal, deployment::Status,
    model_release::ModelRelease, signal::AdoptionSignal,
};
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;

/// Reads and parses one seed file. Both failure modes name the path — a bare
/// `No such file or directory` from four different loaders is impossible to
/// act on, and this one kills startup before the listener binds.
fn read_seed<T: serde::de::DeserializeOwned>(name: &str) -> Result<Vec<T>> {
    let path = format!("data/seed/{name}");
    let data = fs::read_to_string(&path).with_context(|| {
        format!("could not read seed file `{path}` (the working directory must contain `data/seed/`)")
    })?;
    serde_json::from_str(&data).with_context(|| format!("could not parse seed file `{path}`"))
}

pub fn load_all(conn: &Connection) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    let result = (|| {
        load_country_reference(conn)?;
        load_countries(conn)?;
        load_models(conn)?;
        load_deals(conn)?;
        load_signals(conn)?;
        load_services(conn)?;
        load_service_channels(conn)?;
        load_path_templates(conn)?;
        load_path_dependencies(conn)?;
        load_evidence_items(conn)?;
        load_path_evidence(conn)?;
        load_country_constraints(conn)?;
        load_constraint_evidence(conn)?;
        Ok::<_, anyhow::Error>(())
    })();
    match result {
        Ok(_) => {
            conn.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(e) => {
            conn.execute_batch("ROLLBACK;").ok();
            Err(e)
        }
    }
}

/// Loads the world country list. `INSERT OR REPLACE` rather than
/// `OR IGNORE` (used everywhere else here): this is generated reference data
/// with no hand edits to preserve, so a regenerated file should actually take
/// effect instead of being silently ignored on an existing database.
fn load_country_reference(conn: &Connection) -> Result<()> {
    let rows: Vec<CountryReference> = read_seed("country_reference.json")?;
    for r in rows {
        conn.execute(
            "INSERT OR REPLACE INTO country_reference VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            rusqlite::params![
                r.country_code,
                r.country_name,
                r.alpha3,
                r.iso_numeric,
                r.region,
                r.subregion,
                r.centroid_lat,
                r.centroid_lon,
                r.bbox_min_lon,
                r.bbox_min_lat,
                r.bbox_max_lon,
                r.bbox_max_lat,
                r.include_on_globe as i64,
                r.priority_tier,
            ],
        )?;
    }
    Ok(())
}

fn load_countries(conn: &Connection) -> Result<()> {
    let rows: Vec<Country> = read_seed("countries.json")?;
    for r in rows {
        conn.execute(
            "INSERT OR IGNORE INTO countries VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                r.country_code,
                r.country_name,
                serde_json::to_string(&r.sanctions_tier)?
                    .trim_matches('"')
                    .to_string(),
                r.us_service_access,
                r.cloud_access_notes,
                r.export_control_notes,
                r.last_major_legal_change,
                serde_json::to_string(&r.compute_capacity_band)?
                    .trim_matches('"')
                    .to_string(),
                r.confidence,
                r.source_notes,
            ],
        )?;
    }
    Ok(())
}

fn load_models(conn: &Connection) -> Result<()> {
    let rows: Vec<ModelRelease> = read_seed("models.json")?;
    for r in rows {
        conn.execute(
            "INSERT OR IGNORE INTO model_releases VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            rusqlite::params![
                r.model_id,
                r.provider,
                r.model_name,
                r.origin_country,
                r.release_date,
                serde_json::to_string(&r.weight_access)?
                    .trim_matches('"')
                    .to_string(),
                r.license_type,
                r.parameter_class,
                serde_json::to_string(&r.local_deployability)?
                    .trim_matches('"')
                    .to_string(),
                r.telemetry_risk_default,
                r.confidence,
                r.huggingface_repo_id,
                r.source_notes,
            ],
        )?;
    }
    Ok(())
}

fn load_deals(conn: &Connection) -> Result<()> {
    let rows: Vec<Deal> = read_seed("deals.json")?;
    for r in rows {
        conn.execute(
            "INSERT OR IGNORE INTO deals VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                r.deal_id,
                r.country_code,
                r.partner_type,
                r.partner_name,
                r.deal_date,
                r.deal_type,
                r.description,
                r.stack_layer,
                r.confidence,
                r.source_notes,
            ],
        )?;
    }
    Ok(())
}

fn load_signals(conn: &Connection) -> Result<()> {
    let rows: Vec<AdoptionSignal> = read_seed("signals.json")?;
    for r in rows {
        conn.execute(
            "INSERT OR IGNORE INTO adoption_signals VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                r.signal_id,
                r.country_code,
                r.provider,
                r.model_or_service,
                r.user_segment,
                r.signal_type,
                r.value_text,
                r.signal_date,
                r.confidence,
                r.source_notes,
            ],
        )?;
    }
    Ok(())
}

fn encode_status(status: Status) -> Result<String> {
    Ok(serde_json::to_string(&status)?
        .trim_matches('"')
        .to_string())
}

fn load_services(conn: &Connection) -> Result<()> {
    let rows = [
        (
            "openai",
            "OpenAI",
            "AI_ACCESS",
            Some("OpenAI"),
            "MANAGED_AI",
            "Managed frontier AI provider.",
        ),
        (
            "claude",
            "Claude",
            "AI_ACCESS",
            Some("Anthropic"),
            "MANAGED_AI",
            "Managed frontier AI provider.",
        ),
        (
            "huggingface",
            "Hugging Face",
            "AI_ACCESS",
            Some("Hugging Face"),
            "MODEL_DISTRIBUTION",
            "Model distribution and inference platform.",
        ),
        (
            "signal",
            "Signal",
            "CIRCUMVENTION",
            Some("Signal Foundation"),
            "SECURE_MESSAGING",
            "End-to-end encrypted messaging.",
        ),
        (
            "tor",
            "Tor",
            "CIRCUMVENTION",
            Some("Tor Project"),
            "CIRCUMVENTION",
            "Circumvention and anonymity network.",
        ),
    ];

    for (service_id, service_name, category, provider, stack_role, notes) in rows {
        conn.execute(
            "INSERT OR IGNORE INTO services VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![
                service_id,
                service_name,
                category,
                provider,
                stack_role,
                notes
            ],
        )?;
    }
    Ok(())
}

fn load_service_channels(conn: &Connection) -> Result<()> {
    let rows = [
        (
            "openai_api",
            "openai",
            "API",
            "OpenAI API",
            true,
            Some("MEDIUM"),
            "Managed API endpoint for OpenAI models.",
        ),
        (
            "claude_api",
            "claude",
            "API",
            "Claude API",
            true,
            Some("MEDIUM"),
            "Managed API endpoint for Claude models.",
        ),
        (
            "huggingface_weights",
            "huggingface",
            "WEIGHTS",
            "Hugging Face weights",
            true,
            Some("LOW"),
            "Published model weights and model cards.",
        ),
        (
            "signal",
            "signal",
            "APP",
            "Signal app and service",
            true,
            Some("LOW"),
            "Signal website and service distribution channel.",
        ),
        (
            "tor",
            "tor",
            "NETWORK",
            "Tor network access",
            true,
            Some("LOW"),
            "Tor entry and bridge access.",
        ),
    ];

    for (
        channel_id,
        service_id,
        channel_type,
        channel_name,
        internet_required,
        foreign_operator_risk,
        notes,
    ) in rows
    {
        conn.execute(
            "INSERT OR IGNORE INTO service_channels VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![
                channel_id,
                service_id,
                channel_type,
                channel_name,
                internet_required,
                foreign_operator_risk,
                notes
            ],
        )?;
    }
    Ok(())
}

fn load_path_templates(conn: &Connection) -> Result<()> {
    let rows = [
        (
            "local_open_weight",
            "Fully local open-weight deployment",
            "LOCAL",
            "Self-hosted open-weight model on controlled hardware.",
        ),
        (
            "local_small_model",
            "Local small model on workstation hardware",
            "LOCAL",
            "Small open-weight model deployable on workstation-class hardware.",
        ),
        (
            "sovereign_vpc_open_weight",
            "Sovereign VPC with open-weight model",
            "SOVEREIGN_VPC",
            "Open-weight model hosted in a country-controlled VPC.",
        ),
        (
            "us_frontier_api",
            "US hyperscaler cloud or closed frontier API",
            "MANAGED_CLOUD",
            "Managed US frontier API or hyperscaler path.",
        ),
        (
            "selective_cloud_fallback",
            "Selective cloud fallback for non-sensitive tasks",
            "HYBRID",
            "Selective managed cloud use for low-sensitivity workloads.",
        ),
        (
            "circumvention_mediated_access",
            "Circumvention-mediated managed access",
            "CIRCUMVENTION_MEDIATED",
            "Managed AI access path that depends on censorship-resilient connectivity.",
        ),
    ];

    for (path_id, label, architecture_family, description) in rows {
        conn.execute(
            "INSERT OR IGNORE INTO path_templates VALUES (?1,?2,?3,?4)",
            rusqlite::params![path_id, label, architecture_family, description],
        )?;
    }
    Ok(())
}

fn load_path_dependencies(conn: &Connection) -> Result<()> {
    let rows = [
        (
            "local_open_weight",
            "compute",
            "local_compute",
            true,
            Some("Controlled local inference hardware."),
        ),
        (
            "local_open_weight",
            "data",
            "model_weights_download",
            true,
            Some("Published weights must be retrievable."),
        ),
        (
            "local_open_weight",
            "operational",
            "operator_control",
            true,
            Some("Operator controls deployment boundary."),
        ),
        (
            "local_small_model",
            "compute",
            "workstation_compute",
            true,
            Some("Workstation-class local compute."),
        ),
        (
            "local_small_model",
            "data",
            "model_weights_download",
            true,
            Some("Published weights must be retrievable."),
        ),
        (
            "sovereign_vpc_open_weight",
            "infrastructure",
            "country_controlled_vpc",
            true,
            Some("Country-controlled virtual private cloud."),
        ),
        (
            "sovereign_vpc_open_weight",
            "data",
            "model_weights_download",
            true,
            Some("Published weights must be retrievable."),
        ),
        (
            "sovereign_vpc_open_weight",
            "network",
            "internet_connectivity",
            true,
            Some("Ongoing network reachability required."),
        ),
        (
            "us_frontier_api",
            "service_channel",
            "openai_api",
            true,
            Some("Managed API access."),
        ),
        (
            "us_frontier_api",
            "service_channel",
            "claude_api",
            false,
            Some("Alternative managed API access."),
        ),
        (
            "us_frontier_api",
            "payment",
            "cross_border_billing",
            true,
            Some("Cross-border payment rails must work."),
        ),
        (
            "selective_cloud_fallback",
            "service_channel",
            "openai_api",
            true,
            Some("Managed API used selectively."),
        ),
        (
            "selective_cloud_fallback",
            "payment",
            "cross_border_billing",
            true,
            Some("Cross-border payment rails must work."),
        ),
        (
            "circumvention_mediated_access",
            "service_channel",
            "signal",
            true,
            Some("Secure coordination channel for adaptive access."),
        ),
        (
            "circumvention_mediated_access",
            "service_channel",
            "tor",
            true,
            Some("Circumvention path for upstream access."),
        ),
        (
            "circumvention_mediated_access",
            "network",
            "adaptive_connectivity",
            true,
            Some("Connectivity depends on censorship conditions."),
        ),
    ];

    for (path_id, dependency_type, dependency_target, required, notes) in rows {
        conn.execute(
            "INSERT OR IGNORE INTO path_dependencies VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![path_id, dependency_type, dependency_target, required, notes],
        )?;
    }
    Ok(())
}

fn load_evidence_items(conn: &Connection) -> Result<()> {
    let rows = [
        (
            "country-ir-sanctions",
            "SEED",
            "Iran service restrictions",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "Iran remains under comprehensive sanctions that block managed US AI services.",
            "HIGH",
        ),
        (
            "weights-open-policy",
            "SEED",
            "Published open-weight access",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "Published model weights remain the clearest path for self-hosted access.",
            "HIGH",
        ),
        (
            "provider-terms-openai",
            "SEED",
            "OpenAI provider restrictions",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "Provider-level restrictions can block or caution managed API use by country.",
            "HIGH",
        ),
        (
            "provider-terms-anthropic",
            "SEED",
            "Anthropic provider restrictions",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "Provider-level restrictions can block or caution managed API use by country.",
            "HIGH",
        ),
        (
            "country-sy-transition",
            "SEED",
            "Syria post-sanctions transition",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "Formal legal relief does not automatically restore practical managed access.",
            "HIGH",
        ),
        (
            "country-ae-us-stack",
            "SEED",
            "UAE US compute stack integration",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "UAE has direct access to the US compute stack and sovereign hosting options.",
            "HIGH",
        ),
        (
            "country-sa-us-stack",
            "SEED",
            "Saudi US compute stack integration",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "Saudi Arabia has direct access to the US compute stack and sovereign hosting options.",
            "HIGH",
        ),
        (
            "country-iq-cost",
            "SEED",
            "Iraq cost and infrastructure constraints",
            Some("Seed dataset"),
            None::<String>,
            None::<String>,
            "Iraq faces practical cost and infrastructure constraints rather than comprehensive legal blocking.",
            "MEDIUM",
        ),
    ];

    for (evidence_id, source_type, title, publisher, url, observed_at, claim_text, confidence) in
        rows
    {
        conn.execute(
            "INSERT OR IGNORE INTO evidence_items VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                evidence_id,
                source_type,
                title,
                publisher,
                url,
                observed_at,
                claim_text,
                confidence
            ],
        )?;
    }
    Ok(())
}

fn load_path_evidence(conn: &Connection) -> Result<()> {
    let rows = [
        ("local_open_weight", "weights-open-policy"),
        ("local_small_model", "weights-open-policy"),
        ("sovereign_vpc_open_weight", "weights-open-policy"),
        ("us_frontier_api", "provider-terms-openai"),
        ("selective_cloud_fallback", "provider-terms-openai"),
        ("circumvention_mediated_access", "provider-terms-openai"),
    ];

    for (path_id, evidence_id) in rows {
        conn.execute(
            "INSERT OR IGNORE INTO path_evidence VALUES (?1,?2)",
            rusqlite::params![path_id, evidence_id],
        )?;
    }
    Ok(())
}

fn load_country_constraints(conn: &Connection) -> Result<()> {
    let rows = [
        (
            "openai-api-iran",
            "IR",
            "service_channel",
            "openai_api",
            "legal",
            Status::LegallyBlocked,
            "US-origin managed AI services unavailable under the current sanctions posture.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "claude-api-iran",
            "IR",
            "service_channel",
            "claude_api",
            "legal",
            Status::LegallyBlocked,
            "US-origin managed AI services unavailable under the current sanctions posture.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "openai-api-syria",
            "SY",
            "service_channel",
            "openai_api",
            "provider",
            Status::AllowedWithCaveats,
            "Legally unblocked, but provider availability and payment rails remain uneven.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "claude-api-syria",
            "SY",
            "service_channel",
            "claude_api",
            "provider",
            Status::AllowedWithCaveats,
            "Legally unblocked, but provider availability and payment rails remain uneven.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "openai-api-ae",
            "AE",
            "service_channel",
            "openai_api",
            "provider",
            Status::AllowedWithCaveats,
            "Managed US AI access is available, with sovereignty caveats for sensitive workloads.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "claude-api-ae",
            "AE",
            "service_channel",
            "claude_api",
            "provider",
            Status::AllowedWithCaveats,
            "Managed US AI access is available, with sovereignty caveats for sensitive workloads.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "openai-api-sa",
            "SA",
            "service_channel",
            "openai_api",
            "provider",
            Status::AllowedWithCaveats,
            "Managed US AI access is available, with sovereignty caveats for sensitive workloads.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "claude-api-sa",
            "SA",
            "service_channel",
            "claude_api",
            "provider",
            Status::AllowedWithCaveats,
            "Managed US AI access is available, with sovereignty caveats for sensitive workloads.",
            None::<String>,
            None::<String>,
            "HIGH",
        ),
        (
            "openai-api-iq",
            "IQ",
            "service_channel",
            "openai_api",
            "payment",
            Status::AllowedWithCaveats,
            "Managed API use is shaped more by payment and procurement friction than by country-program sanctions.",
            None::<String>,
            None::<String>,
            "MEDIUM",
        ),
        (
            "claude-api-iq",
            "IQ",
            "service_channel",
            "claude_api",
            "payment",
            Status::AllowedWithCaveats,
            "Managed API use is shaped more by payment and procurement friction than by country-program sanctions.",
            None::<String>,
            None::<String>,
            "MEDIUM",
        ),
        (
            "signal-network-ir",
            "IR",
            "service_channel",
            "signal",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "tor-network-ir",
            "IR",
            "service_channel",
            "tor",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "huggingface-network-ir",
            "IR",
            "service_channel",
            "huggingface_weights",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "signal-network-sy",
            "SY",
            "service_channel",
            "signal",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "tor-network-sy",
            "SY",
            "service_channel",
            "tor",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "huggingface-network-sy",
            "SY",
            "service_channel",
            "huggingface_weights",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "signal-network-ae",
            "AE",
            "service_channel",
            "signal",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "tor-network-ae",
            "AE",
            "service_channel",
            "tor",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "huggingface-network-ae",
            "AE",
            "service_channel",
            "huggingface_weights",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "signal-network-sa",
            "SA",
            "service_channel",
            "signal",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "tor-network-sa",
            "SA",
            "service_channel",
            "tor",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "huggingface-network-sa",
            "SA",
            "service_channel",
            "huggingface_weights",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "signal-network-iq",
            "IQ",
            "service_channel",
            "signal",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "tor-network-iq",
            "IQ",
            "service_channel",
            "tor",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
        (
            "huggingface-network-iq",
            "IQ",
            "service_channel",
            "huggingface_weights",
            "network",
            Status::Unknown,
            "Reachability is computed from measurement data.",
            None::<String>,
            None::<String>,
            "LOW",
        ),
    ];

    for (
        constraint_id,
        country_code,
        target_type,
        target_id,
        constraint_type,
        status,
        summary,
        applies_to_org_type,
        applies_to_sensitivity,
        confidence,
    ) in rows
    {
        conn.execute(
            "INSERT OR IGNORE INTO country_constraints VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                constraint_id,
                country_code,
                target_type,
                target_id,
                constraint_type,
                encode_status(status)?,
                summary,
                applies_to_org_type,
                applies_to_sensitivity,
                confidence,
            ],
        )?;
    }
    Ok(())
}

fn load_constraint_evidence(conn: &Connection) -> Result<()> {
    let rows = [
        ("openai-api-iran", "country-ir-sanctions"),
        ("openai-api-iran", "provider-terms-openai"),
        ("claude-api-iran", "country-ir-sanctions"),
        ("claude-api-iran", "provider-terms-anthropic"),
        ("openai-api-syria", "country-sy-transition"),
        ("openai-api-syria", "provider-terms-openai"),
        ("claude-api-syria", "country-sy-transition"),
        ("claude-api-syria", "provider-terms-anthropic"),
        ("openai-api-ae", "country-ae-us-stack"),
        ("claude-api-ae", "country-ae-us-stack"),
        ("openai-api-sa", "country-sa-us-stack"),
        ("claude-api-sa", "country-sa-us-stack"),
        ("openai-api-iq", "country-iq-cost"),
        ("claude-api-iq", "country-iq-cost"),
    ];

    for (constraint_id, evidence_id) in rows {
        conn.execute(
            "INSERT OR IGNORE INTO constraint_evidence VALUES (?1,?2)",
            rusqlite::params![constraint_id, evidence_id],
        )?;
    }
    Ok(())
}
