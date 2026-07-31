//! Reads of `country_reference`.
//!
//! Every one of these returns an owned `Vec`, never a borrow of the connection.
//! That is deliberate: callers are `async` fetchers that must collect their
//! country list, release the DB lock, and only then start awaiting HTTP —
//! holding a `std::sync::MutexGuard` across an `.await` would block the runtime
//! worker for the length of a network request.

use crate::models::country_reference::CountryReference;
use anyhow::Result;
use rusqlite::Connection;

const SELECT: &str = "SELECT country_code, country_name, alpha3, iso_numeric, region, subregion,
    centroid_lat, centroid_lon, bbox_min_lon, bbox_min_lat, bbox_max_lon, bbox_max_lat,
    include_on_globe, priority_tier
    FROM country_reference";

fn row_to_reference(row: &rusqlite::Row) -> rusqlite::Result<CountryReference> {
    Ok(CountryReference {
        country_code: row.get(0)?,
        country_name: row.get(1)?,
        alpha3: row.get(2)?,
        iso_numeric: row.get(3)?,
        region: row.get(4)?,
        subregion: row.get(5)?,
        centroid_lat: row.get(6)?,
        centroid_lon: row.get(7)?,
        bbox_min_lon: row.get(8)?,
        bbox_min_lat: row.get(9)?,
        bbox_max_lon: row.get(10)?,
        bbox_max_lat: row.get(11)?,
        include_on_globe: row.get::<_, i64>(12)? != 0,
        priority_tier: row.get(13)?,
    })
}

/// Every country the globe can draw, ordered by name for display.
pub fn drawable(conn: &Connection) -> Result<Vec<CountryReference>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} WHERE include_on_globe = 1 ORDER BY country_name"
    ))?;
    let rows = stmt
        .query_map([], row_to_reference)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Every row, including ISO codes with no basemap geometry.
pub fn all(conn: &Connection) -> Result<Vec<CountryReference>> {
    let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY country_code"))?;
    let rows = stmt
        .query_map([], row_to_reference)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Drawable country codes in fetch order: priority tier first, then
/// alphabetical so the order is stable across runs.
// Not yet called — consumed when the sweep covers every country rather than
// just the researched ones.
#[allow(dead_code)]
pub fn codes_by_priority(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT country_code FROM country_reference
         WHERE include_on_globe = 1
         ORDER BY priority_tier, country_code",
    )?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// The actively-researched countries (`priority_tier = 0`).
pub fn focus_codes(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT country_code FROM country_reference
         WHERE priority_tier = 0
         ORDER BY country_code",
    )?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Uppercased set of known codes, for validating codes that arrive from
/// upstream feeds. Bulk responses carry entries outside ISO 3166-1 — the Tor
/// CSV emits `''`, `'??'`, `'ap'` among others — and those must be dropped
/// rather than stored as if they were countries.
// Not yet called — consumed by the bulk-response ingest.
#[allow(dead_code)]
pub fn known_codes(conn: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT country_code FROM country_reference")?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;
    Ok(rows)
}
