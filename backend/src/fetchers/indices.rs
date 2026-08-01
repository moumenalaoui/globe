use crate::AppState;
use crate::util::date::today_iso;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

// Two global freedom indices, sourced from Our World in Data's clean grapher
// CSVs (which republish V-Dem and RSF per country-year). Both cover ~180
// countries, giving the whole world a baseline score. Stored in the shared
// `country_scores` table.
//
// OWID serves these behind a 301 to a canonical slug; reqwest follows
// redirects by default. `csvType=full` returns every country-year.
const VDEM_URL: &str = "https://ourworldindata.org/grapher/freedom-of-expression-index.csv?csvType=full&useColumnShortNames=true";
const RSF_URL: &str = "https://ourworldindata.org/grapher/press-freedom-index-rsf.csv?csvType=full&useColumnShortNames=true";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Fetches both indices and upserts the latest year per country into
/// `country_scores`. Each source is independent — one failing does not stop
/// the other.
pub async fn fetch_and_store(state: &AppState) -> Result<()> {
    let alpha3 = {
        let conn = state
            .lock()
            .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
        crate::db::countries::alpha3_to_code(&conn)?
    };

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent("globe-censorship-tracker/0.1")
        .build()?;

    if let Err(e) = fetch_source(state, &client, &alpha3, Source::Vdem).await {
        eprintln!("indices: V-Dem fetch failed: {e}");
    }
    if let Err(e) = fetch_source(state, &client, &alpha3, Source::Rsf).await {
        eprintln!("indices: RSF fetch failed: {e}");
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Source {
    Vdem,
    Rsf,
}

impl Source {
    fn url(self) -> &'static str {
        match self {
            Source::Vdem => VDEM_URL,
            Source::Rsf => RSF_URL,
        }
    }
    fn db_name(self) -> &'static str {
        match self {
            Source::Vdem => "V_DEM",
            Source::Rsf => "RSF",
        }
    }
}

/// The value each source stores, normalised to a 0–100 "higher = more free"
/// scale so all three indices in the sidebar point the same way.
struct Normalised {
    score_overall: f64,
    classification: String,
}

fn normalise(source: Source, raw: f64) -> Normalised {
    match source {
        // V-Dem's freedom-of-expression estimate is already 0–1, higher = freer.
        Source::Vdem => {
            let score = (raw * 100.0).clamp(0.0, 100.0);
            Normalised {
                score_overall: (score * 10.0).round() / 10.0,
                classification: vdem_band(score),
            }
        }
        // OWID's RSF series is the pre-2022 index, where the score is an abuse
        // measure (higher = LESS free). Invert to a 0–100 freedom score so it
        // reads the same direction as the others; the classification is derived
        // from the original value using RSF's old bands.
        Source::Rsf => {
            let inverted = (100.0 - raw).clamp(0.0, 100.0);
            Normalised {
                score_overall: (inverted * 10.0).round() / 10.0,
                classification: rsf_band(raw),
            }
        }
    }
}

fn vdem_band(score_0_100: f64) -> String {
    let label = if score_0_100 >= 80.0 {
        "Very free"
    } else if score_0_100 >= 60.0 {
        "Free"
    } else if score_0_100 >= 40.0 {
        "Partly free"
    } else if score_0_100 >= 20.0 {
        "Repressed"
    } else {
        "Highly repressed"
    };
    label.to_string()
}

// RSF's pre-2022 bands, defined on the raw (higher = worse) score.
fn rsf_band(raw: f64) -> String {
    let label = if raw < 15.0 {
        "Good"
    } else if raw < 25.0 {
        "Satisfactory"
    } else if raw < 35.0 {
        "Problematic"
    } else if raw < 55.0 {
        "Difficult"
    } else {
        "Very serious"
    };
    label.to_string()
}

async fn fetch_source(
    state: &AppState,
    client: &reqwest::Client,
    alpha3: &HashMap<String, String>,
    source: Source,
) -> Result<()> {
    let body = client
        .get(source.url())
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // Keep only the latest year per country. OWID rows are one country-year
    // each: entity, code (ISO3), year, value[, region].
    let mut latest: HashMap<String, (i64, f64)> = HashMap::new();
    let mut reader = csv::Reader::from_reader(body.as_bytes());
    for record in reader.records() {
        let record = record?;
        let code_iso3 = record.get(1).unwrap_or("").trim().to_uppercase();
        if code_iso3.is_empty() {
            continue; // regional/World aggregates carry no ISO3
        }
        let Some(country_code) = alpha3.get(&code_iso3) else {
            continue; // OWID_* pseudo-codes and non-countries
        };
        let Some(year) = record.get(2).and_then(|y| y.trim().parse::<i64>().ok()) else {
            continue;
        };
        let Some(value) = record.get(3).and_then(|v| v.trim().parse::<f64>().ok()) else {
            continue; // blank cells
        };

        latest
            .entry(country_code.clone())
            .and_modify(|cur| {
                if year > cur.0 {
                    *cur = (year, value);
                }
            })
            .or_insert((year, value));
    }

    let mut stored = 0usize;
    for (country_code, (year, raw)) in &latest {
        if let Err(e) = insert_score(state, source, country_code, *year, *raw) {
            eprintln!(
                "indices: failed to store {} for {country_code}: {e}",
                source.db_name()
            );
        } else {
            stored += 1;
        }
    }
    println!(
        "indices: stored {stored} {} scores",
        source.db_name()
    );
    Ok(())
}

fn insert_score(
    state: &AppState,
    source: Source,
    country_code: &str,
    year: i64,
    raw: f64,
) -> Result<()> {
    let n = normalise(source, raw);
    // Keyed without the year so a re-run refreshes in place rather than
    // accumulating a row per year; the latest year is what we keep.
    let id = format!("{country_code}-{}", source.db_name());

    let conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    conn.execute(
        "INSERT OR REPLACE INTO country_scores
         (id, country_code, source, year, score_overall, score_access, score_content, score_rights, classification, last_updated)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            id,
            country_code,
            source.db_name(),
            year,
            n.score_overall,
            Option::<f64>::None,
            Option::<f64>::None,
            Option::<f64>::None,
            n.classification,
            today_iso(),
        ],
    )?;
    Ok(())
}
