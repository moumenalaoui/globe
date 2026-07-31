use std::collections::{HashMap, HashSet};

use anyhow::Result;
use rusqlite::Connection;

use crate::models::{
    country::{Country, SanctionsTier},
    deployment::{
        BaselineScenario, BlockedPath, ChangedConstraint, ChangedPath, ChangedRisk,
        ConstraintAssessment, CountryConstraintRow, DeploymentAssessment, DiffFromBaseline,
        EvalRequest, EvaluateResponse, EvidenceItem, PathAssessment, PathDependencyRef,
        PolicyLever, Risk, Scenario, SensitivityLevel, Status, Summary,
    },
};

pub struct SharedContractData {
    path_templates: HashMap<String, (String, String)>,
    path_dependencies: HashMap<String, Vec<PathDependencyRef>>,
    path_evidence: HashMap<String, Vec<String>>,
    constraints: Vec<ConstraintAssessment>,
}

pub fn load_shared(
    conn: &Connection,
    country_code: &str,
) -> Result<(SharedContractData, Vec<EvidenceItem>)> {
    let path_templates = load_path_templates(conn)?;
    let path_dependencies = load_path_dependencies(conn)?;
    let path_evidence = load_path_evidence(conn)?;
    let (constraints, evidence_items) = load_constraints(conn, country_code)?;

    Ok((
        SharedContractData {
            path_templates,
            path_dependencies,
            path_evidence,
            constraints,
        },
        evidence_items,
    ))
}

pub fn persist_evidence_items(conn: &Connection, items: &[EvidenceItem]) -> Result<()> {
    for item in items {
        conn.execute(
            "INSERT OR REPLACE INTO evidence_items
             (evidence_id, source_type, title, publisher, url, observed_at, claim_text, confidence)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                item.evidence_id,
                item.source_type,
                item.title,
                item.publisher,
                item.url,
                item.observed_at,
                item.claim_text,
                item.confidence,
            ],
        )?;
    }
    Ok(())
}

pub fn build_response(
    country: &Country,
    req: &EvalRequest,
    legacy: &DeploymentAssessment,
    shared: &SharedContractData,
) -> EvaluateResponse {
    let baseline_request = EvalRequest {
        country_code: country.country_code.clone(),
        org_type: crate::models::deployment::OrgType::Private,
        sensitivity: SensitivityLevel::Public,
    };
    let baseline_legacy = crate::engine::rules::evaluate(country, &baseline_request);

    let current = build_core(country, req, legacy, shared);
    let baseline = build_core(country, &baseline_request, &baseline_legacy, shared);
    let diff = diff_from_baseline(&baseline, &current, &baseline_request);

    EvaluateResponse {
        diff_from_baseline: diff,
        ..current
    }
}

pub fn collect_evidence_refs(response: &EvaluateResponse) -> Vec<String> {
    let mut refs = Vec::new();
    for path in &response.paths {
        refs.extend(path.evidence_refs.clone());
    }
    for constraint in &response.constraints {
        refs.extend(constraint.evidence_refs.clone());
    }
    dedupe(&mut refs);
    refs
}

pub fn load_evidence_items_by_ids(conn: &Connection, refs: &[String]) -> Result<Vec<EvidenceItem>> {
    let mut stmt = conn.prepare(
        "SELECT evidence_id, source_type, title, publisher, url, observed_at, claim_text, confidence
         FROM evidence_items",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(EvidenceItem {
            evidence_id: row.get(0)?,
            source_type: row.get(1)?,
            title: row.get(2)?,
            publisher: row.get(3)?,
            url: row.get(4)?,
            observed_at: row.get(5)?,
            claim_text: row.get(6)?,
            confidence: row.get(7)?,
        })
    })?;

    let wanted: HashSet<_> = refs.iter().cloned().collect();
    let mut items = Vec::new();
    for row in rows {
        let item = row?;
        if wanted.contains(&item.evidence_id) {
            items.push(item);
        }
    }
    items.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(items)
}

fn build_core(
    country: &Country,
    req: &EvalRequest,
    legacy: &DeploymentAssessment,
    shared: &SharedContractData,
) -> EvaluateResponse {
    let recommended_path_id = recommended_path_id(country, req, legacy);
    let mut path_map: HashMap<String, PathAssessment> = HashMap::new();

    for (index, path) in legacy.feasible_paths.iter().enumerate() {
        let new_path_id = map_legacy_path_id(&path.path_id);
        let status = if new_path_id == recommended_path_id {
            Status::Recommended
        } else if path.path_id == "us-cloud-cautionary" {
            Status::Fragile
        } else {
            Status::AllowedWithCaveats
        };
        let mut assessment = build_feasible_path(
            country,
            req,
            path,
            &legacy.privacy_warnings,
            &new_path_id,
            status,
            shared,
        );
        if index == 0 && assessment.why.is_empty() {
            assessment
                .why
                .push("Primary path retained from current country-tier logic.".to_string());
        }
        path_map.insert(new_path_id, assessment);
    }

    for blocked in &legacy.blocked_paths {
        let new_path_id = map_legacy_path_id(&blocked.path_id);
        if let Some(existing) = path_map.get_mut(&new_path_id) {
            existing.risks.push(blocked_to_risk(blocked));
            if !existing.why.iter().any(|w| w == &blocked.reason) {
                existing.why.push(blocked.reason.clone());
            }
            continue;
        }

        path_map.insert(
            new_path_id.clone(),
            build_blocked_path(blocked, &new_path_id, shared),
        );
    }

    if country.sanctions_tier != SanctionsTier::ComprehensivelySanctioned {
        let circumvention_status = shared
            .constraints
            .iter()
            .filter(|c| c.constraint_type == "network")
            .filter_map(|c| match c.status {
                Status::Fragile | Status::PracticallyUnavailable => Some(c.status),
                _ => None,
            })
            .next();

        if let Some(status) = circumvention_status {
            path_map.entry("circumvention_mediated_access".to_string()).or_insert_with(|| {
                let (label, description) = shared
                    .path_templates
                    .get("circumvention_mediated_access")
                    .cloned()
                    .unwrap_or_else(|| {
                        (
                            "Circumvention-mediated managed access".to_string(),
                            "Managed access path that depends on censorship-resilient connectivity.".to_string(),
                        )
                    });
                PathAssessment {
                    path_id: "circumvention_mediated_access".to_string(),
                    label,
                    status,
                    description,
                    why: vec![
                        "Managed access depends on active censorship conditions.".to_string(),
                        "Adaptive connectivity becomes a first-order dependency for this path.".to_string(),
                    ],
                    dependencies: shared
                        .path_dependencies
                        .get("circumvention_mediated_access")
                        .cloned()
                        .unwrap_or_default(),
                    risks: vec![Risk {
                        risk_type: "operational".to_string(),
                        severity: "MEDIUM".to_string(),
                        message: "Connectivity may degrade as infrastructure-level blocking conditions change.".to_string(),
                    }],
                    evidence_refs: shared
                        .constraints
                        .iter()
                        .filter(|c| c.constraint_type == "network")
                        .flat_map(|c| c.evidence_refs.clone())
                        .collect(),
                    recommended_models: Vec::new(),
                }
            });
        }
    }

    let mut paths: Vec<PathAssessment> = path_map.into_values().collect();
    paths.sort_by_key(|path| status_rank(path.status));

    EvaluateResponse {
        scenario: Scenario {
            country_code: country.country_code.clone(),
            org_type: req.org_type.clone(),
            sensitivity: req.sensitivity.clone(),
        },
        summary: Summary {
            recommended_path_id,
            overall_posture: overall_posture(country),
            confidence: legacy.confidence.clone(),
        },
        paths,
        constraints: shared.constraints.clone(),
        evidence_items: Vec::new(),
        policy_levers: policy_levers(country),
        decision_trace: decision_trace(country, req),
        diff_from_baseline: None,
    }
}

fn build_feasible_path(
    country: &Country,
    req: &EvalRequest,
    legacy: &crate::models::deployment::DeploymentPath,
    warnings: &[String],
    new_path_id: &str,
    status: Status,
    shared: &SharedContractData,
) -> PathAssessment {
    let (label, description) = shared
        .path_templates
        .get(new_path_id)
        .cloned()
        .unwrap_or_else(|| (legacy.label.clone(), legacy.description.clone()));

    let mut why = Vec::new();
    if !legacy.requires_internet {
        why.push("No external inference dependency.".to_string());
    }
    if new_path_id == "local_open_weight" {
        why.push("Published weights remain accessible.".to_string());
    }
    if status == Status::Recommended && req.sensitivity == SensitivityLevel::Classified {
        why.push("Best fit for classified workloads.".to_string());
    }
    if new_path_id == "us_frontier_api" && country.sanctions_tier == SanctionsTier::UsComputeStack {
        why.push("Managed US AI access is available in this country tier.".to_string());
    }
    if new_path_id == "selective_cloud_fallback" {
        why.push("Reserved for non-sensitive tasks where local capacity is limited.".to_string());
    }

    let risks = risks_for_path(
        new_path_id,
        warnings,
        &legacy.requires_internet,
        &legacy.foreign_operator_risk,
        country,
    );
    let mut evidence_refs = shared
        .path_evidence
        .get(new_path_id)
        .cloned()
        .unwrap_or_default();
    evidence_refs.extend(country_specific_path_evidence(country, new_path_id));
    dedupe(&mut evidence_refs);

    PathAssessment {
        path_id: new_path_id.to_string(),
        label,
        status,
        description,
        why,
        dependencies: shared
            .path_dependencies
            .get(new_path_id)
            .cloned()
            .unwrap_or_default(),
        risks,
        evidence_refs,
        recommended_models: legacy.recommended_models.clone(),
    }
}

fn build_blocked_path(
    blocked: &BlockedPath,
    new_path_id: &str,
    shared: &SharedContractData,
) -> PathAssessment {
    let status = blocked_status(blocked);
    let (label, description) = shared
        .path_templates
        .get(new_path_id)
        .cloned()
        .unwrap_or_else(|| (blocked.label.clone(), blocked.reason.clone()));

    PathAssessment {
        path_id: new_path_id.to_string(),
        label,
        status,
        description,
        why: vec![blocked.reason.clone()],
        dependencies: shared
            .path_dependencies
            .get(new_path_id)
            .cloned()
            .unwrap_or_default(),
        risks: vec![blocked_to_risk(blocked)],
        evidence_refs: Vec::new(),
        recommended_models: Vec::new(),
    }
}

fn risks_for_path(
    new_path_id: &str,
    warnings: &[String],
    requires_internet: &bool,
    foreign_operator_risk: &str,
    country: &Country,
) -> Vec<Risk> {
    let mut risks = Vec::new();

    if *requires_internet {
        risks.push(Risk {
            risk_type: "network".to_string(),
            severity: "MEDIUM".to_string(),
            message:
                "This path depends on ongoing network reachability for inference or service access."
                    .to_string(),
        });
    }

    if foreign_operator_risk != "NONE" {
        risks.push(Risk {
            risk_type: "privacy".to_string(),
            severity: "MEDIUM".to_string(),
            message: format!(
                "Foreign operator exposure remains {} for this path.",
                foreign_operator_risk.to_lowercase().replace('_', " ")
            ),
        });
    }

    if new_path_id == "local_open_weight" || new_path_id == "local_small_model" {
        risks.push(Risk {
            risk_type: "operational".to_string(),
            severity: "MEDIUM".to_string(),
            message: "Requires local ops capacity and hardware stewardship.".to_string(),
        });
    }

    if country.sanctions_tier == SanctionsTier::PostSanctionsLag && new_path_id == "us_frontier_api"
    {
        risks.push(Risk {
            risk_type: "provider".to_string(),
            severity: "MEDIUM".to_string(),
            message: "Provider policy and payment updates may lag formal legal relief.".to_string(),
        });
    }

    for warning in warnings {
        if applies_to_path(warning, new_path_id) {
            risks.push(warning_to_risk(warning));
        }
    }

    dedupe_risks(&mut risks);
    risks
}

fn applies_to_path(warning: &str, new_path_id: &str) -> bool {
    let lower = warning.to_lowercase();
    match new_path_id {
        "local_open_weight" | "local_small_model" => {
            lower.contains("local") || lower.contains("offline") || lower.contains("hardware")
        }
        "sovereign_vpc_open_weight" => {
            lower.contains("sovereign") || lower.contains("vpc") || lower.contains("cloud")
        }
        "us_frontier_api" | "selective_cloud_fallback" => {
            lower.contains("cloud")
                || lower.contains("api")
                || lower.contains("provider")
                || lower.contains("foreign")
        }
        "circumvention_mediated_access" => {
            lower.contains("geo-blocking")
                || lower.contains("infrastructure")
                || lower.contains("connectivity")
        }
        _ => false,
    }
}

fn warning_to_risk(warning: &str) -> Risk {
    let lower = warning.to_lowercase();
    let risk_type = if lower.contains("cloud") || lower.contains("provider") {
        "provider"
    } else if lower.contains("legal") || lower.contains("ofac") || lower.contains("ear") {
        "legal"
    } else if lower.contains("network")
        || lower.contains("connect")
        || lower.contains("geo-blocking")
    {
        "network"
    } else {
        "operational"
    };

    let severity = if lower.contains("classified") || lower.contains("blocked") {
        "HIGH"
    } else {
        "MEDIUM"
    };

    Risk {
        risk_type: risk_type.to_string(),
        severity: severity.to_string(),
        message: warning.to_string(),
    }
}

fn load_path_templates(conn: &Connection) -> Result<HashMap<String, (String, String)>> {
    let mut stmt = conn.prepare("SELECT path_id, label, description FROM path_templates")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, (row.get(1)?, row.get(2)?)))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (path_id, data) = row?;
        map.insert(path_id, data);
    }
    Ok(map)
}

fn load_path_dependencies(conn: &Connection) -> Result<HashMap<String, Vec<PathDependencyRef>>> {
    let mut stmt = conn.prepare(
        "SELECT path_id, dependency_type, dependency_target, required
         FROM path_dependencies ORDER BY path_id, dependency_type, dependency_target",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            PathDependencyRef {
                dependency_type: row.get(1)?,
                dependency_target: row.get(2)?,
                required: row.get::<_, bool>(3)?,
            },
        ))
    })?;
    let mut map: HashMap<String, Vec<PathDependencyRef>> = HashMap::new();
    for row in rows {
        let (path_id, dependency) = row?;
        map.entry(path_id).or_default().push(dependency);
    }
    Ok(map)
}

fn load_path_evidence(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut stmt = conn
        .prepare("SELECT path_id, evidence_id FROM path_evidence ORDER BY path_id, evidence_id")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (path_id, evidence_id) = row?;
        map.entry(path_id).or_default().push(evidence_id);
    }
    Ok(map)
}

fn load_constraints(
    conn: &Connection,
    country_code: &str,
) -> Result<(Vec<ConstraintAssessment>, Vec<EvidenceItem>)> {
    let mut stmt = conn.prepare(
        "SELECT constraint_id, country_code, target_type, target_id, constraint_type,
                status, summary, applies_to_org_type, applies_to_sensitivity, confidence
         FROM country_constraints WHERE country_code = ?1 ORDER BY constraint_type, constraint_id",
    )?;
    let rows = stmt.query_map(rusqlite::params![country_code], |row| {
        Ok(CountryConstraintRow {
            constraint_id: row.get(0)?,
            country_code: row.get(1)?,
            target_type: row.get(2)?,
            target_id: row.get(3)?,
            constraint_type: row.get(4)?,
            status: decode_status(row.get::<_, String>(5)?),
            summary: row.get(6)?,
            applies_to_org_type: row.get(7)?,
            applies_to_sensitivity: row.get(8)?,
            confidence: row.get(9)?,
        })
    })?;

    let evidence_map = load_constraint_evidence(conn)?;
    let mut constraints = Vec::new();
    let mut evidence_items = Vec::new();

    for row in rows {
        let row = row?;
        if row.constraint_type == "network" {
            let (constraint, items) = compute_network_constraint(conn, &row)?;
            constraints.push(constraint);
            evidence_items.extend(items);
        } else {
            constraints.push(ConstraintAssessment {
                constraint_id: row.constraint_id.clone(),
                constraint_type: row.constraint_type,
                status: row.status,
                target_type: row.target_type,
                target_id: row.target_id,
                summary: row.summary,
                confidence: row.confidence,
                evidence_refs: evidence_map
                    .get(&row.constraint_id)
                    .cloned()
                    .unwrap_or_default(),
            });
        }
    }

    constraints.sort_by_key(|constraint| status_rank(constraint.status));
    Ok((constraints, evidence_items))
}

fn load_constraint_evidence(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut stmt = conn.prepare("SELECT constraint_id, evidence_id FROM constraint_evidence ORDER BY constraint_id, evidence_id")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (constraint_id, evidence_id) = row?;
        map.entry(constraint_id).or_default().push(evidence_id);
    }
    Ok(map)
}

fn compute_network_constraint(
    conn: &Connection,
    row: &CountryConstraintRow,
) -> Result<(ConstraintAssessment, Vec<EvidenceItem>)> {
    let technology = target_technology(&row.target_id);
    let mut evidence_refs = Vec::new();
    let mut evidence_items = Vec::new();

    let mut tor_stmt = conn.prepare(
        "SELECT id, date, blocking_signal, relay_users, bridge_users
         FROM tor_metrics
         WHERE country_code = ?1 AND blocking_signal = 'HIGH_BLOCKING'
           AND date >= date('now', '-12 months')
         ORDER BY date DESC",
    )?;
    let tor_rows = tor_stmt.query_map(rusqlite::params![row.country_code], |value| {
        Ok((
            value.get::<_, String>(0)?,
            value.get::<_, String>(1)?,
            value.get::<_, String>(2)?,
            value.get::<_, Option<i64>>(3)?,
            value.get::<_, Option<i64>>(4)?,
        ))
    })?;
    let tor_rows: Vec<_> = tor_rows.filter_map(|item| item.ok()).collect();
    if matches!(row.target_id.as_str(), "tor")
        && let Some((_id, date, signal, relay_users, bridge_users)) = tor_rows.first()
    {
        let evidence_id = format!("tor-metrics-{}-{}", row.country_code.to_lowercase(), date);
        evidence_refs.push(evidence_id.clone());
        evidence_items.push(EvidenceItem {
            evidence_id,
            source_type: "TOR_METRICS".to_string(),
            title: format!("Tor metrics {} {}", row.country_code, date),
            publisher: Some("Tor Metrics".to_string()),
            url: None,
            observed_at: Some(date.clone()),
            claim_text: format!(
                "{} showed {} for {} with relay_users={:?} and bridge_users={:?}.",
                row.country_code, signal, date, relay_users, bridge_users
            ),
            confidence: "HIGH".to_string(),
        });
        return Ok((
            ConstraintAssessment {
                constraint_id: row.constraint_id.clone(),
                constraint_type: row.constraint_type.clone(),
                status: Status::Fragile,
                target_type: row.target_type.clone(),
                target_id: row.target_id.clone(),
                summary: "Reachability depends on active infrastructure-level blocking conditions."
                    .to_string(),
                confidence: "HIGH".to_string(),
                evidence_refs,
            },
            evidence_items,
        ));
    }

    let mut timeline_stmt = conn.prepare(
        "SELECT measurement_date, confirmed_count, anomaly_count, measurement_count, ok_count
         FROM blocking_timeline
         WHERE country_code = ?1 AND technology = ?2 AND measurement_date >= date('now', '-90 days')
         ORDER BY measurement_date DESC",
    )?;
    let timeline_rows =
        timeline_stmt.query_map(rusqlite::params![row.country_code, technology], |value| {
            Ok((
                value.get::<_, String>(0)?,
                value.get::<_, i64>(1)?,
                value.get::<_, i64>(2)?,
                value.get::<_, i64>(3)?,
                value.get::<_, i64>(4)?,
            ))
        })?;
    let timeline_rows: Vec<_> = timeline_rows.filter_map(|item| item.ok()).collect();
    let fragile_timeline: Vec<_> = timeline_rows
        .iter()
        .filter(|(_, confirmed_count, _, _, _)| *confirmed_count >= 2)
        .collect();
    if !fragile_timeline.is_empty() {
        for (date, confirmed_count, anomaly_count, measurement_count, ok_count) in
            fragile_timeline.iter().take(3)
        {
            let evidence_id = format!(
                "blocking-timeline-{}-{}-{}",
                row.country_code.to_lowercase(),
                technology.replace('.', "-"),
                date
            );
            evidence_refs.push(evidence_id.clone());
            evidence_items.push(EvidenceItem {
                evidence_id,
                source_type: "BLOCKING_TIMELINE".to_string(),
                title: format!("Blocking timeline {} {}", technology, date),
                publisher: Some("OONI".to_string()),
                url: None,
                observed_at: Some(date.clone()),
                claim_text: format!(
                    "{} on {} had confirmed_count={}, anomaly_count={}, measurement_count={}, ok_count={}.",
                    technology, date, confirmed_count, anomaly_count, measurement_count, ok_count
                ),
                confidence: "HIGH".to_string(),
            });
        }
        return Ok((
            ConstraintAssessment {
                constraint_id: row.constraint_id.clone(),
                constraint_type: row.constraint_type.clone(),
                status: Status::Fragile,
                target_type: row.target_type.clone(),
                target_id: row.target_id.clone(),
                summary: "Reachability depends on recent confirmed blocking activity in measurement data.".to_string(),
                confidence: "HIGH".to_string(),
                evidence_refs,
            },
            evidence_items,
        ));
    }

    let mut block_stmt = conn.prepare(
        "SELECT status, anomaly_rate, measurement_count, checked_date
         FROM technology_blocks WHERE country_code = ?1 AND technology = ?2",
    )?;
    let tech_row = block_stmt.query_row(rusqlite::params![row.country_code, technology], |value| {
        Ok((
            value.get::<_, String>(0)?,
            value.get::<_, f64>(1)?,
            value.get::<_, i64>(2)?,
            value.get::<_, String>(3)?,
        ))
    });

    match tech_row {
        Ok((status, anomaly_rate, measurement_count, checked_date)) => {
            let evidence_id = format!(
                "tech-block-{}-{}-{}",
                row.country_code.to_lowercase(),
                technology.replace('.', "-"),
                checked_date
            );
            evidence_refs.push(evidence_id.clone());
            evidence_items.push(EvidenceItem {
                evidence_id,
                source_type: "TECHNOLOGY_BLOCK".to_string(),
                title: format!("Technology block {} {}", technology, checked_date),
                publisher: Some("OONI".to_string()),
                url: None,
                observed_at: Some(checked_date),
                claim_text: format!(
                    "{} measured status {} with anomaly_rate={} and measurement_count={}",
                    technology, status, anomaly_rate, measurement_count
                ),
                confidence: if measurement_count == 0 {
                    "LOW"
                } else if measurement_count > 5 {
                    "HIGH"
                } else {
                    "MEDIUM"
                }
                .to_string(),
            });

            let (computed_status, confidence, summary) = if measurement_count == 0 {
                (
                    Status::Unknown,
                    "LOW".to_string(),
                    "No recent successful probe data exists for this channel; reachability remains unknown.".to_string(),
                )
            } else {
                match status.as_str() {
                    "CONFIRMED_BLOCKED" => (
                        Status::PracticallyUnavailable,
                        "HIGH".to_string(),
                        "Measurement data indicates this channel is currently not dependable in practice.".to_string(),
                    ),
                    "LIKELY_BLOCKED" => (
                        Status::Fragile,
                        "MEDIUM".to_string(),
                        "Measurement data indicates this channel is vulnerable to active blocking.".to_string(),
                    ),
                    "INCONCLUSIVE" => (
                        Status::Fragile,
                        "MEDIUM".to_string(),
                        "Reachability depends on unstable infrastructure conditions and inconclusive measurements.".to_string(),
                    ),
                    "ACCESSIBLE" => (
                        Status::AllowedWithCaveats,
                        "MEDIUM".to_string(),
                        "Recent measurements show reachability, but continued access still depends on infrastructure conditions.".to_string(),
                    ),
                    _ => (
                        Status::Unknown,
                        "LOW".to_string(),
                        "Measurement data is insufficient to classify this channel.".to_string(),
                    ),
                }
            };

            Ok((
                ConstraintAssessment {
                    constraint_id: row.constraint_id.clone(),
                    constraint_type: row.constraint_type.clone(),
                    status: computed_status,
                    target_type: row.target_type.clone(),
                    target_id: row.target_id.clone(),
                    summary,
                    confidence,
                    evidence_refs,
                },
                evidence_items,
            ))
        }
        Err(_) => Ok((
            ConstraintAssessment {
                constraint_id: row.constraint_id.clone(),
                constraint_type: row.constraint_type.clone(),
                status: Status::Unknown,
                target_type: row.target_type.clone(),
                target_id: row.target_id.clone(),
                summary: "No measurement row was available for this channel.".to_string(),
                confidence: "LOW".to_string(),
                evidence_refs,
            },
            evidence_items,
        )),
    }
}

fn diff_from_baseline(
    baseline: &EvaluateResponse,
    current: &EvaluateResponse,
    baseline_request: &EvalRequest,
) -> Option<DiffFromBaseline> {
    let mut changed_paths = Vec::new();
    let baseline_paths: HashMap<_, _> = baseline
        .paths
        .iter()
        .map(|path| (path.path_id.clone(), path.status))
        .collect();
    for path in &current.paths {
        if let Some(previous) = baseline_paths.get(&path.path_id) {
            if previous != &path.status {
                changed_paths.push(ChangedPath {
                    path_id: path.path_id.clone(),
                    from: *previous,
                    to: path.status,
                });
            }
        }
    }

    let mut changed_constraints = Vec::new();
    let baseline_constraints: HashMap<_, _> = baseline
        .constraints
        .iter()
        .map(|constraint| (constraint.constraint_id.clone(), constraint.status))
        .collect();
    for constraint in &current.constraints {
        if let Some(previous) = baseline_constraints.get(&constraint.constraint_id) {
            if previous != &constraint.status {
                changed_constraints.push(ChangedConstraint {
                    constraint_id: constraint.constraint_id.clone(),
                    from: *previous,
                    to: constraint.status,
                });
            }
        }
    }

    let baseline_risks: HashSet<_> = baseline
        .paths
        .iter()
        .flat_map(|path| path.risks.iter().map(|risk| risk.message.clone()))
        .collect();
    let current_risks: HashSet<_> = current
        .paths
        .iter()
        .flat_map(|path| path.risks.iter().map(|risk| risk.message.clone()))
        .collect();
    let mut changed_risks = Vec::new();
    for message in baseline_risks.union(&current_risks) {
        let before_present = baseline_risks.contains(message);
        let after_present = current_risks.contains(message);
        if before_present != after_present {
            changed_risks.push(ChangedRisk {
                message: message.clone(),
                before_present,
                after_present,
            });
        }
    }

    if changed_paths.is_empty() && changed_constraints.is_empty() && changed_risks.is_empty() {
        None
    } else {
        Some(DiffFromBaseline {
            baseline: BaselineScenario {
                org_type: baseline_request.org_type.clone(),
                sensitivity: baseline_request.sensitivity.clone(),
            },
            changed_paths,
            changed_constraints,
            changed_risks,
        })
    }
}

fn decode_status(value: String) -> Status {
    serde_json::from_str(&format!("\"{}\"", value)).unwrap_or(Status::Unknown)
}

fn blocked_status(blocked: &BlockedPath) -> Status {
    if blocked.legal_basis.starts_with("No legal constraint") {
        Status::PracticallyUnavailable
    } else if blocked.reason.contains("Legally unblocked") || blocked.reason.contains("unreliable")
    {
        Status::Fragile
    } else {
        Status::LegallyBlocked
    }
}

fn blocked_to_risk(blocked: &BlockedPath) -> Risk {
    Risk {
        risk_type: if blocked_status(blocked) == Status::LegallyBlocked {
            "legal"
        } else {
            "operational"
        }
        .to_string(),
        severity: "HIGH".to_string(),
        message: blocked.reason.clone(),
    }
}

fn map_legacy_path_id(path_id: &str) -> String {
    match path_id {
        "local-open-weight" => "local_open_weight",
        "local-small-model" => "local_small_model",
        "sovereign-vpc" => "sovereign_vpc_open_weight",
        "us-cloud"
        | "us-cloud-cautionary"
        | "closed-model-api"
        | "frontier-cloud-reliable"
        | "frontier-cloud-at-scale" => "us_frontier_api",
        "selective-cloud-fallback" => "selective_cloud_fallback",
        _ => path_id,
    }
    .to_string()
}

fn recommended_path_id(
    country: &Country,
    req: &EvalRequest,
    legacy: &DeploymentAssessment,
) -> String {
    match country.sanctions_tier {
        SanctionsTier::ComprehensivelySanctioned => "local_open_weight".to_string(),
        SanctionsTier::PostSanctionsLag => {
            if matches!(
                req.sensitivity,
                SensitivityLevel::Public | SensitivityLevel::Internal
            ) && legacy
                .feasible_paths
                .iter()
                .any(|path| path.path_id == "sovereign-vpc")
            {
                "sovereign_vpc_open_weight".to_string()
            } else {
                "local_open_weight".to_string()
            }
        }
        SanctionsTier::UsComputeStack => {
            if req.org_type == crate::models::deployment::OrgType::Government
                && req.sensitivity == SensitivityLevel::Classified
            {
                "sovereign_vpc_open_weight".to_string()
            } else if matches!(
                req.sensitivity,
                SensitivityLevel::Public | SensitivityLevel::Internal
            ) {
                "us_frontier_api".to_string()
            } else {
                "sovereign_vpc_open_weight".to_string()
            }
        }
        SanctionsTier::ResourceConstrained => "local_small_model".to_string(),
    }
}

fn overall_posture(country: &Country) -> String {
    match country.sanctions_tier {
        SanctionsTier::ComprehensivelySanctioned => "HIGHLY_CONSTRAINED".to_string(),
        SanctionsTier::PostSanctionsLag => "TRANSITIONAL".to_string(),
        SanctionsTier::UsComputeStack => "MANAGED_ACCESS_AVAILABLE".to_string(),
        SanctionsTier::ResourceConstrained => "COST_CONSTRAINED".to_string(),
    }
}

fn country_specific_path_evidence(country: &Country, path_id: &str) -> Vec<String> {
    match (country.country_code.as_str(), path_id) {
        ("IR", "local_open_weight") => vec!["country-ir-sanctions".to_string()],
        ("SY", "local_open_weight") | ("SY", "sovereign_vpc_open_weight") => {
            vec!["country-sy-transition".to_string()]
        }
        ("AE", "us_frontier_api") | ("AE", "sovereign_vpc_open_weight") => {
            vec!["country-ae-us-stack".to_string()]
        }
        ("SA", "us_frontier_api") | ("SA", "sovereign_vpc_open_weight") => {
            vec!["country-sa-us-stack".to_string()]
        }
        ("IQ", "local_small_model") => vec!["country-iq-cost".to_string()],
        _ => Vec::new(),
    }
}

fn target_technology(target_id: &str) -> &'static str {
    match target_id {
        "openai_api" => "openai.com",
        "claude_api" => "claude.ai",
        "huggingface_weights" => "huggingface",
        "signal" => "signal",
        "tor" => "tor",
        _ => "openai.com",
    }
}

fn decision_trace(country: &Country, req: &EvalRequest) -> Vec<String> {
    let mut trace = vec![format!("Country tier is {:?}", country.sanctions_tier)];
    trace.push(format!(
        "Scenario is {:?} + {:?}",
        req.org_type, req.sensitivity
    ));
    if req.sensitivity == SensitivityLevel::Classified {
        trace.push(
            "External network dependence is disfavored for classified workloads.".to_string(),
        );
    }
    if country.sanctions_tier == SanctionsTier::ComprehensivelySanctioned {
        trace.push(
            "Managed US paths remain legally blocked under the current country posture."
                .to_string(),
        );
    }
    trace
}

fn policy_levers(country: &Country) -> Vec<PolicyLever> {
    match country.country_code.as_str() {
        "IR" => vec![PolicyLever {
            lever: "Clarify generative AI under communications carve-outs".to_string(),
            effect: "Could move some managed AI channels from legally blocked to allowed-with-caveats.".to_string(),
            affects_constraint_ids: vec!["openai-api-iran".to_string(), "claude-api-iran".to_string()],
        }],
        "SY" => vec![PolicyLever {
            lever: "Press providers to align post-revocation policies".to_string(),
            effect: "Could move some managed AI channels from allowed-with-caveats to more dependable access.".to_string(),
            affects_constraint_ids: vec!["openai-api-syria".to_string(), "claude-api-syria".to_string()],
        }],
        _ => Vec::new(),
    }
}

fn status_rank(status: Status) -> usize {
    match status {
        Status::Recommended => 0,
        Status::AllowedWithCaveats => 1,
        Status::Fragile => 2,
        Status::Unknown => 3,
        Status::PracticallyUnavailable => 4,
        Status::LegallyBlocked => 5,
    }
}

fn dedupe(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn dedupe_risks(risks: &mut Vec<Risk>) {
    let mut seen = HashSet::new();
    risks.retain(|risk| seen.insert(risk.message.clone()));
}
