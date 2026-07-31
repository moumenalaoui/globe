use crate::{
    AppState,
    models::model_release::{LocalDeployability, ModelRelease, WeightAccess},
};
use axum::{Json, extract::State, http::StatusCode};

fn row_to_model(row: &rusqlite::Row) -> rusqlite::Result<ModelRelease> {
    let weight_access: String = row.get(5)?;
    let local_deployability: String = row.get(8)?;

    Ok(ModelRelease {
        model_id: row.get(0)?,
        provider: row.get(1)?,
        model_name: row.get(2)?,
        origin_country: row.get(3)?,
        release_date: row.get(4)?,
        weight_access: serde_json::from_str(&format!("\"{}\"", weight_access))
            .unwrap_or(WeightAccess::ClosedApi),
        license_type: row.get(6)?,
        parameter_class: row.get(7)?,
        local_deployability: serde_json::from_str(&format!("\"{}\"", local_deployability))
            .unwrap_or(LocalDeployability::CloudOnly),
        telemetry_risk_default: row.get(9)?,
        confidence: row.get(10)?,
        huggingface_repo_id: row.get(11)?,
        source_notes: row.get(12)?,
        downloads_all_time: row.get(13)?,
    })
}

const SELECT: &str = "SELECT model_id, provider, model_name, origin_country, release_date,
    weight_access, license_type, parameter_class, local_deployability,
    telemetry_risk_default, confidence, huggingface_repo_id, model_releases.source_notes, downloads_all_time
    FROM model_releases LEFT JOIN model_usage USING (model_id)";

pub async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<Vec<ModelRelease>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map([], row_to_model)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
