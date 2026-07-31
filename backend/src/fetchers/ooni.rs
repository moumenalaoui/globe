use crate::AppState;
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;

const MAX_RATE_LIMIT_RETRIES: u32 = 2;

// The OONI API rate-limits (HTTP 429) hard enough that firing requests
// concurrently just front-loads a wall of throttling. Every fetch is
// sequential and paced with this delay between requests — combined with
// get_with_retry's backoff as a safety net, this stays under the limit
// instead of tripping it and paying for backoff anyway. Each result is also
// inserted immediately after its own fetch (rather than batching fetch-all
// then insert-all) so a startup timeout only loses whatever hadn't been
// reached yet, not everything fetched so far.
const REQUEST_PACING: Duration = Duration::from_millis(2000);

const COUNTRY_CODES: [&str; 5] = ["IR", "SY", "AE", "SA", "IQ"];

const TARGET_URLS: [&str; 4] = [
    "https://api.openai.com",
    "https://openai.com",
    "https://api.anthropic.com",
    "https://www.deepseek.com",
];

const MEASUREMENTS_ENDPOINT: &str = "https://api.ooni.io/api/v1/measurements";
const AGGREGATION_ENDPOINT: &str = "https://api.ooni.io/api/v1/aggregation";

#[derive(Debug, Deserialize)]
struct MeasurementsResponse {
    #[serde(default)]
    results: Vec<Measurement>,
}

#[derive(Debug, Deserialize)]
struct Measurement {
    #[serde(default)]
    anomaly: Option<bool>,
    #[serde(default)]
    confirmed: Option<bool>,
}

// ── Technology registry ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TechCategory {
    AiAccess,
    Circumvention,
    PrivacyOs,
}

impl TechCategory {
    fn as_db_str(self) -> &'static str {
        match self {
            TechCategory::AiAccess => "AI_ACCESS",
            TechCategory::Circumvention => "CIRCUMVENTION",
            TechCategory::PrivacyOs => "PRIVACY_OS",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Technology {
    key: &'static str,
    category: TechCategory,
    // `None` for Tor, which is measured via the dedicated `tor` OONI test
    // rather than a `web_connectivity` probe against a URL.
    url: Option<&'static str>,
    test_name: &'static str,
}

static REGISTRY: &[Technology] = &[
    // AI access
    Technology {
        key: "openai.com",
        category: TechCategory::AiAccess,
        url: Some("https://openai.com"),
        test_name: "web_connectivity",
    },
    Technology {
        key: "claude.ai",
        category: TechCategory::AiAccess,
        url: Some("https://claude.ai"),
        test_name: "web_connectivity",
    },
    Technology {
        key: "deepseek",
        category: TechCategory::AiAccess,
        url: Some("https://www.deepseek.com"),
        test_name: "web_connectivity",
    },
    Technology {
        key: "huggingface",
        category: TechCategory::AiAccess,
        url: Some("https://huggingface.co"),
        test_name: "web_connectivity",
    },
    // Circumvention
    Technology {
        key: "tor",
        category: TechCategory::Circumvention,
        url: None,
        test_name: "tor",
    },
    Technology {
        key: "torproject",
        category: TechCategory::Circumvention,
        url: Some("https://www.torproject.org"),
        test_name: "web_connectivity",
    },
    Technology {
        key: "signal",
        category: TechCategory::Circumvention,
        url: Some("https://signal.org"),
        test_name: "web_connectivity",
    },
    Technology {
        key: "i2p",
        category: TechCategory::Circumvention,
        url: Some("https://geti2p.net"),
        test_name: "web_connectivity",
    },
    // Psiphon and Snowflake (torsf) are dedicated OONI nettests, like `tor`
    // above — they measure whether the tool itself works, not a URL.
    Technology {
        key: "psiphon",
        category: TechCategory::Circumvention,
        url: None,
        test_name: "psiphon",
    },
    Technology {
        key: "torsf",
        category: TechCategory::Circumvention,
        url: None,
        test_name: "torsf",
    },
    // Privacy-preserving OS
    Technology {
        key: "grapheneos",
        category: TechCategory::PrivacyOs,
        url: Some("https://grapheneos.org"),
        test_name: "web_connectivity",
    },
    Technology {
        key: "tails",
        category: TechCategory::PrivacyOs,
        url: Some("https://tails.boum.org"),
        test_name: "web_connectivity",
    },
];

// Country/technology pairs to backfill a daily blocking timeline for. Covers
// every tracked circumvention technology (torproject, signal, i2p, psiphon,
// torsf) across all 5 countries, so historical charts aren't lopsided toward
// Iran/Syria.
const TIMELINE_TARGETS: &[(&str, &str)] = &[
    ("IR", "torproject"),
    ("IR", "signal"),
    ("IR", "i2p"),
    ("IR", "psiphon"),
    ("IR", "torsf"),
    ("IR", "openai.com"),
    ("SY", "torproject"),
    ("SY", "signal"),
    ("SY", "i2p"),
    ("SY", "psiphon"),
    ("SY", "torsf"),
    ("AE", "torproject"),
    ("AE", "signal"),
    ("AE", "i2p"),
    ("AE", "psiphon"),
    ("AE", "torsf"),
    ("SA", "torproject"),
    ("SA", "signal"),
    ("SA", "i2p"),
    ("SA", "psiphon"),
    ("SA", "torsf"),
    ("IQ", "torproject"),
    ("IQ", "signal"),
    ("IQ", "i2p"),
    ("IQ", "psiphon"),
    ("IQ", "torsf"),
];

/// Runs the adoption-signal reachability sweep, the per-technology block
/// sweep, and the historical timeline backfill. Each phase is independent —
/// a failure in one does not prevent the others from running.
pub async fn fetch_and_store(state: &AppState) -> Result<()> {
    if let Err(e) = fetch_and_store_signals(state).await {
        eprintln!("ooni: adoption signal fetch failed: {e}");
    }
    if let Err(e) = fetch_and_store_technology_blocks(state).await {
        eprintln!("ooni: technology block fetch failed: {e}");
    }
    if let Err(e) = fetch_and_store_timeline(state).await {
        eprintln!("ooni: timeline fetch failed: {e}");
    }
    Ok(())
}

// ── Adoption signal reachability sweep (existing behavior) ────────────────

/// Queries the OONI Measurement Aggregation API for each (country, URL) pair
/// and records a reachability signal per pair in `adoption_signals`.
async fn fetch_and_store_signals(state: &AppState) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    for country in COUNTRY_CODES {
        for url in TARGET_URLS {
            match fetch_measurements(&client, country, url).await {
                Ok(measurements) => {
                    let (status, confidence) = classify(&measurements);
                    if let Err(e) =
                        insert_signal(state, country, url, status, confidence, measurements.len())
                    {
                        eprintln!("ooni: failed to store signal for {country} / {url}: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("ooni: failed to fetch measurements for {country} / {url}: {e}");
                }
            }
            tokio::time::sleep(REQUEST_PACING).await;
        }
    }
    Ok(())
}

/// Issues a GET, retrying on HTTP 429 with backoff (honoring `Retry-After`
/// when the API sends one). The OONI API rate-limits hard enough that
/// without this, most of a ~75-request sweep comes back empty.
async fn get_with_retry(
    client: &reqwest::Client,
    url: &str,
    query: &[(&str, &str)],
) -> Result<reqwest::Response> {
    let mut attempt = 0u32;
    loop {
        let resp = client.get(url).query(query).send().await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
            && attempt < MAX_RATE_LIMIT_RETRIES
        {
            let wait_secs = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or_else(|| 3u64 * 2u64.pow(attempt));
            tokio::time::sleep(Duration::from_secs(wait_secs.min(60))).await;
            attempt += 1;
            continue;
        }

        return Ok(resp.error_for_status()?);
    }
}

async fn fetch_measurements(
    client: &reqwest::Client,
    country: &str,
    url: &str,
) -> Result<Vec<Measurement>> {
    let resp = get_with_retry(
        client,
        MEASUREMENTS_ENDPOINT,
        &[
            ("probe_cc", country),
            ("input", url),
            ("test_name", "web_connectivity"),
            ("limit", "100"),
        ],
    )
    .await?;
    Ok(resp.json::<MeasurementsResponse>().await?.results)
}

/// Confirmed blocking on any measurement wins outright. Otherwise, any
/// anomaly without confirmation is treated as inconclusive rather than
/// accessible, since anomalies can stem from transient network issues.
fn classify(measurements: &[Measurement]) -> (&'static str, &'static str) {
    let total = measurements.len();
    let confirmed_count = measurements
        .iter()
        .filter(|m| m.confirmed == Some(true))
        .count();
    let anomaly_count = measurements
        .iter()
        .filter(|m| m.anomaly == Some(true))
        .count();

    let status = if confirmed_count > 0 {
        "BLOCKED"
    } else if total == 0 {
        "INCONCLUSIVE"
    } else if anomaly_count == 0 {
        "ACCESSIBLE"
    } else {
        "INCONCLUSIVE"
    };

    let confidence = if total >= 10 {
        "HIGH"
    } else if total >= 3 {
        "MEDIUM"
    } else {
        "LOW"
    };

    (status, confidence)
}

fn provider_for(url: &str) -> &'static str {
    match url {
        "https://api.openai.com" | "https://openai.com" => "OpenAI",
        "https://api.anthropic.com" => "Anthropic",
        "https://www.deepseek.com" => "DeepSeek",
        _ => "Unknown",
    }
}

fn insert_signal(
    state: &AppState,
    country: &str,
    url: &str,
    status: &str,
    confidence: &str,
    measurement_count: usize,
) -> Result<()> {
    let date = today_iso();
    let signal_id = format!("ooni-{country}-{}-{date}", slug(url));
    let value_text = format!(
        "{status}: {measurement_count} OONI web_connectivity measurement{} for {url} from {country}",
        if measurement_count == 1 { "" } else { "s" }
    );

    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    conn.execute(
        "INSERT OR REPLACE INTO adoption_signals VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            signal_id,
            country,
            provider_for(url),
            url,
            "NETWORK_MEASUREMENT",
            "OONI_MEASUREMENT",
            value_text,
            date,
            confidence,
            "OONI Measurement Aggregation API v1 (api.ooni.io/api/v1/measurements)",
        ],
    )?;
    Ok(())
}

fn slug(url: &str) -> String {
    url.replace("https://", "").replace(['/', '.'], "-")
}

// ── Per-technology block sweep ─────────────────────────────────────────────

/// Sweeps every (country, technology) pair — 5 countries x 10 technologies,
/// 50 requests — sequentially and paced (see REQUEST_PACING), inserting
/// each result as soon as its fetch completes.
async fn fetch_and_store_technology_blocks(state: &AppState) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    for country in COUNTRY_CODES {
        for tech in REGISTRY {
            match fetch_tech_measurements(&client, country, tech.url, tech.test_name).await {
                Ok(measurements) => {
                    let (status, rate) = classify_technology(&measurements);
                    if let Err(e) = insert_technology_block(
                        state,
                        country,
                        tech,
                        status,
                        rate,
                        measurements.len(),
                    ) {
                        eprintln!(
                            "ooni: failed to store technology block for {country} / {}: {e}",
                            tech.key
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "ooni: failed to fetch measurements for {country} / {}: {e}",
                        tech.key
                    );
                }
            }
            tokio::time::sleep(REQUEST_PACING).await;
        }
    }
    Ok(())
}

async fn fetch_tech_measurements(
    client: &reqwest::Client,
    country: &str,
    url: Option<&str>,
    test_name: &str,
) -> Result<Vec<Measurement>> {
    let mut query: Vec<(&str, &str)> = vec![
        ("probe_cc", country),
        ("test_name", test_name),
        ("limit", "50"),
    ];
    if let Some(u) = url {
        query.push(("input", u));
    }

    let resp = get_with_retry(client, MEASUREMENTS_ENDPOINT, &query).await?;
    Ok(resp.json::<MeasurementsResponse>().await?.results)
}

/// Classifies by anomaly rate + sample size rather than the `confirmed`
/// flag alone — OONI confirms blocking for very few test types, so relying
/// on it exclusively would under-report circumvention-tool blocking.
fn classify_technology(measurements: &[Measurement]) -> (&'static str, f64) {
    let total = measurements.len();
    if total == 0 {
        return ("INCONCLUSIVE", 0.0);
    }

    let anomaly_count = measurements
        .iter()
        .filter(|m| m.anomaly == Some(true))
        .count();
    let rate = anomaly_count as f64 / total as f64;

    let status = if rate > 0.7 && total > 10 {
        "CONFIRMED_BLOCKED"
    } else if rate > 0.4 && total > 5 {
        "LIKELY_BLOCKED"
    } else if rate < 0.1 && total > 5 {
        "ACCESSIBLE"
    } else {
        "INCONCLUSIVE"
    };

    (status, rate)
}

fn insert_technology_block(
    state: &AppState,
    country: &str,
    tech: &Technology,
    status: &str,
    anomaly_rate: f64,
    measurement_count: usize,
) -> Result<()> {
    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    conn.execute(
        "INSERT OR REPLACE INTO technology_blocks
         (country_code, category, technology, domain_or_url, test_name, status, anomaly_rate, measurement_count, checked_date, source_notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            country,
            tech.category.as_db_str(),
            tech.key,
            tech.url.unwrap_or(""),
            tech.test_name,
            status,
            anomaly_rate,
            measurement_count as i64,
            today_iso(),
            "OONI Measurement Aggregation API v1 (api.ooni.io/api/v1/measurements)",
        ],
    )?;
    Ok(())
}

// ── Historical blocking timeline backfill ──────────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct AggregationRow {
    #[serde(default)]
    measurement_start_day: String,
    #[serde(default)]
    anomaly_count: i64,
    #[serde(default)]
    confirmed_count: i64,
    #[serde(default)]
    failure_count: i64,
    #[serde(default)]
    ok_count: i64,
    #[serde(default)]
    measurement_count: i64,
}

#[derive(Debug, Deserialize)]
struct AggregationResponse {
    #[serde(default)]
    result: Vec<AggregationRow>,
}

async fn fetch_and_store_timeline(state: &AppState) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    for (country, tech_key) in TIMELINE_TARGETS {
        let Some(tech) = REGISTRY.iter().find(|t| t.key == *tech_key) else {
            continue;
        };
        // Tools tested via a dedicated OONI test (tor/psiphon/torsf) have no
        // `url`, and the aggregation API aggregates those by test_name alone
        // rather than by domain.
        let domain = tech.url.map(bare_domain);

        match fetch_aggregation(&client, country, domain.as_deref(), tech.test_name).await {
            Ok(rows) => {
                for row in &rows {
                    if row.measurement_start_day.is_empty() {
                        continue;
                    }
                    if let Err(e) = insert_timeline_row(state, country, tech_key, row) {
                        eprintln!(
                            "ooni: failed to store timeline row for {country} / {tech_key}: {e}"
                        );
                    }
                }
            }
            Err(e) => eprintln!("ooni: failed to fetch timeline for {country} / {tech_key}: {e}"),
        }
        tokio::time::sleep(REQUEST_PACING).await;
    }
    Ok(())
}

async fn fetch_aggregation(
    client: &reqwest::Client,
    country: &str,
    domain: Option<&str>,
    test_name: &str,
) -> Result<Vec<AggregationRow>> {
    let mut query = vec![
        ("probe_cc", country),
        ("test_name", test_name),
        ("axis_x", "measurement_start_day"),
        ("since", "2024-01-01"),
    ];
    if let Some(d) = domain {
        query.push(("domain", d));
    }

    let resp = get_with_retry(client, AGGREGATION_ENDPOINT, &query).await?;
    Ok(resp.json::<AggregationResponse>().await?.result)
}

fn insert_timeline_row(
    state: &AppState,
    country: &str,
    technology: &str,
    row: &AggregationRow,
) -> Result<()> {
    let total = if row.measurement_count > 0 {
        row.measurement_count
    } else {
        row.anomaly_count + row.confirmed_count + row.failure_count + row.ok_count
    };

    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    conn.execute(
        "INSERT OR REPLACE INTO blocking_timeline
         (country_code, technology, measurement_date, anomaly_count, confirmed_count, measurement_count, ok_count)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params![
            country,
            technology,
            row.measurement_start_day,
            row.anomaly_count,
            row.confirmed_count,
            total,
            row.ok_count,
        ],
    )?;
    Ok(())
}

fn bare_domain(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .to_string()
}

// ── Date helpers ────────────────────────────────────────────────────────────

fn today_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's days-since-epoch to civil-date algorithm
/// (http://howardhinnant.github.io/date_algorithms.html), used here to avoid
/// pulling in a date/time crate for a single "what's today's date" call.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
