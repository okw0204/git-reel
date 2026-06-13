use crate::{
    app::AppState,
    discovery::DiscoveryService,
    error::ApiError,
    models::{ReelResponse, RepoEventKind},
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/current", get(current))
        .route("/next", post(next))
        .route("/previous", post(previous))
        .route("/:id/save", post(save))
        .route("/:id/skip", post(skip))
        .route("/:id/detail", get(detail))
}

// route 層ではユーザー操作として何を記録するかを決め、履歴やキューの詳細は RepositoryStore に寄せる。

// current はプレビュー用なので、キューを消費せず現在の先頭候補だけを返す。
async fn current(State(state): State<AppState>) -> Result<Json<ReelResponse>, ApiError> {
    // 認証前は discovery を走らせず、フロントが接続導線を出せる empty_reason だけを返す。
    if !auth_connected(&state).await? {
        return Ok(Json(ReelResponse {
            repository: None,
            empty_reason: Some("auth_required".to_string()),
        }));
    }
    DiscoveryService::new(state.repositories.clone())
        .ensure_candidates()
        .await?;
    let repository = state.repositories.next_queued_repository().await?;
    let empty_reason = if repository.is_none() {
        Some("queue_empty".to_string())
    } else {
        None
    };
    Ok(Json(ReelResponse {
        repository,
        empty_reason,
    }))
}

// next はユーザーが候補を見た操作として扱い、キュー消費と viewed 記録を行う。
async fn next(State(state): State<AppState>) -> Result<Json<ReelResponse>, ApiError> {
    // 未接続状態でキューを進めると、接続後に見ていない候補が履歴化されるため先に止める。
    if !auth_connected(&state).await? {
        return Ok(Json(ReelResponse {
            repository: None,
            empty_reason: Some("auth_required".to_string()),
        }));
    }
    DiscoveryService::new(state.repositories.clone())
        .ensure_candidates()
        .await?;
    let repository = state.repositories.claim_next_queued_repository().await?;
    let empty_reason = if repository.is_none() {
        Some("queue_empty".to_string())
    } else {
        None
    };
    Ok(Json(ReelResponse {
        repository,
        empty_reason,
    }))
}

// 戻った履歴もイベント化しておくと、以後の「前へ」の現在地として使える。
async fn previous(State(state): State<AppState>) -> Result<Json<ReelResponse>, ApiError> {
    let repository = state.repositories.previous_reel_repository().await?;
    if let Some(repo) = repository.as_ref() {
        state
            .repositories
            .record_event(repo.id, RepoEventKind::Returned)
            .await?;
    }
    Ok(Json(ReelResponse {
        repository,
        empty_reason: None,
    }))
}

async fn save(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ActionResponse>, ApiError> {
    state.repositories.save_repository(id).await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn skip(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ActionResponse>, ApiError> {
    state
        .repositories
        .record_event(id, RepoEventKind::Skipped)
        .await?;
    // skipped は履歴として残し、キュー上では次候補へ進めるために別途 consumed にする。
    state.repositories.consume_repository(id).await?;
    Ok(Json(ActionResponse { ok: true }))
}

// 詳細を開いたことも軽い関心シグナルとして保存する。
async fn detail(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DetailResponse>, ApiError> {
    state
        .repositories
        .record_event(id, RepoEventKind::DetailOpened)
        .await?;
    let repository = state.repositories.find_repository(id).await?;
    Ok(Json(DetailResponse {
        repository_id: id,
        memo: state.repositories.note_for(id).await?.unwrap_or_default(),
        tags: state.repositories.tags_for(id).await?,
        readme_preview: repository.readme_preview,
        detail_error: None,
    }))
}

async fn auth_connected(state: &AppState) -> Result<bool, ApiError> {
    let connected: Option<i64> = sqlx::query_scalar(
        "SELECT connected FROM auth_state WHERE id = 1 AND access_token IS NOT NULL",
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(connected.unwrap_or(0) == 1)
}

#[derive(Serialize)]
struct ActionResponse {
    ok: bool,
}

#[derive(Serialize)]
struct DetailResponse {
    repository_id: i64,
    memo: String,
    tags: Vec<String>,
    readme_preview: Option<String>,
    detail_error: Option<String>,
}
