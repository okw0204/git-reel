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
    let row: Option<(i64, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT connected, username, access_token FROM auth_state WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;
    let oauth_configured =
        state.config.github_client_id.is_some() && state.config.github_client_secret.is_some();
    let auth_connected = row
        .as_ref()
        .map(|(connected, _, access_token)| {
            oauth_configured && *connected == 1 && access_token.is_some()
        })
        .unwrap_or(false);

    Ok(Json(SettingsResponse {
        auth_connected,
        username: auth_connected.then(|| row.and_then(|r| r.1)).flatten(),
        discovery_mix: vec!["recently_updated", "recently_created", "language_rotation"],
        database: "sqlite",
    }))
}
