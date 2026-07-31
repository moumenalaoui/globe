pub mod cloudflare;
pub mod freedom_house;
pub mod indices;
pub mod ioda;
pub mod ooni;
pub mod tor_metrics;

use crate::AppState;
use anyhow::Result;

/// The country list a per-country fetch sweep should cover — every drawable
/// country, in priority order — read from `country_reference` rather than a
/// compile-time array. The OONI and Tor sweeps no longer iterate this (they
/// group by country server-side and filter against `known_codes`); it remains
/// for genuinely per-country feeds like Cloudflare, which walk it highest
/// priority first so a tight time budget still covers the researched countries.
///
/// Returns an owned `Vec` and drops the guard before returning, so callers
/// cannot accidentally hold the DB lock across the `.await` of an HTTP request.
pub(crate) fn fetch_codes(state: &AppState) -> Result<Vec<String>> {
    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    crate::db::countries::codes_by_priority(&conn)
}
