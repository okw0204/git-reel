# GitHub Connection Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** GitHub 接続を OAuth 必須に一本化し、`dev-connect`、`GITHUB_TOKEN` fallback、通常 seed fallback を削除する。

**Architecture:** サーバーは保存済み OAuth access token だけを接続状態と Discovery 補充元として扱う。Web は OAuth 設定済みの場合だけ接続ボタンを表示し、OAuth 未設定では設定不足の案内に留める。固定 seed は通常経路から外し、必要なテストではテストコードが明示的に候補を投入する。

**Tech Stack:** Rust, Axum, SQLx, SQLite, reqwest, React, TypeScript, Vitest, Playwright, npm workspace.

---

## File Structure

- Modify: `server/src/config.rs` removes `github_token` and its env test.
- Modify: `server/src/app.rs` removes `AppState.github_client` and env-token `GitHubClient` construction.
- Modify: `server/src/routes/auth.rs` removes `/dev-connect`, request type, handler, and tests.
- Modify: `server/src/routes/reel.rs` requires saved OAuth token for reel access and stops passing env-token clients to Discovery.
- Modify: `server/src/discovery.rs` removes `with_github_client`, env-token fallback, and seed fallback from normal `ensure_candidates()`.
- Modify: `server/tests/api_flow.rs` rewrites API and Discovery tests around OAuth tokens and explicit candidate setup.
- Modify: `web/src/api/client.ts` removes `devConnect`.
- Modify: `web/src/screens/ReelScreen.tsx` removes dev-connect UI and shows OAuth setup guidance when OAuth env is missing.
- Modify: `web/src/App.test.tsx` updates Web tests for OAuth-only connection behavior.
- Modify: `e2e/git-reel.spec.ts` replaces dev-connect flow with OAuth setup guidance smoke test.
- Modify: `.env.example`, `README.md`, `AGENTS.md` remove outdated connection and fallback descriptions.

## Task 1: Server Config And App State Cleanup

**Files:**
- Modify: `server/src/config.rs`
- Modify: `server/src/app.rs`
- Modify: `server/tests/api_flow.rs`
- Test: `server/src/config.rs`

- [ ] **Step 1: Remove the config test that only exists for `GITHUB_TOKEN`**

In `server/src/config.rs`, delete the entire `#[cfg(test)] mod tests` block at the bottom of the file. The file should no longer reference `GITHUB_TOKEN`, `env_lock`, or `EnvVarGuard`.

- [ ] **Step 2: Remove `github_token` from `Config`**

Edit `server/src/config.rs` so the file becomes:

```rust
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub public_base_url: String,
    pub public_app_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("GIT_REEL_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:git-reel.db".to_string()),
            github_client_id: optional_env("GITHUB_CLIENT_ID"),
            github_client_secret: optional_env("GITHUB_CLIENT_SECRET"),
            public_base_url: std::env::var("GIT_REEL_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:4317".to_string()),
            public_app_url: std::env::var("GIT_REEL_PUBLIC_APP_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:5173".to_string()),
        }
    }

    pub fn test() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            github_client_id: None,
            github_client_secret: None,
            public_base_url: "http://127.0.0.1:4317".to_string(),
            public_app_url: "http://127.0.0.1:5173".to_string(),
        }
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}
```

- [ ] **Step 3: Remove env-token client from app state**

Edit `server/src/app.rs` so imports and state construction become:

```rust
use crate::{config::Config, db::connect, repositories::RepositoryStore, routes};
use axum::{routing::get, Router};
use sqlx::SqlitePool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub repositories: RepositoryStore,
    pub config: Config,
}

pub async fn build_app() -> anyhow::Result<Router> {
    build_app_with_config(Config::from_env()).await
}

pub async fn build_test_app() -> anyhow::Result<Router> {
    build_app_with_config(Config::test()).await
}

// 本番用とテスト用で同じルーター構築経路を通し、差分を Config に閉じ込める。
async fn build_app_with_config(config: Config) -> anyhow::Result<Router> {
    let pool = connect(&config).await?;
    let state = AppState {
        repositories: RepositoryStore::new(pool.clone()),
        pool,
        config,
    };

    // フロントエンドからは /api/* だけを見ればよいように、機能単位でルートを分ける。
    Ok(Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .nest("/api/auth", routes::auth::router())
        .nest("/api/reel", routes::reel::router())
        .nest("/api/saved", routes::saved::router())
        .nest("/api/history", routes::history::router())
        .nest("/api/settings", routes::settings::router())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http()))
}
```

- [ ] **Step 4: Remove `github_client: None` from test `AppState` literals**

In `server/src/routes/auth.rs` tests and any other Rust test files, every literal like this:

```rust
let state = AppState {
    repositories: RepositoryStore::new(pool.clone()),
    pool,
    config,
    github_client: None,
};
```

must become:

```rust
let state = AppState {
    repositories: RepositoryStore::new(pool.clone()),
    pool,
    config,
};
```

If the literal uses `pool: pool.clone()`, keep that field value and only remove `github_client: None`.

- [ ] **Step 5: Run server tests and verify the expected failures**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: FAIL. Remaining failures should be from references to `dev_connect`, `DevConnectRequest`, `with_github_client`, `/api/auth/dev-connect`, or seed fallback behavior. There should be no failure about `Config.github_token` or `AppState.github_client`.

- [ ] **Step 6: Commit**

```bash
git add server/src/config.rs server/src/app.rs server/src/routes/auth.rs server/tests/api_flow.rs
git commit -m "refactor: remove github token app state"
```

## Task 2: Remove Dev Connect API And Server Auth Assumptions

**Files:**
- Modify: `server/src/routes/auth.rs`
- Modify: `server/src/routes/reel.rs`
- Modify: `server/tests/api_flow.rs`

- [ ] **Step 1: Delete the dev-connect route and types**

In `server/src/routes/auth.rs`, remove `post` from the `routing` import if no longer needed by this file:

```rust
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
```

Change `router()` to:

```rust
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/state", get(auth_state))
        .route("/github/start", get(github_start))
        .route("/github/callback", get(github_callback))
}
```

Delete this type:

```rust
#[derive(Deserialize)]
struct DevConnectRequest {
    username: String,
}
```

Delete the whole `dev_connect` function.

- [ ] **Step 2: Remove dev-connect unit tests**

In `server/src/routes/auth.rs`, delete these tests completely:

```rust
dev_connect_clears_existing_access_token
dev_connect_is_rejected_when_oauth_is_configured
```

- [ ] **Step 3: Require access token for reel route auth**

In `server/src/routes/reel.rs`, update `current()` and `next()` to stop passing `state.github_client`:

```rust
DiscoveryService::new(state.repositories.clone())
    .ensure_candidates()
    .await?;
```

Update `auth_connected()` to require a saved OAuth token:

```rust
async fn auth_connected(state: &AppState) -> Result<bool, ApiError> {
    let connected: Option<i64> = sqlx::query_scalar(
        "SELECT connected FROM auth_state WHERE id = 1 AND access_token IS NOT NULL",
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(connected.unwrap_or(0) == 1)
}
```

- [ ] **Step 4: Replace the dev-connect integration test**

In `server/tests/api_flow.rs`, replace `auth_state_starts_disconnected_and_dev_connect_sets_user` with:

```rust
#[tokio::test]
async fn auth_state_starts_disconnected_and_dev_connect_route_is_gone() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let response = app
        .clone()
        .oneshot(Request::get("/api/auth/state").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let state: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(state["connected"], false);

    let response = app
        .oneshot(
            Request::post("/api/auth/dev-connect")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"local-dev"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 5: Add an integration-test helper for an authenticated app**

In `server/tests/api_flow.rs`, add these imports near the top:

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use git_reel_server::{
    app::AppState,
    config::Config,
    db::connect,
    discovery::{DiscoveryCandidate, DiscoveryService},
    github::{GitHubDiscoveryClient, GitHubError},
    models::{NewRepository, RepoEventKind},
    repositories::RepositoryStore,
    routes,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
```

Replace the existing `use axum::{ body::Body, http::{Request, StatusCode}, };` and `use git_reel_server::{ ... };` blocks with the imports above.

Then add this helper below `sample_repo()`:

```rust
async fn authenticated_test_app_with_candidates(
    repositories: Vec<NewRepository>,
) -> axum::Router {
    let config = Config::test();
    let pool = connect(&config).await.unwrap();
    let store = RepositoryStore::new(pool.clone());
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token)
        VALUES (1, 1, 'octocat', 'gho_test_token')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let service = DiscoveryService::new(store.clone());
    let candidates = repositories
        .into_iter()
        .map(DiscoveryCandidate::from_new_repository)
        .collect();
    service
        .enqueue_candidates("test-seed", "explicit test candidates", candidates)
        .await
        .unwrap();

    let state = AppState {
        repositories: store,
        pool,
        config,
    };

    Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .nest("/api/auth", routes::auth::router())
        .nest("/api/reel", routes::reel::router())
        .nest("/api/saved", routes::saved::router())
        .nest("/api/history", routes::history::router())
        .nest("/api/settings", routes::settings::router())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
```

- [ ] **Step 6: Rewrite reel API tests to use OAuth auth and explicit candidates**

In `server/tests/api_flow.rs`, replace the body of `reel_next_save_and_skip_record_events` with:

```rust
let app = authenticated_test_app_with_candidates(vec![sample_repo("acme/reel", 501)]).await;

let next = Request::post("/api/reel/next").body(Body::empty()).unwrap();
let response = app.clone().oneshot(next).await.unwrap();
assert_eq!(response.status(), StatusCode::OK);
let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
let payload: Value = serde_json::from_slice(&body).unwrap();
let id = payload["repository"]["id"].as_i64().unwrap();

let save = Request::post(format!("/api/reel/{id}/save"))
    .body(Body::empty())
    .unwrap();
assert_eq!(
    app.clone().oneshot(save).await.unwrap().status(),
    StatusCode::OK
);

let skip = Request::post(format!("/api/reel/{id}/skip"))
    .body(Body::empty())
    .unwrap();
assert_eq!(app.oneshot(skip).await.unwrap().status(), StatusCode::OK);
```

Replace the second half of `reel_next_requires_auth_before_consuming_queue` after the initial unauthenticated assertion with this explicit authenticated app check:

```rust
let app = authenticated_test_app_with_candidates(vec![sample_repo("acme/oauth-reel", 502)]).await;
let response = app
    .oneshot(Request::post("/api/reel/next").body(Body::empty()).unwrap())
    .await
    .unwrap();
let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
let payload: Value = serde_json::from_slice(&body).unwrap();
assert_eq!(payload["repository"]["full_name"], "acme/oauth-reel");
```

Replace the start of `reel_previous_walks_back_through_view_history` through the third `next` response setup with:

```rust
let app = authenticated_test_app_with_candidates(vec![
    sample_repo("acme/first-history", 503),
    sample_repo("acme/second-history", 504),
    sample_repo("acme/third-history", 505),
])
.await;

let first_response = app
    .clone()
    .oneshot(Request::post("/api/reel/next").body(Body::empty()).unwrap())
    .await
    .unwrap();
let first_body = axum::body::to_bytes(first_response.into_body(), usize::MAX)
    .await
    .unwrap();
let first_payload: Value = serde_json::from_slice(&first_body).unwrap();

let second_response = app
    .clone()
    .oneshot(Request::post("/api/reel/next").body(Body::empty()).unwrap())
    .await
    .unwrap();
let second_body = axum::body::to_bytes(second_response.into_body(), usize::MAX)
    .await
    .unwrap();
let second_payload: Value = serde_json::from_slice(&second_body).unwrap();

let third_response = app
    .clone()
    .oneshot(Request::post("/api/reel/next").body(Body::empty()).unwrap())
    .await
    .unwrap();
let third_body = axum::body::to_bytes(third_response.into_body(), usize::MAX)
    .await
    .unwrap();
let third_payload: Value = serde_json::from_slice(&third_body).unwrap();
```

- [ ] **Step 7: Add a stale tokenless auth test**

In `server/tests/api_flow.rs`, add this test near `reel_next_requires_auth_before_consuming_queue`:

```rust
#[tokio::test]
async fn reel_next_rejects_tokenless_legacy_connection() {
    let config = Config::test();
    let pool = connect(&config).await.unwrap();
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token)
        VALUES (1, 1, 'local-dev', NULL)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState {
        repositories: RepositoryStore::new(pool.clone()),
        pool,
        config,
    };
    let app = Router::new()
        .nest("/api/reel", routes::reel::router())
        .with_state(state);

    let response = app
        .oneshot(Request::post("/api/reel/next").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert!(payload["repository"].is_null());
    assert_eq!(payload["empty_reason"], "auth_required");
}
```

- [ ] **Step 8: Run server tests and verify expected Discovery failures remain**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: FAIL only for tests or code still referencing `with_github_client`, GitHub token fallback, or seed fallback in `DiscoveryService`.

- [ ] **Step 9: Commit**

```bash
git add server/src/routes/auth.rs server/src/routes/reel.rs server/tests/api_flow.rs
git commit -m "refactor: remove dev connect api"
```

## Task 3: Restrict Discovery To OAuth Token Only

**Files:**
- Modify: `server/src/discovery.rs`
- Modify: `server/tests/api_flow.rs`

- [ ] **Step 1: Remove env-token client state from DiscoveryService**

In `server/src/discovery.rs`, change the struct to:

```rust
#[derive(Clone)]
pub struct DiscoveryService {
    store: RepositoryStore,
    oauth_github_client_factory: GitHubClientFactory,
}
```

Change `new()` to:

```rust
pub fn new(store: RepositoryStore) -> Self {
    Self {
        store,
        oauth_github_client_factory: Arc::new(|token| Arc::new(GitHubClient::new(token))),
    }
}
```

Delete the entire `with_github_client` method.

- [ ] **Step 2: Make `ensure_candidates()` OAuth-only with no seed fallback**

Replace `ensure_candidates()` with:

```rust
pub async fn ensure_candidates(&self) -> Result<(), ApiError> {
    // 候補が残っている間は補充せず、空になった時だけ保存済み OAuth token で補充を試す。
    if self.store.next_queued_repository().await?.is_some() {
        return Ok(());
    }

    if let Some(token) = self.store.auth_access_token().await? {
        let github_client = (self.oauth_github_client_factory)(token);
        if let Some(accepted) = self
            .try_github_discovery("recently_updated_oauth_search", github_client)
            .await?
        {
            if accepted > 0 {
                return Ok(());
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Remove normal seed fallback methods**

In `server/src/discovery.rs`, delete these items because tests can enqueue candidates explicitly:

```rust
pub async fn seed_if_empty(&self) -> Result<(), ApiError> {
    self.ensure_candidates().await
}

async fn enqueue_seed_candidates(&self) -> Result<usize, ApiError> { ... }

fn seed_repo(...) -> DiscoveryCandidate { ... }
```

After this deletion, `server/src/discovery.rs` should no longer contain `seed`, `rust-lang/rust`, `tauri-apps/tauri`, or `sqlite/sqlite`.

- [ ] **Step 4: Update Discovery tests for OAuth-only behavior**

In `server/tests/api_flow.rs`, delete these tests:

```rust
discovery_uses_github_candidates_when_queue_is_empty
discovery_falls_back_to_github_token_client_when_oauth_client_fails
discovery_uses_github_token_client_when_oauth_token_is_missing
discovery_falls_back_to_seed_without_github_client
discovery_falls_back_to_seed_when_github_fails
discovery_falls_back_to_seed_when_github_returns_no_accepted_candidates
```

Keep `discovery_prefers_oauth_token_client_when_auth_token_exists`, but remove `.with_github_client(Some(fallback_client))` and the unused `fallback_client` variable. The service setup should be:

```rust
let service = DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(
    |token| {
        assert_eq!(token, "gho_oauth_token");
        Arc::new(FakeGitHubClient {
            result: Ok((
                "oauth query".to_string(),
                vec![sample_repo("acme/oauth", 403)],
            )),
        })
    },
));
```

- [ ] **Step 5: Add tests for no-token and failed-OAuth empty queues**

In `server/tests/api_flow.rs`, add these tests after `discovery_prefers_oauth_token_client_when_auth_token_exists`:

```rust
#[tokio::test]
async fn discovery_leaves_queue_empty_without_oauth_token() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let service = DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(
        |_| panic!("OAuth client factory should not be called without a saved token"),
    ));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap();
    assert!(next.is_none());
}

#[tokio::test]
async fn discovery_leaves_queue_empty_when_oauth_client_fails() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool.clone());
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token)
        VALUES (1, 1, 'octocat', 'gho_expired_token')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let service = DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(
        |_| {
            Arc::new(FakeGitHubClient {
                result: Err(GitHubError::HttpStatus(StatusCode::UNAUTHORIZED)),
            })
        },
    ));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap();
    assert!(next.is_none());
}

#[tokio::test]
async fn discovery_leaves_queue_empty_when_oauth_returns_no_accepted_candidates() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool.clone());
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token)
        VALUES (1, 1, 'octocat', 'gho_oauth_token')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let service = DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(
        |_| {
            Arc::new(FakeGitHubClient {
                result: Ok(("oauth query".to_string(), Vec::new())),
            })
        },
    ));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap();
    assert!(next.is_none());
}
```

- [ ] **Step 6: Run server tests**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add server/src/discovery.rs server/tests/api_flow.rs server/src/routes/reel.rs
git commit -m "refactor: use oauth only discovery"
```

## Task 4: Update Web For OAuth-Only Connection

**Files:**
- Modify: `web/src/api/client.ts`
- Modify: `web/src/screens/ReelScreen.tsx`
- Modify: `web/src/App.test.tsx`

- [ ] **Step 1: Remove Web devConnect API client**

In `web/src/api/client.ts`, remove this property:

```ts
devConnect: (username = "local-dev") =>
  request<AuthState>("/api/auth/dev-connect", {
    method: "POST",
    body: JSON.stringify({ username })
  }),
```

- [ ] **Step 2: Remove dev-connect behavior from ReelScreen**

In `web/src/screens/ReelScreen.tsx`, replace `connect` with:

```tsx
const connect = async () => {
  if (!auth.oauth_configured) return;
  window.location.href = auth.oauth_start_url ?? "/api/auth/github/start";
};
```

Then replace the unauthenticated JSX block with:

```tsx
if (!auth.connected || emptyReason === "auth_required") {
  return (
    <section className="center-panel">
      <p className="eyebrow">Local-first discovery</p>
      <h1>GitHubに接続するとリールを開始できます</h1>
      <p>
        {auth.oauth_configured
          ? "GitHub OAuth で接続すると、保存済み OAuth token を使って実リポジトリ候補を取得します。"
          : "リールを開始するには GitHub OAuth の設定が必要です。GITHUB_CLIENT_ID と GITHUB_CLIENT_SECRET を設定してサーバーを起動してください。"}
      </p>
      {auth.oauth_configured ? (
        <button className="primary-button" onClick={connect} type="button">
          <UserCheck aria-hidden="true" size={18} />
          GitHubに接続
        </button>
      ) : null}
    </section>
  );
}
```

- [ ] **Step 3: Update default Web test fetch mock**

In `web/src/App.test.tsx`, remove the `/api/auth/dev-connect` response from the `beforeEach` mock. The default mock should start:

```ts
vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
  const path = String(input);
  if (path === "/api/auth/state") return Response.json({ connected: false, username: null, oauth_configured: false });
  if (path === "/api/reel/current") return Response.json({ repository: null, empty_reason: "auth_required" });
  if (path === "/api/reel/next") return Response.json({ repository: repo, empty_reason: null });
  if (path === "/api/reel/1/save") return Response.json({ ok: true });
  if (path === "/api/history") return Response.json([{ repository: repo, latest_event: "saved", latest_event_at: "2026-05-25T00:00:00Z" }]);
  return Response.json({ ok: true });
}));
```

- [ ] **Step 4: Replace the local dev-connect Web test**

In `web/src/App.test.tsx`, replace `OAuth 未設定のローカル環境では開発用接続でリールを表示できる` with:

```tsx
test("OAuth 未設定時は設定案内を表示して接続ボタンを出さない", async () => {
  render(<App />);

  await screen.findByText("GitHubに接続するとリールを開始できます");
  expect(screen.getByText("リールを開始するには GitHub OAuth の設定が必要です。GITHUB_CLIENT_ID と GITHUB_CLIENT_SECRET を設定してサーバーを起動してください。")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "開発用に接続" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "GitHubに接続" })).not.toBeInTheDocument();
});
```

- [ ] **Step 5: Ensure no Web test expects dev-connect**

Search: `dev-connect|開発用に接続|api.devConnect` in `web/src`.

Expected: no matches except a negative assertion for `開発用に接続` in `web/src/App.test.tsx`.

- [ ] **Step 6: Run Web tests and build**

Run: `npm --workspace web run test -- --run web/src/App.test.tsx`

Expected: PASS.

Run: `npm --workspace web run build`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add web/src/api/client.ts web/src/screens/ReelScreen.tsx web/src/App.test.tsx
git commit -m "refactor: require oauth connection in web"
```

## Task 5: Update E2E For OAuth Setup Guidance

**Files:**
- Modify: `e2e/git-reel.spec.ts`

- [ ] **Step 1: Replace dev-connect E2E flow**

Replace the full contents of `e2e/git-reel.spec.ts` with:

```ts
import { expect, test } from "@playwright/test";

test("OAuth 未設定時は設定案内を表示する", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("GitHubに接続するとリールを開始できます")).toBeVisible();
  await expect(page.getByText("リールを開始するには GitHub OAuth の設定が必要です。GITHUB_CLIENT_ID と GITHUB_CLIENT_SECRET を設定してサーバーを起動してください。")).toBeVisible();
  await expect(page.getByRole("button", { name: "開発用に接続" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "GitHubに接続" })).toHaveCount(0);
});
```

- [ ] **Step 2: Run E2E test**

Run: `npm run test:e2e`

Expected: PASS. If Chromium is not installed, run `npx playwright install chromium` once, then rerun `npm run test:e2e`.

- [ ] **Step 3: Commit**

```bash
git add e2e/git-reel.spec.ts
git commit -m "test: update e2e for oauth setup guidance"
```

## Task 6: Update Documentation And Agent Guidance

**Files:**
- Modify: `.env.example`
- Modify: `README.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Remove `GITHUB_TOKEN` from `.env.example`**

Edit `.env.example` so it contains only:

```dotenv
# SQLite の接続先。未設定時は sqlite:git-reel.db が使われます。
GIT_REEL_DATABASE_URL=sqlite:git-reel.db

# GitHub OAuth App の Client ID / Client Secret。
# ローカル開発でもリールを開始するには両方の設定が必要です。
GITHUB_CLIENT_ID=
GITHUB_CLIENT_SECRET=

# ローカル開発の既定 URL。
# GitHub OAuth App の callback URL は http://127.0.0.1:4317/api/auth/github/callback にします。
GIT_REEL_PUBLIC_BASE_URL=http://127.0.0.1:4317
GIT_REEL_PUBLIC_APP_URL=http://127.0.0.1:5173
```

- [ ] **Step 2: Update README connection policy**

In `README.md`, make these targeted changes:

- Replace descriptions saying OAuth or development connection is available with OAuth-only wording.
- Remove every mention of `GITHUB_TOKEN`.
- Remove every mention of `開発用接続` and `/api/auth/dev-connect`.
- Replace seed fallback descriptions with a statement that seed candidates are test-only and normal execution does not fall back to seed.
- In the environment variable table, remove the `GITHUB_TOKEN` row.
- Keep GitHub OAuth App callback URL instructions.

After editing, this command must return no matches:

```bash
rg "GITHUB_TOKEN|dev-connect|開発用接続|開発用に接続" README.md
```

- [ ] **Step 3: Update AGENTS.md runtime notes**

In `AGENTS.md`, replace lines that describe current fallback behavior with these statements:

```markdown
- 候補が空のときは、OAuth 接続済みの保存済み `auth_state.access_token` を使って GitHub Search API から最近更新されたリポジトリを補充する。OAuth token がない場合や取得失敗時、通常実行では seed へフォールバックしない。
- ローカル開発でもリールを開始するには `GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定し、GitHub OAuth で接続する。OAuth の URL 生成には `GIT_REEL_PUBLIC_BASE_URL` と `GIT_REEL_PUBLIC_APP_URL` を使い、未指定時はそれぞれ `http://127.0.0.1:4317` と `http://127.0.0.1:5173`。
- GitHub OAuth は接続状態とアクセストークンをローカル DB に保存する。現 MVP は GitHub への書き込みはしない。
```

In `# 構成メモ`, replace the Discovery and E2E bullets with:

```markdown
- 空のリール候補は `DiscoveryService::ensure_candidates` が保存済み OAuth token だけで補充を試す。候補追加や重複排除の挙動変更は `server/src/discovery.rs`、`server/src/github.rs`、repository store を確認する。
- seed 候補は通常実行の fallback ではなく、テスト用の候補準備に限定する。
- E2E は OAuth 未設定時の設定案内など、外部 GitHub OAuth に依存しない範囲を検証する。通常の単体テストと別に必要な変更だけ実行する。
```

- [ ] **Step 4: Search docs for stale connection references**

Run: `rg "GITHUB_TOKEN|dev-connect|開発用接続|開発用に接続|seed fallback" README.md AGENTS.md .env.example server web e2e`

Expected: no matches except historical design/plan docs under `docs/` are not included in this command. If matches remain in code or active docs, update them to OAuth-only wording.

- [ ] **Step 5: Commit**

```bash
git add .env.example README.md AGENTS.md
git commit -m "docs: document oauth only connection"
```

## Task 7: Final Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run all standard tests**

Run: `npm test`

Expected: PASS.

- [ ] **Step 2: Run Web build**

Run: `npm --workspace web run build`

Expected: PASS.

- [ ] **Step 3: Run E2E**

Run: `npm run test:e2e`

Expected: PASS.

- [ ] **Step 4: Run stale-reference search**

Run: `rg "GITHUB_TOKEN|dev-connect|開発用接続|開発用に接続" .env.example README.md AGENTS.md server web e2e`

Expected: no matches.

- [ ] **Step 5: Inspect final diff and status**

Run: `git status --short --branch`

Expected: clean working tree on `github-connect-cleanup` after all task commits.

Run: `git log --oneline -10`

Expected: recent commits include the design, plan, and implementation commits for the cleanup.

## Self-Review

- Spec coverage: The plan covers dev-connect deletion, `GITHUB_TOKEN` deletion, OAuth-only Discovery, seed removal from normal execution, Web connection UX, E2E adjustment, README, `.env.example`, and `AGENTS.md` updates.
- Placeholder scan: The plan avoids unfinished markers and vague implementation steps. Each code-changing step names exact files and concrete code or replacement behavior.
- Type consistency: `Config`, `AppState`, `DiscoveryService`, `AuthState`, and route paths are consistent across tasks. The integration-test helper builds the same route tree as `build_app_with_config` without reintroducing `github_client`.
