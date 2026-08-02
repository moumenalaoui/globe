pub mod countries;
pub mod migrations;
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
    // After create_tables (which only ever builds a *fresh* database) and
    // before any read or seed, so an existing database has every column the
    // rest of the code expects.
    migrations::run(conn)?;
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

/// Default gap between full fetch cycles. Six hours is well inside the two-day
/// staleness threshold `/health` enforces, so a single missed cycle does not
/// turn the deployment red.
const DEFAULT_FETCH_INTERVAL_HOURS: f64 = 6.0;

/// Hard floor on the cycle gap regardless of configuration. These fetchers hit
/// third-party APIs; a misconfigured interval should be slow, never abusive.
const MIN_FETCH_INTERVAL_SECS: u64 = 60;

async fn run_with_timeout<F>(state: &AppState, name: &str, seconds: u64, fut: F)
where
    F: std::future::Future<Output = Result<()>>,
{
    let outcome = match tokio::time::timeout(Duration::from_secs(seconds), fut).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            eprintln!("{name} fetcher failed: {e}");
            Err(e.to_string())
        }
        Err(_) => {
            let msg = format!("timed out after {seconds}s");
            eprintln!("{name} fetcher {msg}");
            Err(msg)
        }
    };
    if let Err(e) = record_run(state, name, outcome.as_ref().err().map(String::as_str)) {
        eprintln!("{name}: could not record fetch outcome: {e}");
    }
}

/// Upserts this fetcher's row in `fetch_runs`. `error` None means success.
fn record_run(state: &AppState, name: &str, error: Option<&str>) -> Result<()> {
    let now = crate::util::date::now_utc_iso();
    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    match error {
        None => conn.execute(
            "INSERT INTO fetch_runs
               (fetcher, last_attempt_at, last_success_at, last_outcome, last_error, consecutive_failures)
             VALUES (?1, ?2, ?2, 'ok', NULL, 0)
             ON CONFLICT(fetcher) DO UPDATE SET
               last_attempt_at = ?2, last_success_at = ?2,
               last_outcome = 'ok', last_error = NULL, consecutive_failures = 0",
            rusqlite::params![name, now],
        )?,
        // last_success_at is deliberately left untouched on failure — that is
        // the whole point of the table.
        Some(err) => conn.execute(
            "INSERT INTO fetch_runs
               (fetcher, last_attempt_at, last_success_at, last_outcome, last_error, consecutive_failures)
             VALUES (?1, ?2, NULL, 'error', ?3, 1)
             ON CONFLICT(fetcher) DO UPDATE SET
               last_attempt_at = ?2, last_outcome = 'error', last_error = ?3,
               consecutive_failures = consecutive_failures + 1",
            rusqlite::params![name, now, err],
        )?,
    };
    Ok(())
}

/// Re-runs every fetcher forever, `FETCH_INTERVAL_HOURS` apart.
///
/// Spawned from `main` so it never blocks request serving. The first cycle
/// starts immediately (a fresh deployment has an empty database); subsequent
/// cycles wait out the interval. This loop is the reason `/health` can stay
/// green — before it existed, data was only ever as fresh as the last restart.
pub async fn run_fetcher_loop(state: AppState) {
    // Parsed as a float so fractional hours work — 0.5 for a half-hourly
    // refresh, and small values to exercise the loop without waiting hours.
    // Floored at MIN_FETCH_INTERVAL_SECS so a typo can't turn this into a
    // tight loop against other people's APIs.
    let hours = std::env::var("FETCH_INTERVAL_HOURS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|h| h.is_finite() && *h > 0.0)
        .unwrap_or(DEFAULT_FETCH_INTERVAL_HOURS);
    let secs = ((hours * 3600.0) as u64).max(MIN_FETCH_INTERVAL_SECS);
    let gap = Duration::from_secs(secs);
    println!("Fetch loop: every {hours}h ({secs}s)");

    loop {
        run_fetchers(&state).await;
        report_cycle(&state);
        tokio::time::sleep(gap).await;
    }
}

/// One line per fetcher after each cycle, so a partial failure is visible in
/// the log as well as on disk.
fn report_cycle(state: &AppState) {
    let Ok(conn) = state.lock() else { return };
    let mut stmt = match conn.prepare(
        "SELECT fetcher, last_outcome, last_success_at, consecutive_failures
         FROM fetch_runs ORDER BY fetcher",
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("fetch cycle: could not read fetch_runs: {e}");
            return;
        }
    };
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            r.get::<_, Option<String>>(2)?,
            r.get::<_, i64>(3)?,
        ))
    });
    let Ok(rows) = rows else { return };

    let mut failed = Vec::new();
    for row in rows.flatten() {
        let (name, outcome, last_success, fails) = row;
        if outcome != "ok" {
            failed.push(format!(
                "{name} (x{fails}, last ok: {})",
                last_success.unwrap_or_else(|| "never".into())
            ));
        }
    }
    if failed.is_empty() {
        println!("Fetch cycle complete: all fetchers ok.");
    } else {
        eprintln!(
            "WARNING: fetch cycle complete with {} failing: {}",
            failed.len(),
            failed.join(", ")
        );
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
        run_with_timeout(state, "ooni", 300, crate::fetchers::ooni::fetch_and_store(state)),
        run_with_timeout(
            state,
            "ooni_categories",
            180,
            crate::fetchers::ooni::fetch_categories(state)
        ),
        run_with_timeout(
            state,
            "tor_metrics",
            600,
            crate::fetchers::tor_metrics::fetch_and_store(state)
        ),
        run_with_timeout(
            state,
            "cloudflare",
            30,
            crate::fetchers::cloudflare::fetch_and_store(state)
        ),
        run_with_timeout(state, "ioda", 180, crate::fetchers::ioda::fetch_and_store(state)),
        run_with_timeout(
            state,
            "indices",
            60,
            crate::fetchers::indices::fetch_and_store(state)
        ),
        run_with_timeout(state, "pulse", 90, crate::fetchers::pulse::fetch_and_store(state)),
    );
    println!("Background data fetchers finished.");
}
