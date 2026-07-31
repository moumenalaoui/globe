use crate::AppState;
use anyhow::{Result, bail};
use std::collections::HashMap;
use std::time::Duration;

const COUNTRY_CODES: [&str; 5] = ["IR", "SY", "AE", "SA", "IQ"];

// Tor Metrics exposes CSV downloads, not a REST/JSON API — there is no
// /api/1.0/ path. `events=off` on the relay endpoint matches the confirmed
// live URL format.
const RELAY_ENDPOINT: &str = "https://metrics.torproject.org/userstats-relay-country.csv";
const BRIDGE_ENDPOINT: &str = "https://metrics.torproject.org/userstats-bridge-country.csv";
const START_DATE: &str = "2024-01-01";
const END_DATE: &str = "2026-07-05";

const USER_AGENT: &str = "SCSP-MENA-AI-Workbench/1.0 (research; contact: research@scsp.ai)";

// A small, polite gap between the 10 requests (5 countries x relay+bridge)
// so we don't fire a burst at an API we have no rate-limit data on.
const REQUEST_PACING: Duration = Duration::from_millis(300);

struct RelayRow {
    date: String,
    users: i64,
    lower: Option<i64>,
}

struct BridgeRow {
    date: String,
    users: i64,
}

/// Fetches relay and bridge user-count CSVs per country from Tor Metrics,
/// joins them by date, and derives a blocking signal. `lower`/`upper` on the
/// relay CSV are Tor's own anomaly-detection bounds — a user count below
/// `lower` is a likely censorship event. When `lower` isn't published for a
/// given day, there's no basis for a HIGH_BLOCKING/LOW call, so that day is
/// marked INCONCLUSIVE rather than guessed at.
pub async fn fetch_and_store(state: &AppState) -> Result<()> {
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(USER_AGENT),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(default_headers)
        .build()?;

    for country in COUNTRY_CODES {
        let relay_rows = match fetch_relay_csv(&client, country).await {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("tor_metrics: failed to fetch relay CSV for {country}: {e}");
                Vec::new()
            }
        };
        tokio::time::sleep(REQUEST_PACING).await;

        let bridge_rows = match fetch_bridge_csv(&client, country).await {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("tor_metrics: failed to fetch bridge CSV for {country}: {e}");
                Vec::new()
            }
        };
        tokio::time::sleep(REQUEST_PACING).await;

        let relay_map: HashMap<String, (i64, Option<i64>)> = relay_rows
            .into_iter()
            .map(|r| (r.date, (r.users, r.lower)))
            .collect();
        let bridge_map: HashMap<String, i64> =
            bridge_rows.into_iter().map(|r| (r.date, r.users)).collect();

        let mut dates: Vec<String> = relay_map.keys().chain(bridge_map.keys()).cloned().collect();
        dates.sort();
        dates.dedup();

        for date in dates {
            let relay_entry = relay_map.get(&date);
            let relay_users = relay_entry.map(|(users, _)| *users);
            let lower = relay_entry.and_then(|(_, lower)| *lower);
            let bridge_users = bridge_map.get(&date).copied();

            let signal = classify_blocking(relay_users, lower);

            if let Err(e) =
                insert_tor_metric(state, country, &date, relay_users, bridge_users, signal)
            {
                eprintln!("tor_metrics: failed to store row for {country} / {date}: {e}");
            }
        }
    }

    Ok(())
}

async fn fetch_relay_csv(client: &reqwest::Client, country: &str) -> Result<Vec<RelayRow>> {
    let cc = country.to_lowercase();
    let url = format!("{RELAY_ENDPOINT}?start={START_DATE}&end={END_DATE}&country={cc}&events=off");
    let body = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // Columns: date,country,users,lower,upper,frac — read by header name
    // (not position), since the spec says column order may change. Comment
    // lines start with '#'.
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .comment(Some(b'#'))
        .from_reader(body.as_bytes());

    let headers = reader.headers()?.clone();
    let date_col = find_column(&headers, "date");
    let users_col = find_column(&headers, "users");
    let lower_col = find_column(&headers, "lower");

    let (Some(date_col), Some(users_col)) = (date_col, users_col) else {
        bail!("relay CSV missing required date/users columns");
    };

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let Some(date) = record
            .get(date_col)
            .map(str::trim)
            .filter(|d| !d.is_empty())
        else {
            continue;
        };
        let Some(users) = record.get(users_col).and_then(parse_optional_int) else {
            continue;
        };
        let lower = lower_col
            .and_then(|c| record.get(c))
            .and_then(parse_optional_int);
        rows.push(RelayRow {
            date: date.to_string(),
            users,
            lower,
        });
    }
    Ok(rows)
}

async fn fetch_bridge_csv(client: &reqwest::Client, country: &str) -> Result<Vec<BridgeRow>> {
    let cc = country.to_lowercase();
    let url = format!("{BRIDGE_ENDPOINT}?start={START_DATE}&end={END_DATE}&country={cc}");
    let body = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // Columns: date,country,users,frac
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .comment(Some(b'#'))
        .from_reader(body.as_bytes());

    let headers = reader.headers()?.clone();
    let date_col = find_column(&headers, "date");
    let users_col = find_column(&headers, "users");

    let (Some(date_col), Some(users_col)) = (date_col, users_col) else {
        bail!("bridge CSV missing required date/users columns");
    };

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let Some(date) = record
            .get(date_col)
            .map(str::trim)
            .filter(|d| !d.is_empty())
        else {
            continue;
        };
        let Some(users) = record.get(users_col).and_then(parse_optional_int) else {
            continue;
        };
        rows.push(BridgeRow {
            date: date.to_string(),
            users,
        });
    }
    Ok(rows)
}

fn find_column(headers: &csv::StringRecord, name: &str) -> Option<usize> {
    headers
        .iter()
        .position(|h| h.trim().eq_ignore_ascii_case(name))
}

/// `lower`/`upper` (and occasionally `users`) come through as empty strings
/// rather than absent fields when Tor has no anomaly-detection bound for
/// that day — those must read as None, not 0.
fn parse_optional_int(s: &str) -> Option<i64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok().map(|v| v.round() as i64)
}

fn classify_blocking(users: Option<i64>, lower: Option<i64>) -> &'static str {
    match (users, lower) {
        (Some(u), Some(l)) if u < l => "HIGH_BLOCKING",
        (Some(u), Some(l)) if (u as f64) < (l as f64) * 1.2 => "MODERATE",
        (Some(_), Some(_)) => "LOW",
        // No anomaly-detection bound published for this day — nothing to
        // compare against, so we can't call it blocked or not.
        _ => "INCONCLUSIVE",
    }
}

fn insert_tor_metric(
    state: &AppState,
    country: &str,
    date: &str,
    relay_users: Option<i64>,
    bridge_users: Option<i64>,
    signal: &str,
) -> Result<()> {
    let ratio = bridge_users.unwrap_or(0) as f64 / (relay_users.unwrap_or(0) as f64 + 1.0);
    let id = format!("{country}-{date}");
    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    conn.execute(
        "INSERT OR REPLACE INTO tor_metrics
         (id, country_code, date, relay_users, bridge_users, bridge_relay_ratio, blocking_signal, source)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        rusqlite::params![id, country, date, relay_users, bridge_users, ratio, signal, "TOR_METRICS"],
    )?;
    Ok(())
}

