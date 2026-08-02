use axum::{Router, routing::get};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tower_http::services::{ServeDir, ServeFile};

mod api;
mod db;
// Retained but unrouted. `POST /api/evaluate` is not mounted on this public
// read-only deployment, which leaves the whole assessment engine unreachable
// and therefore "dead" to the compiler. It is kept compiling — and its
// snapshot test kept running — so the feature can be restored behind a guard
// without resurrecting deleted code.
#[allow(dead_code)]
mod engine;
mod fetchers;
mod models;
mod util;

pub type AppState = Arc<Mutex<Connection>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load backend/.env (cwd is `backend/`) into the process environment so
    // CLOUDFLARE_API_TOKEN and friends reach std::env::var below. A plain Rust
    // binary — unlike Vite — does not read .env on its own. `.ok()` keeps
    // startup working when no .env is present.
    dotenvy::dotenv().ok();

    // Defaults to the historical relative path so `cargo run` from `backend/`
    // keeps working untouched; in production this points at a mounted volume.
    let db_path = std::env::var("DATABASE_PATH")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "mena_ai.db".to_string());

    let conn = Connection::open(&db_path)
        .map_err(|e| anyhow::anyhow!("could not open database at `{db_path}`: {e}"))?;
    println!("DB at {db_path}");
    db::init_schema(&conn)?;
    println!("DB initialized and seeded.");
    warn_missing_optional_tokens();

    let state: AppState = Arc::new(Mutex::new(conn));

    // The built SPA. Default is the repo layout relative to `backend/`, so a
    // local `npm run build` is served without configuration; in a container
    // this points wherever dist was copied.
    let static_dir = std::env::var("STATIC_DIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "../frontend/dist".to_string());
    if std::path::Path::new(&static_dir).join("index.html").exists() {
        println!("Serving SPA from {static_dir}");
    } else {
        eprintln!(
            "WARNING: no index.html under `{static_dir}` — the API will work but \
             the app will 404. Run `npm run build` in frontend/, or set STATIC_DIR."
        );
    }

    // Fetchers hit slow/rate-limited external APIs (OONI alone can take
    // minutes) — run them in the background instead of blocking server
    // startup on them. The server is fully usable immediately; fetched data
    // fills in as each source completes.
    let fetch_state = state.clone();
    tokio::spawn(async move {
        db::run_fetcher_loop(fetch_state).await;
    });

    // Public and read-only. Every route below is a GET; `POST /api/evaluate`
    // is deliberately NOT mounted — it was the one write path (it persists
    // evidence rows) and nothing in the UI calls it, so on a public URL it
    // would be an anonymous write for no benefit. `api::evaluate` and the
    // whole `engine` module stay compiled and ready to re-route behind a
    // guard if the feature is ever wanted.
    //
    // CORS is absent on purpose: the browser loads the app and calls the API
    // from the same origin, so there is no cross-origin request to permit.
    let app = Router::new()
        .route("/health", get(api::health::health))
        .route("/api/countries", get(api::countries::list_countries))
        .route("/api/countries/:code", get(api::countries::get_country))
        .route("/api/geo", get(api::geo::list_geo))
        .route("/api/models", get(api::models::list_models))
        .route("/api/signals", get(api::signals::list_signals))
        .route("/api/blocking", get(api::blocking::list_blocking))
        .route("/api/categories", get(api::categories::list_categories))
        .route("/api/timeline", get(api::timeline::list_timeline))
        .route("/api/tor-metrics", get(api::tor_metrics::list_tor_metrics))
        .route("/api/outages", get(api::outages::list_outages))
        .route("/api/rankings", get(api::rankings::list_rankings))
        .route(
            "/api/censorship-index",
            get(api::censorship_index::list_censorship_index),
        )
        .route(
            "/api/country-scores",
            get(api::country_scores::list_country_scores),
        )
        // The built SPA. Anything not matching a route above falls through to
        // ServeDir, and anything ServeDir can't find falls through to
        // index.html so client-side routes and deep links resolve.
        .fallback_service(spa_service(&static_dir))
        .with_state(state.clone());

    // Defaults to 3001 (what the Vite dev proxy targets). Overridable so a
    // second instance can be run alongside a dev server for verification.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Listening on 0.0.0.0:{port} (app + API, public read-only)");
    axum::serve(listener, app).await?;
    Ok(())
}

/// `ServeDir` for the built SPA, falling back to `index.html` so client-side
/// routes and refreshes on a deep link return the app instead of a 404.
fn spa_service(dir: &str) -> ServeDir<ServeFile> {
    ServeDir::new(dir).fallback(ServeFile::new(format!("{dir}/index.html")))
}

/// The optional API tokens gate whole data sources, and their absence used to
/// surface only as one `eprintln!` from deep inside a background task — so a
/// deploy that forgot `PULSE_API_TOKEN` looked identical to "Pulse has no data
/// for these countries". Say it once, loudly, at boot.
fn warn_missing_optional_tokens() {
    let missing: Vec<(&str, &str)> = [
        ("PULSE_API_TOKEN", "Internet Resilience Index (all countries)"),
        ("CLOUDFLARE_API_TOKEN", "Cloudflare Radar outage annotations"),
    ]
    .into_iter()
    .filter(|(key, _)| {
        std::env::var(key)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_none()
    })
    .collect();

    if missing.is_empty() {
        return;
    }
    eprintln!("WARNING: {} optional API token(s) missing —", missing.len());
    for (key, effect) in missing {
        eprintln!("WARNING:   {key} unset -> no data for: {effect}");
    }
    eprintln!("WARNING: those sources will render empty, not error. Set them in .env or the host environment.");
}
