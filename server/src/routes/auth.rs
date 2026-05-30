use crate::{
    app::AppState,
    error::ApiError,
    github::{parse_oauth_token_response, parse_user_response},
};
use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/state", get(auth_state))
        .route("/dev-connect", post(dev_connect))
        .route("/github/start", get(github_start))
        .route("/github/callback", get(github_callback))
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

#[derive(Deserialize)]
struct GitHubCallbackQuery {
    code: Option<String>,
    error: Option<String>,
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
    let location = Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri.as_str()),
            ("scope", "read:user"),
        ],
    )
    .map_err(|error| ApiError::OAuth(error.to_string()))?;
    Ok(Redirect::temporary(location.as_str()))
}

async fn github_callback(
    State(state): State<AppState>,
    Query(query): Query<GitHubCallbackQuery>,
) -> Result<Redirect, ApiError> {
    if query.error.is_some() {
        return Ok(Redirect::to("/"));
    }

    let code = query
        .code
        .ok_or_else(|| ApiError::OAuth("GitHub OAuth callback did not include code".to_string()))?;
    let (client_id, client_secret) = github_oauth_config(&state)?;
    let access_token = exchange_github_code(client_id, client_secret, &code).await?;
    let username = fetch_github_username(&access_token).await?;

    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token)
        VALUES (1, 1, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          connected = 1,
          username = excluded.username,
          access_token = excluded.access_token,
          updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&username)
    .bind(&access_token)
    .execute(&state.pool)
    .await?;

    Ok(Redirect::to("/"))
}

async fn exchange_github_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
) -> Result<String, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
        ])
        .send()
        .await
        .map_err(|error| ApiError::OAuth(error.to_string()))?;

    if !response.status().is_success() {
        return Err(ApiError::OAuth(format!(
            "GitHub token endpoint returned {}",
            response.status()
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|error| ApiError::OAuth(error.to_string()))?;
    parse_oauth_token_response(&body).map_err(|error| ApiError::OAuth(error.to_string()))
}

async fn fetch_github_username(access_token: &str) -> Result<String, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .bearer_auth(access_token)
        .header("accept", "application/vnd.github+json")
        .header("user-agent", "git-reel")
        .send()
        .await
        .map_err(|error| ApiError::OAuth(error.to_string()))?;

    if !response.status().is_success() {
        return Err(ApiError::OAuth(format!(
            "GitHub user endpoint returned {}",
            response.status()
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|error| ApiError::OAuth(error.to_string()))?;
    parse_user_response(&body).map_err(|error| ApiError::OAuth(error.to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, db::connect, repositories::RepositoryStore};
    use axum::{http::header::LOCATION, response::IntoResponse};

    #[tokio::test]
    async fn github_start_encodes_redirect_uri() {
        let mut config = Config::test();
        config.github_client_id = Some("test-client".to_string());
        config.github_client_secret = Some("test-secret".to_string());
        config.public_base_url = "http://127.0.0.1:4317".to_string();
        let pool = connect(&config).await.unwrap();
        let state = AppState {
            repositories: RepositoryStore::new(pool.clone()),
            pool,
            config,
        };

        let response = github_start(State(state)).await.unwrap().into_response();

        assert_eq!(
            response.headers().get(LOCATION).unwrap(),
            "https://github.com/login/oauth/authorize?client_id=test-client&redirect_uri=http%3A%2F%2F127.0.0.1%3A4317%2Fapi%2Fauth%2Fgithub%2Fcallback&scope=read%3Auser"
        );
    }
}
