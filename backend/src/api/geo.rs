use crate::{AppState, db, models::country_reference::CountryReference};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GeoQuery {
    /// Include ISO codes the basemap has no geometry for. Off by default,
    /// since a client drawing a globe cannot do anything with them.
    #[serde(default)]
    pub all: bool,
}

/// Country identity and map geometry for the whole world. Separate from
/// `/api/countries`, which returns the researched policy dossiers — this is the
/// list a client uses to decide what to draw and what is clickable.
///
/// Mounted at `/api/geo` rather than under `/api/countries/...` so it cannot be
/// shadowed by the dynamic `/api/countries/:code` route.
pub async fn list_geo(
    State(state): State<AppState>,
    Query(params): Query<GeoQuery>,
) -> Result<Json<Vec<CountryReference>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = if params.all {
        db::countries::all(&conn)
    } else {
        db::countries::drawable(&conn)
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
