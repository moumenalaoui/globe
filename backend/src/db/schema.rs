use anyhow::Result;
use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS countries (
            country_code             TEXT PRIMARY KEY,
            country_name             TEXT NOT NULL,
            sanctions_tier           TEXT NOT NULL,
            us_service_access        TEXT NOT NULL,
            cloud_access_notes       TEXT NOT NULL,
            export_control_notes     TEXT NOT NULL,
            last_major_legal_change  TEXT,
            compute_capacity_band    TEXT NOT NULL,
            confidence               TEXT NOT NULL,
            source_notes             TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS model_releases (
            model_id              TEXT PRIMARY KEY,
            provider              TEXT NOT NULL,
            model_name            TEXT NOT NULL,
            origin_country        TEXT NOT NULL,
            release_date          TEXT NOT NULL,
            weight_access         TEXT NOT NULL,
            license_type          TEXT NOT NULL,
            parameter_class       TEXT NOT NULL,
            local_deployability   TEXT NOT NULL,
            telemetry_risk_default TEXT NOT NULL,
            confidence            TEXT NOT NULL,
            huggingface_repo_id   TEXT,
            source_notes          TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS model_usage (
            model_id            TEXT PRIMARY KEY,
            downloads_all_time  INTEGER NOT NULL,
            checked_date        TEXT NOT NULL,
            source_notes        TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS deals (
            deal_id       TEXT PRIMARY KEY,
            country_code  TEXT NOT NULL,
            partner_type  TEXT NOT NULL,
            partner_name  TEXT NOT NULL,
            deal_date     TEXT NOT NULL,
            deal_type     TEXT NOT NULL,
            description   TEXT NOT NULL,
            stack_layer   TEXT NOT NULL,
            confidence    TEXT NOT NULL,
            source_notes  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS adoption_signals (
            signal_id        TEXT PRIMARY KEY,
            country_code     TEXT NOT NULL,
            provider         TEXT NOT NULL,
            model_or_service TEXT NOT NULL,
            user_segment     TEXT NOT NULL,
            signal_type      TEXT NOT NULL,
            value_text       TEXT NOT NULL,
            signal_date      TEXT NOT NULL,
            confidence       TEXT NOT NULL,
            source_notes     TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS technology_blocks (
            country_code      TEXT NOT NULL,
            category          TEXT NOT NULL,
            technology        TEXT NOT NULL,
            domain_or_url     TEXT NOT NULL,
            test_name         TEXT NOT NULL,
            status            TEXT NOT NULL,
            anomaly_rate      REAL NOT NULL,
            measurement_count INTEGER NOT NULL,
            checked_date      TEXT NOT NULL,
            source_notes      TEXT NOT NULL,
            PRIMARY KEY (country_code, technology)
        );

        CREATE TABLE IF NOT EXISTS blocking_timeline (
            country_code      TEXT NOT NULL,
            technology        TEXT NOT NULL,
            measurement_date  TEXT NOT NULL,
            anomaly_count     INTEGER NOT NULL,
            confirmed_count   INTEGER NOT NULL,
            measurement_count INTEGER NOT NULL,
            ok_count          INTEGER NOT NULL,
            PRIMARY KEY (country_code, technology, measurement_date)
        );

        CREATE TABLE IF NOT EXISTS tor_metrics (
            id TEXT PRIMARY KEY,
            country_code TEXT NOT NULL,
            date TEXT NOT NULL,
            relay_users INTEGER,
            bridge_users INTEGER,
            bridge_relay_ratio REAL,
            blocking_signal TEXT,
            source TEXT DEFAULT 'TOR_METRICS'
        );

        CREATE TABLE IF NOT EXISTS country_scores (
            id TEXT PRIMARY KEY,
            country_code TEXT NOT NULL,
            source TEXT NOT NULL,
            year INTEGER NOT NULL,
            score_overall REAL,
            score_access REAL,
            score_content REAL,
            score_rights REAL,
            classification TEXT,
            last_updated TEXT
        );

        CREATE TABLE IF NOT EXISTS services (
            service_id TEXT PRIMARY KEY,
            service_name TEXT NOT NULL,
            category TEXT NOT NULL,
            provider TEXT,
            stack_role TEXT NOT NULL,
            notes TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS service_channels (
            channel_id TEXT PRIMARY KEY,
            service_id TEXT NOT NULL,
            channel_type TEXT NOT NULL,
            channel_name TEXT NOT NULL,
            internet_required BOOLEAN NOT NULL,
            foreign_operator_risk TEXT,
            notes TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS path_templates (
            path_id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            architecture_family TEXT NOT NULL,
            description TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS path_dependencies (
            path_id TEXT NOT NULL,
            dependency_type TEXT NOT NULL,
            dependency_target TEXT NOT NULL,
            required BOOLEAN NOT NULL,
            notes TEXT,
            PRIMARY KEY (path_id, dependency_type, dependency_target)
        );

        CREATE TABLE IF NOT EXISTS country_constraints (
            constraint_id TEXT PRIMARY KEY,
            country_code TEXT NOT NULL,
            target_type TEXT NOT NULL,
            target_id TEXT NOT NULL,
            constraint_type TEXT NOT NULL,
            status TEXT NOT NULL,
            summary TEXT NOT NULL,
            applies_to_org_type TEXT,
            applies_to_sensitivity TEXT,
            confidence TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS evidence_items (
            evidence_id TEXT PRIMARY KEY,
            source_type TEXT NOT NULL,
            title TEXT NOT NULL,
            publisher TEXT,
            url TEXT,
            observed_at TEXT,
            claim_text TEXT NOT NULL,
            confidence TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS constraint_evidence (
            constraint_id TEXT NOT NULL,
            evidence_id TEXT NOT NULL,
            PRIMARY KEY (constraint_id, evidence_id)
        );

        CREATE TABLE IF NOT EXISTS path_evidence (
            path_id TEXT NOT NULL,
            evidence_id TEXT NOT NULL,
            PRIMARY KEY (path_id, evidence_id)
        );
    ",
    )?;
    Ok(())
}
