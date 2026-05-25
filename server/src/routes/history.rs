use crate::{app::AppState, error::ApiError, models::HistoryItem};
use axum::{extract::State, routing::get, Json, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(history))
}

async fn history(State(state): State<AppState>) -> Result<Json<Vec<HistoryItem>>, ApiError> {
    Ok(Json(state.repositories.history().await?))
}
