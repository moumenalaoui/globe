use axum::{
    Router,
    routing::{get, post},
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

mod api;
mod db;
mod engine;
mod fetchers;
mod models;
mod util;

pub type AppState = Arc<Mutex<Connection>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conn = Connection::open("mena_ai.db")?;
    db::init_schema(&conn)?;
    println!("DB initialized and seeded.");

    let state: AppState = Arc::new(Mutex::new(conn));

    // Fetchers hit slow/rate-limited external APIs (OONI alone can take
    // minutes) — run them in the background instead of blocking server
    // startup on them. The server is fully usable immediately; fetched data
    // fills in as each source completes.
    let fetch_state = state.clone();
    tokio::spawn(async move {
        db::run_fetchers(&fetch_state).await;
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/countries", get(api::countries::list_countries))
        .route("/api/countries/:code", get(api::countries::get_country))
        .route("/api/geo", get(api::geo::list_geo))
        .route("/api/models", get(api::models::list_models))
        .route("/api/deals", get(api::deals::list_deals))
        .route("/api/evaluate", post(api::evaluate::evaluate))
        .route("/api/signals", get(api::signals::list_signals))
        .route("/api/blocking", get(api::blocking::list_blocking))
        .route("/api/categories", get(api::categories::list_categories))
        .route("/api/timeline", get(api::timeline::list_timeline))
        .route("/api/tor-metrics", get(api::tor_metrics::list_tor_metrics))
        .route("/api/outages", get(api::outages::list_outages))
        .route("/api/rankings", get(api::rankings::list_rankings))
        .route(
            "/api/country-scores",
            get(api::country_scores::list_country_scores),
        )
        .layer(cors)
        .with_state(state);

    // Defaults to 3001 (what the Vite dev proxy targets). Overridable so a
    // second instance can be run alongside a dev server for verification.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Backend on http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
