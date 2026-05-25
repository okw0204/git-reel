use crate::{
    app::AppState,
    error::ApiError,
    models::{NoteRequest, SavedRepository, TagsRequest},
};
use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(saved))
        .route("/:id/note", patch(note))
        .route("/:id/tags", put(tags))
}

#[derive(Deserialize)]
struct SavedQuery {
    query: Option<String>,
}

#[derive(Serialize)]
struct ActionResponse {
    ok: bool,
}

async fn saved(
    State(state): State<AppState>,
    Query(query): Query<SavedQuery>,
) -> Result<Json<Vec<SavedRepository>>, ApiError> {
    Ok(Json(
        state
            .repositories
            .saved(query.query.as_deref().unwrap_or(""))
            .await?,
    ))
}

async fn note(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<NoteRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    state.repositories.set_note(id, &payload.body).await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn tags(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<TagsRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    state.repositories.replace_tags(id, payload.tags).await?;
    Ok(Json(ActionResponse { ok: true }))
}
