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
use std::time::Duration;
use uuid::Uuid;

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
    oauth_configured: bool,
}

#[derive(Deserialize)]
struct DevConnectRequest {
    username: String,
}

#[derive(Deserialize)]
struct GitHubCallbackQuery {
    code: Option<String>,
    error: Option<String>,
    state: Option<String>,
}

fn github_oauth_configured(state: &AppState) -> bool {
    state.config.github_client_id.is_some() && state.config.github_client_secret.is_some()
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
    let oauth_state = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, oauth_state)
        VALUES (1, 0, ?)
        ON CONFLICT(id) DO UPDATE SET
          oauth_state = excluded.oauth_state,
          updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&oauth_state)
    .execute(&state.pool)
    .await?;

    let redirect_uri = github_callback_url(&state);
    let location = Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri.as_str()),
            ("scope", "read:user"),
            ("state", oauth_state.as_str()),
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
        return Ok(Redirect::to(&state.config.public_app_url));
    }

    let returned_state = query.state.ok_or_else(|| {
        ApiError::OAuth("GitHub OAuth callback did not include state".to_string())
    })?;
    let stored_state: Option<String> =
        sqlx::query_scalar("SELECT oauth_state FROM auth_state WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?
            .flatten();
    if stored_state.as_deref() != Some(returned_state.as_str()) {
        return Err(ApiError::OAuth(
            "GitHub OAuth callback state did not match".to_string(),
        ));
    }

    let code = query
        .code
        .ok_or_else(|| ApiError::OAuth("GitHub OAuth callback did not include code".to_string()))?;
    let (client_id, client_secret) = github_oauth_config(&state)?;
    let redirect_uri = github_callback_url(&state);
    let access_token = exchange_github_code(client_id, client_secret, &code, &redirect_uri).await?;
    let username = fetch_github_username(&access_token).await?;

    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token)
        VALUES (1, 1, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          connected = 1,
          username = excluded.username,
          access_token = excluded.access_token,
          oauth_state = NULL,
          updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&username)
    .bind(&access_token)
    .execute(&state.pool)
    .await?;

    Ok(Redirect::to(&state.config.public_app_url))
}

async fn exchange_github_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String, ApiError> {
    let client = github_oauth_http_client()?;
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("accept", "application/json")
        .form(&github_token_exchange_form(
            client_id,
            client_secret,
            code,
            redirect_uri,
        ))
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
    let client = github_oauth_http_client()?;
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

fn github_callback_url(state: &AppState) -> String {
    format!("{}/api/auth/github/callback", state.config.public_base_url)
}

fn github_token_exchange_form<'a>(
    client_id: &'a str,
    client_secret: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
) -> Vec<(&'static str, &'a str)> {
    vec![
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", code),
        ("redirect_uri", redirect_uri),
    ]
}

fn github_oauth_http_timeout() -> Duration {
    crate::github::GITHUB_HTTP_TIMEOUT
}

fn github_oauth_http_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(github_oauth_http_timeout())
        .build()
        .map_err(|error| ApiError::OAuth(error.to_string()))
}

async fn auth_state(State(state): State<AppState>) -> Result<Json<AuthStateResponse>, ApiError> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT connected, username FROM auth_state WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;
    Ok(Json(AuthStateResponse {
        connected: row.as_ref().map(|r| r.0 == 1).unwrap_or(false),
        username: row.and_then(|r| r.1),
        oauth_configured: github_oauth_configured(&state),
    }))
}

// 実 OAuth の代わりにローカル状態だけを接続済みにする、開発用の入口。
async fn dev_connect(
    State(state): State<AppState>,
    Json(payload): Json<DevConnectRequest>,
) -> Result<Json<AuthStateResponse>, ApiError> {
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token, oauth_state)
        VALUES (1, 1, ?, NULL, NULL)
        ON CONFLICT(id) DO UPDATE SET
          connected = 1,
          username = excluded.username,
          access_token = excluded.access_token,
          oauth_state = excluded.oauth_state,
          updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&payload.username)
    .execute(&state.pool)
    .await?;

    Ok(Json(AuthStateResponse {
        connected: true,
        username: Some(payload.username),
        oauth_configured: github_oauth_configured(&state),
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
            github_client: None,
        };

        let response = github_start(State(state)).await.unwrap().into_response();

        let location = response.headers().get(LOCATION).unwrap().to_str().unwrap();
        assert!(location.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(location.contains("client_id=test-client"));
        assert!(location.contains(
            "redirect_uri=http%3A%2F%2F127.0.0.1%3A4317%2Fapi%2Fauth%2Fgithub%2Fcallback"
        ));
        assert!(location.contains("scope=read%3Auser"));
        assert!(location.contains("state="));
    }

    #[tokio::test]
    async fn github_start_persists_oauth_state() {
        let mut config = Config::test();
        config.github_client_id = Some("test-client".to_string());
        config.github_client_secret = Some("test-secret".to_string());
        let pool = connect(&config).await.unwrap();
        let state = AppState {
            repositories: RepositoryStore::new(pool.clone()),
            pool: pool.clone(),
            config,
            github_client: None,
        };

        let response = github_start(State(state)).await.unwrap().into_response();
        let location = response.headers().get(LOCATION).unwrap().to_str().unwrap();
        assert!(location.contains("state="));

        let oauth_state: Option<String> =
            sqlx::query_scalar("SELECT oauth_state FROM auth_state WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(oauth_state.is_some_and(|state| location.contains(&state)));
    }

    #[tokio::test]
    async fn github_callback_rejects_mismatched_state_before_exchange() {
        let mut config = Config::test();
        config.github_client_id = Some("test-client".to_string());
        config.github_client_secret = Some("test-secret".to_string());
        let pool = connect(&config).await.unwrap();
        sqlx::query(
            r#"
            INSERT INTO auth_state (id, connected, oauth_state)
            VALUES (1, 0, 'expected-state')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        let state = AppState {
            repositories: RepositoryStore::new(pool.clone()),
            pool,
            config,
            github_client: None,
        };

        let response = github_callback(
            State(state),
            Query(GitHubCallbackQuery {
                code: Some("code-from-github".to_string()),
                error: None,
                state: Some("wrong-state".to_string()),
            }),
        )
        .await
        .unwrap_err()
        .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn github_token_exchange_form_includes_redirect_uri() {
        let form = github_token_exchange_form(
            "test-client",
            "test-secret",
            "code-from-github",
            "http://127.0.0.1:4317/api/auth/github/callback",
        );

        assert!(form.contains(&(
            "redirect_uri",
            "http://127.0.0.1:4317/api/auth/github/callback"
        )));
    }

    #[test]
    fn github_oauth_http_timeout_matches_discovery_client() {
        assert_eq!(
            github_oauth_http_timeout(),
            crate::github::GITHUB_HTTP_TIMEOUT
        );
    }

    #[tokio::test]
    async fn dev_connect_clears_existing_access_token() {
        let config = Config::test();
        let pool = connect(&config).await.unwrap();
        sqlx::query(
            r#"
            INSERT INTO auth_state (id, connected, username, access_token)
            VALUES (1, 1, 'github-user', 'github-token')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        let state = AppState {
            repositories: RepositoryStore::new(pool.clone()),
            pool: pool.clone(),
            config,
            github_client: None,
        };

        let _ = dev_connect(
            State(state),
            Json(DevConnectRequest {
                username: "local-dev".to_string(),
            }),
        )
        .await
        .unwrap();

        let token: Option<String> =
            sqlx::query_scalar("SELECT access_token FROM auth_state WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert!(token.is_none());
    }
}
