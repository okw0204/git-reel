# GitHub OAuth 接続 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** GitHub OAuth で接続し、GitHub ユーザー名とアクセストークンを既存の `auth_state` に保存できるようにする。

**Architecture:** サーバーは `Config` に OAuth 設定を持たせ、`/api/auth/github/start` と `/api/auth/github/callback` を `routes/auth.rs` に追加する。GitHub の token レスポンスと `/user` レスポンスの JSON 変換は `github.rs` に閉じ込め、Web は未接続ボタンだけを OAuth 開始 URL への遷移に差し替える。

**Tech Stack:** Rust, axum, reqwest, sqlx SQLite, serde, Vitest, React, TypeScript, Vite

---

## ファイル構成

- Modify: `server/src/config.rs`
- 役割: `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GIT_REEL_PUBLIC_BASE_URL` を optional な設定として読み込む。OAuth 未設定でもサーバー起動は成功させる。
- Modify: `server/src/app.rs`
- 役割: `Config` を `AppState` に保持し、認証ルートから OAuth 設定を参照できるようにする。
- Modify: `server/src/error.rs`
- 役割: OAuth 設定不足や GitHub API 失敗を 500 系 API エラーとして扱う。
- Modify: `server/src/github.rs`
- 役割: OAuth token レスポンスと user レスポンスの JSON パース関数を追加する。
- Modify: `server/src/routes/auth.rs`
- 役割: OAuth start/callback ルート、GitHub への HTTP 呼び出し、`auth_state` 更新を実装する。既存の `dev-connect` は残す。
- Modify: `server/tests/github_fixtures.rs`
- 役割: GitHub OAuth 関連 JSON のパーステストを追加する。
- Modify: `server/tests/api_flow.rs`
- 役割: OAuth 設定不足時の start 失敗と、GitHub 側キャンセル時に未接続のまま戻ることをテストする。
- Modify: `web/src/screens/ReelScreen.tsx`
- 役割: 未接続ボタンを「GitHubに接続」に変更し、`/api/auth/github/start` へ遷移する。
- Modify: `web/src/App.test.tsx`
- 役割: 未接続時の OAuth ボタン表示と遷移を確認する。

## タスク

### Task 1: OAuth 設定と `AppState`

**Files:**
- Modify: `server/src/config.rs:1-22`
- Modify: `server/src/app.rs:1-39`
- Modify: `server/src/error.rs:1-34`
- Modify: `server/src/routes/auth.rs:1-60`
- Test: `server/tests/api_flow.rs`

- [ ] **Step 1: 失敗する API テストを追加する**

`server/tests/api_flow.rs` の `auth_state_starts_disconnected_and_dev_connect_sets_user` の後に追加する。

```rust
#[tokio::test]
async fn github_oauth_start_requires_oauth_config() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let response = app
        .oneshot(
            Request::get("/api/auth/github/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
```

- [ ] **Step 2: 失敗を確認する**

Run: `cargo test --manifest-path server/Cargo.toml github_oauth_start_requires_oauth_config`

Expected: `/api/auth/github/start` が未実装なので 404 になり、テストが失敗する。

- [ ] **Step 3: `Config` に OAuth 設定を追加する**

`server/src/config.rs` を次の内容に置き換える。

```rust
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub github_token: Option<String>,
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub public_base_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("GIT_REEL_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:git-reel.db".to_string()),
            github_token: std::env::var("GITHUB_TOKEN").ok(),
            github_client_id: std::env::var("GITHUB_CLIENT_ID").ok(),
            github_client_secret: std::env::var("GITHUB_CLIENT_SECRET").ok(),
            public_base_url: std::env::var("GIT_REEL_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:4317".to_string()),
        }
    }

    pub fn test() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            github_token: None,
            github_client_id: None,
            github_client_secret: None,
            public_base_url: "http://127.0.0.1:4317".to_string(),
        }
    }
}
```

- [ ] **Step 4: `AppState` に `Config` を保持する**

`server/src/app.rs` の `AppState` を次の形にする。

```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub repositories: RepositoryStore,
    pub config: Config,
}
```

同じファイルの `state` 作成部分を次の形にする。

```rust
let state = AppState {
    repositories: RepositoryStore::new(pool.clone()),
    pool,
    config,
};
```

- [ ] **Step 5: OAuth 用の API エラーを追加する**

`server/src/error.rs` の `ApiError` を次の形にする。

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("oauth error: {0}")]
    OAuth(String),
    #[error("not found")]
    NotFound,
}
```

`IntoResponse` の status 判定を次の形にする。

```rust
let status = match self {
    ApiError::NotFound => StatusCode::NOT_FOUND,
    ApiError::Database(_)
    | ApiError::Migration(_)
    | ApiError::Serialization(_)
    | ApiError::OAuth(_) => StatusCode::INTERNAL_SERVER_ERROR,
};
```

- [ ] **Step 6: OAuth start ルートを追加する**

`server/src/routes/auth.rs` の import を次の形にする。

```rust
use crate::{app::AppState, error::ApiError};
use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
```

`router()` に次のルートを追加する。

```rust
.route("/github/start", get(github_start))
```

`DevConnectRequest` の後に追加する。

```rust
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
```

- [ ] **Step 7: テストが通ることを確認する**

Run: `cargo test --manifest-path server/Cargo.toml github_oauth_start_requires_oauth_config`

Expected: PASS。

- [ ] **Step 8: コミットする**

```bash
git add server/src/config.rs server/src/app.rs server/src/error.rs server/src/routes/auth.rs server/tests/api_flow.rs
git commit -m "feat: add github oauth configuration"
```

### Task 2: GitHub OAuth JSON パース

**Files:**
- Modify: `server/src/github.rs:1-93`
- Modify: `server/tests/github_fixtures.rs:1-31`

- [ ] **Step 1: 失敗する parser テストを追加する**

`server/tests/github_fixtures.rs` の import を次の形にする。

```rust
use git_reel_server::github::{
    parse_graphql_readme_preview, parse_oauth_token_response, parse_search_response,
    parse_user_response,
};
```

ファイル末尾に追加する。

```rust
#[test]
fn extracts_oauth_access_token() {
    let token = parse_oauth_token_response(
        r#"{"access_token":"gho_example","token_type":"bearer","scope":"read:user"}"#,
    )
    .unwrap();
    assert_eq!(token, "gho_example");
}

#[test]
fn extracts_github_user_login() {
    let login = parse_user_response(r#"{"login":"okw0204","id":12345}"#).unwrap();
    assert_eq!(login, "okw0204");
}
```

- [ ] **Step 2: 失敗を確認する**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: `parse_oauth_token_response` と `parse_user_response` が未定義で失敗する。

- [ ] **Step 3: OAuth レスポンス型と parser を追加する**

`server/src/github.rs` の `GitHubError` の後に追加する。

```rust
#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}
```

`server/src/github.rs` の末尾に追加する。

```rust
pub fn parse_oauth_token_response(body: &str) -> Result<String, GitHubError> {
    let response: OAuthTokenResponse = serde_json::from_str(body)?;
    Ok(response.access_token)
}

pub fn parse_user_response(body: &str) -> Result<String, GitHubError> {
    let response: UserResponse = serde_json::from_str(body)?;
    Ok(response.login)
}
```

- [ ] **Step 4: parser テストが通ることを確認する**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: PASS。

- [ ] **Step 5: コミットする**

```bash
git add server/src/github.rs server/tests/github_fixtures.rs
git commit -m "feat: parse github oauth responses"
```

### Task 3: OAuth callback フロー

**Files:**
- Modify: `server/src/routes/auth.rs:1-60`
- Modify: `server/tests/api_flow.rs`

- [ ] **Step 1: GitHub 側キャンセルの失敗テストを追加する**

`server/tests/api_flow.rs` の `github_oauth_start_requires_oauth_config` の後に追加する。

```rust
#[tokio::test]
async fn github_oauth_callback_error_redirects_without_connecting() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::get("/api/auth/github/callback?error=access_denied")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("location").unwrap(), "/");

    let response = app
        .oneshot(Request::get("/api/auth/state").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let state: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(state["connected"], false);
    assert!(state["username"].is_null());
}
```

- [ ] **Step 2: 失敗を確認する**

Run: `cargo test --manifest-path server/Cargo.toml github_oauth_callback_error_redirects_without_connecting`

Expected: `/api/auth/github/callback` が未実装なので 404 になり、テストが失敗する。

- [ ] **Step 3: callback query とルートを追加する**

`server/src/routes/auth.rs` の `DevConnectRequest` の後に追加する。

```rust
#[derive(Deserialize)]
struct GitHubCallbackQuery {
    code: Option<String>,
    error: Option<String>,
}
```

`router()` に次のルートを追加する。

```rust
.route("/github/callback", get(github_callback))
```

- [ ] **Step 4: callback handler を追加する**

`server/src/routes/auth.rs` の `github_start` の後に追加する。

```rust
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
```

- [ ] **Step 5: GitHub HTTP helper を追加する**

`server/src/routes/auth.rs` の先頭に追加する。

```rust
use crate::github::{parse_oauth_token_response, parse_user_response};
```

`github_callback` の後に追加する。

```rust
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
```

- [ ] **Step 6: callback テストが通ることを確認する**

Run: `cargo test --manifest-path server/Cargo.toml github_oauth_callback_error_redirects_without_connecting`

Expected: PASS。

- [ ] **Step 7: サーバーテスト全体を実行する**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: PASS。

- [ ] **Step 8: コミットする**

```bash
git add server/src/routes/auth.rs server/tests/api_flow.rs
git commit -m "feat: handle github oauth callback"
```

### Task 4: Web の OAuth ボタン

**Files:**
- Modify: `web/src/screens/ReelScreen.tsx:1-143`
- Modify: `web/src/App.test.tsx:1-76`

- [ ] **Step 1: 失敗する Web テストを追加する**

`web/src/App.test.tsx` の最初の test を次の内容に置き換える。

```tsx
test("未接続時に GitHub OAuth 接続へ遷移できる", async () => {
  const location = { href: "http://127.0.0.1:5173/" };
  vi.stubGlobal("location", location);

  render(<App />);

  await screen.findByText("GitHubに接続するとリールを開始できます");
  await userEvent.click(screen.getByRole("button", { name: "GitHubに接続" }));

  expect(window.location.href).toBe("/api/auth/github/start");
});
```

- [ ] **Step 2: 失敗を確認する**

Run: `npm --workspace web run test -- --run web/src/App.test.tsx`

Expected: ボタン文言がまだ `開発用に接続` のため失敗する。

- [ ] **Step 3: 接続 handler を OAuth 遷移に変更する**

`web/src/screens/ReelScreen.tsx` の `connect` 関数を次の内容に置き換える。

```tsx
const connect = () => {
  window.location.href = "/api/auth/github/start";
};
```

- [ ] **Step 4: 未接続時の文言とボタンを変更する**

`web/src/screens/ReelScreen.tsx` の未接続表示内の説明文とボタンを次の内容に置き換える。

```tsx
<p>GitHub OAuth で接続後も、リポジトリ候補はローカルのシード候補を使います。</p>
<button className="primary-button" onClick={connect} type="button">
  <UserCheck aria-hidden="true" size={18} />
  GitHubに接続
</button>
```

- [ ] **Step 5: Web テストが通ることを確認する**

Run: `npm --workspace web run test -- --run web/src/App.test.tsx`

Expected: PASS。

- [ ] **Step 6: Web build を実行する**

Run: `npm --workspace web run build`

Expected: PASS。

- [ ] **Step 7: コミットする**

```bash
git add web/src/screens/ReelScreen.tsx web/src/App.test.tsx
git commit -m "feat: start github oauth from reel screen"
```

### Task 5: 最終検証

**Files:**
- Verify: `server/src/config.rs`
- Verify: `server/src/app.rs`
- Verify: `server/src/error.rs`
- Verify: `server/src/github.rs`
- Verify: `server/src/routes/auth.rs`
- Verify: `server/tests/api_flow.rs`
- Verify: `server/tests/github_fixtures.rs`
- Verify: `web/src/screens/ReelScreen.tsx`
- Verify: `web/src/App.test.tsx`

- [ ] **Step 1: 標準テストを実行する**

Run: `npm test`

Expected: `npm run test:web` と `npm run test:server` が PASS。

- [ ] **Step 2: 型チェック込み build を実行する**

Run: `npm --workspace web run build`

Expected: PASS。

- [ ] **Step 3: 変更ファイルを確認する**

Run: `git diff --stat`

Expected: この計画に列挙したファイルだけが変更されている。

- [ ] **Step 4: 差分内容を確認する**

Run: `git diff`

Expected: OAuth 設定は optional、`/api/auth/dev-connect` は残っている、OAuth start は `scope=read:user`、callback 成功時は `username` と `access_token` を保存、Web ボタンは `api.devConnect()` を呼ばない。

- [ ] **Step 5: 検証で修正が発生した場合だけコミットする**

修正が発生した場合は次を実行する。

```bash
git add server/src/config.rs server/src/app.rs server/src/error.rs server/src/github.rs server/src/routes/auth.rs server/tests/api_flow.rs server/tests/github_fixtures.rs web/src/screens/ReelScreen.tsx web/src/App.test.tsx
git commit -m "test: verify github oauth connection flow"
```

修正がなかった場合は空コミットを作らない。

## 自己レビュー

- 仕様網羅: OAuth 設定、start/callback ルート、`read:user` scope、GitHub `login` 取得、`auth_state` の `username` と `access_token` 保存、設定不足時の起動継続、UI ボタン差し替え、`dev-connect` 維持、標準テストを含めた。
- スコープ境界: GitHub Search API や GraphQL API からの実データ取得、GitHub 書き込み、複数ユーザー、Cookie セッション、refresh token、deep link は実装しない。
- プレースホルダー確認: 未定義の作業、空のテスト手順、将来の追記に依存する記述は含めていない。
- 型と名前の整合性: `Config.github_client_id`, `Config.github_client_secret`, `Config.public_base_url`, `parse_oauth_token_response`, `parse_user_response`, `github_start`, `github_callback` の名称はタスク間で一致している。
