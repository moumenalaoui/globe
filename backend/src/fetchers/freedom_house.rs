use crate::AppState;
use crate::util::date::today_iso;
use anyhow::Result;

// Source: Freedom House, "Freedom on the Net 2024" — verified directly
// against freedomhouse.org/country/{slug}/freedom-net/2024 on 2026-07-07.
// There is no confirmed live/bulk feed for this report: Freedom House
// publishes no JSON API (the previously-attempted report endpoint 404s), and
// Our World in Data's Freedom House mirror only carries the "Freedom in the
// World" aggregate score, not Freedom on the Net's access/content/rights
// breakdown — a different report with a different scale. Static until FOTN
// 2025 is confirmed reachable some other way.
//
// Syria is intentionally absent: Freedom on the Net rates 72 countries
// selected by internet population and regional significance, and Syria is
// not among them. There is no real score to show for it.
// (country_code, overall, obstacles_access, limits_content, violations_rights, status)
const FOTN_2024: &[(&str, f64, f64, f64, f64, &str)] = &[
    ("IR", 12.0, 7.0, 4.0, 1.0, "Not Free"),
    ("AE", 30.0, 14.0, 9.0, 7.0, "Not Free"),
    ("SA", 25.0, 13.0, 7.0, 5.0, "Not Free"),
    ("IQ", 40.0, 10.0, 16.0, 14.0, "Partly Free"),
];

const FOTN_YEAR: i64 = 2024;

/// Populates `country_scores` for the 4 countries Freedom on the Net
/// actually rates. No live fetch — see the sourcing note above.
pub async fn fetch_and_store(state: &AppState) -> Result<()> {
    for (code, overall, access, content, rights, status) in FOTN_2024 {
        if let Err(e) =
            insert_country_score(state, code, *overall, *access, *content, *rights, status)
        {
            eprintln!("freedom_house: failed to store score for {code}: {e}");
        }
    }
    Ok(())
}

fn insert_country_score(
    state: &AppState,
    country: &str,
    score_overall: f64,
    score_access: f64,
    score_content: f64,
    score_rights: f64,
    classification: &str,
) -> Result<()> {
    let id = format!("{country}-FREEDOM_HOUSE-{FOTN_YEAR}");
    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    conn.execute(
        "INSERT OR REPLACE INTO country_scores
         (id, country_code, source, year, score_overall, score_access, score_content, score_rights, classification, last_updated)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            id,
            country,
            "FREEDOM_HOUSE",
            FOTN_YEAR,
            score_overall,
            score_access,
            score_content,
            score_rights,
            classification,
            today_iso(),
        ],
    )?;
    Ok(())
}

// today_iso lives in crate::util::date — it was duplicated here and in ooni.rs.
