use crate::{app::AppState, error::ApiError};
use axum::{
    extract::State,
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/state", get(auth_state))
        .route("/dev-connect", post(dev_connect))
        .route("/github/start", get(github_start))
}

#[derive(Serialize)]
struct AuthStateResponse {
    connected: bool,
    username: Option<String>,
}

#[derive(Deserialize)]
struct DevConnectRequest {
    username: String,
}

fn github_oauth_config(state: &AppState) -> Result<(&str, &str), ApiError> {
    let client_id = state
        .config
        .github_client_id
        .as_deref()
        .ok_or_else(|| ApiError::OAuth("GITHUB_CLIENT_ID is not configured".to_string()))?;
    let client_secret = state
        .config
        .github_client_secret
        .as_deref()
        .ok_or_else(|| ApiError::OAuth("GITHUB_CLIENT_SECRET is not configured".to_string()))?;
    Ok((client_id, client_secret))
}

async fn github_start(State(state): State<AppState>) -> Result<Redirect, ApiError> {
    let (client_id, _) = github_oauth_config(&state)?;
    let redirect_uri = format!("{}/api/auth/github/callback", state.config.public_base_url);
    let location = format!(
        "https://github.com/login/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}&scope=read:user"
    );
    Ok(Redirect::temporary(&location))
}

async fn auth_state(State(state): State<AppState>) -> Result<Json<AuthStateResponse>, ApiError> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT connected, username FROM auth_state WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;
    Ok(Json(AuthStateResponse {
        connected: row.as_ref().map(|r| r.0 == 1).unwrap_or(false),
        username: row.and_then(|r| r.1),
    }))
}

// 実 OAuth の代わりにローカル状態だけを接続済みにする、開発用の入口。
async fn dev_connect(
    State(state): State<AppState>,
    Json(payload): Json<DevConnectRequest>,
) -> Result<Json<AuthStateResponse>, ApiError> {
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username)
        VALUES (1, 1, ?)
        ON CONFLICT(id) DO UPDATE SET
          connected = 1,
          username = excluded.username,
          updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&payload.username)
    .execute(&state.pool)
    .await?;

    Ok(Json(AuthStateResponse {
        connected: true,
        username: Some(payload.username),
    }))
}
