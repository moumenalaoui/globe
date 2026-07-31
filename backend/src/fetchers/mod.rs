pub mod cloudflare;
pub mod freedom_house;
pub mod ioda;
pub mod ooni;
pub mod tor_metrics;

use crate::AppState;
use anyhow::Result;

/// The country list a fetch sweep should cover, read from `country_reference`
/// rather than a compile-time array.
///
/// Returns an owned `Vec` and drops the guard before returning, so callers
/// cannot accidentally hold the DB lock across the `.await` of an HTTP request.
pub(crate) fn fetch_codes(state: &AppState) -> Result<Vec<String>> {
    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    crate::db::countries::focus_codes(&conn)
}
