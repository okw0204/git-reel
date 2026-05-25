use crate::{app::AppState, error::ApiError};
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(settings))
}

#[derive(Serialize)]
struct SettingsResponse {
    auth_connected: bool,
    username: Option<String>,
    discovery_mix: Vec<&'static str>,
    database: &'static str,
}

async fn settings(State(state): State<AppState>) -> Result<Json<SettingsResponse>, ApiError> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT connected, username FROM auth_state WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;

    Ok(Json(SettingsResponse {
        auth_connected: row.as_ref().map(|r| r.0 == 1).unwrap_or(false),
        username: row.and_then(|r| r.1),
        discovery_mix: vec!["recently_updated", "recently_created", "language_rotation"],
        database: "sqlite",
    }))
}
