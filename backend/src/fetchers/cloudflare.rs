use crate::AppState;
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;

// The `/outages` sub-resource is the one that returns network-outage
// annotations in the shape parsed below. The bare `/radar/annotations` path
// rejects the request with HTTP 400 (code 2001) unless a date range is given.
const ANNOTATIONS_ENDPOINT: &str = "https://api.cloudflare.com/client/v4/radar/annotations/outages";

#[derive(Debug, Deserialize, Default)]
struct RadarResponse {
    #[serde(default)]
    result: RadarResult,
}

#[derive(Debug, Deserialize, Default)]
struct RadarResult {
    #[serde(default)]
    annotations: Vec<RadarEvent>,
}

#[derive(Debug, Deserialize, Default)]
struct RadarEvent {
    #[serde(default, rename = "startDate")]
    start_date: String,
    #[serde(default, rename = "endDate")]
    end_date: String,
    #[serde(default, rename = "eventType")]
    event_type: String,
    #[serde(default)]
    description: String,
}

/// Fetches Cloudflare Radar outage/traffic annotations per country and
/// records each as an adoption signal. Skips entirely (not an error) when no
/// API token is configured, since this integration is optional.
pub async fn fetch_and_store(state: &AppState) -> Result<()> {
    let token = match std::env::var("CLOUDFLARE_API_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            eprintln!("cloudflare: CLOUDFLARE_API_TOKEN not set, skipping");
            return Ok(());
        }
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    for country in &super::fetch_codes(state)? {
        match fetch_annotations(&client, &token, country).await {
            Ok(events) => {
                for event in events {
                    if event.start_date.is_empty() {
                        continue;
                    }
                    if let Err(e) = insert_cloudflare_event(state, country, &event) {
                        eprintln!("cloudflare: failed to store event for {country}: {e}");
                    }
                }
            }
            Err(e) => eprintln!("cloudflare: failed to fetch annotations for {country}: {e}"),
        }
    }

    Ok(())
}

async fn fetch_annotations(
    client: &reqwest::Client,
    token: &str,
    country: &str,
) -> Result<Vec<RadarEvent>> {
    let resp = client
        .get(ANNOTATIONS_ENDPOINT)
        .query(&[
            ("location", country),
            ("limit", "50"),
            // Required: Cloudflare returns HTTP 400 (code 2001, "must send
            // either range or start & end dates") without a time bound. A
            // one-year window captures recent outage annotations.
            ("dateRange", "52w"),
            ("format", "json"),
        ])
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<RadarResponse>()
        .await?;
    Ok(resp.result.annotations)
}

fn insert_cloudflare_event(state: &AppState, country: &str, event: &RadarEvent) -> Result<()> {
    let signal_id = format!("cloudflare-{country}-{}", event.start_date);
    let value_text = format!("{}: {}", event.event_type, event.description);

    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    conn.execute(
        "INSERT OR REPLACE INTO adoption_signals VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            signal_id,
            country,
            "Cloudflare Radar",
            event.event_type,
            "NETWORK_INFRASTRUCTURE",
            "CLOUDFLARE_OUTAGE_EVENT",
            value_text,
            event.start_date,
            "HIGH",
            format!(
                "Cloudflare Radar annotations API (api.cloudflare.com/client/v4/radar/annotations); ends {}",
                event.end_date
            ),
        ],
    )?;
    Ok(())
}
