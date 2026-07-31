pub mod countries;
pub mod schema;
pub mod seed;

use crate::AppState;
use anyhow::{Result, bail};
use rusqlite::Connection;
use std::time::Duration;

/// Table creation + seeding — fast, local, synchronous. Must finish before
/// the server starts, but never touches the network so it doesn't hold
/// things up.
pub fn init_schema(conn: &Connection) -> Result<()> {
    schema::create_tables(conn)?;
    seed::load_all(conn)?;
    assert_reference_covers_researched(conn)?;
    Ok(())
}

/// Two country lists exist on purpose — `country_reference` (every country) and
/// `countries` (the researched dossiers) — and two lists can drift. Fail at
/// boot, before the listener binds, rather than letting a researched country
/// silently lack a centroid and vanish from the globe.
fn assert_reference_covers_researched(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT c.country_code FROM countries c
         LEFT JOIN country_reference r USING (country_code)
         WHERE r.country_code IS NULL
         ORDER BY c.country_code",
    )?;
    let missing = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if !missing.is_empty() {
        bail!(
            "countries present in `countries` but absent from `country_reference`: {}. \
             Add them to data/seed/country_reference.json (regenerate with \
             scripts/gen_country_reference.mjs).",
            missing.join(", ")
        );
    }
    Ok(())
}

async fn run_with_timeout<F>(name: &str, seconds: u64, fut: F)
where
    F: std::future::Future<Output = Result<()>>,
{
    match tokio::time::timeout(Duration::from_secs(seconds), fut).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("{name} fetcher failed: {e}"),
        Err(_) => eprintln!("{name} fetcher timed out after {seconds}s, continuing startup"),
    }
}

/// Runs all data fetchers concurrently, each with its own timeout, so one
/// slow/rate-limited API (OONI, historically — up to a few minutes) can't
/// starve the others of time budget. Intended to be `tokio::spawn`ed from
/// `main` *after* the HTTP listener is already up, rather than awaited
/// before it — otherwise the server doesn't accept a single request until
/// every external fetch finishes.
pub async fn run_fetchers(state: &AppState) {
    tokio::join!(
        run_with_timeout("ooni", 300, crate::fetchers::ooni::fetch_and_store(state)),
        run_with_timeout(
            "ooni_categories",
            180,
            crate::fetchers::ooni::fetch_categories(state)
        ),
        run_with_timeout(
            "tor_metrics",
            120,
            crate::fetchers::tor_metrics::fetch_and_store(state)
        ),
        run_with_timeout(
            "cloudflare",
            30,
            crate::fetchers::cloudflare::fetch_and_store(state)
        ),
        run_with_timeout(
            "freedom_house",
            30,
            crate::fetchers::freedom_house::fetch_and_store(state)
        ),
        run_with_timeout("ioda", 180, crate::fetchers::ioda::fetch_and_store(state)),
        run_with_timeout(
            "indices",
            60,
            crate::fetchers::indices::fetch_and_store(state)
        ),
    );
    println!("Background data fetchers finished.");
}
