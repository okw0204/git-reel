# Real GitHub Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `GITHUB_TOKEN` が設定されているとき、空の discovery queue を GitHub Search API の実候補で補充し、失敗時や token なしでは既存の開発用 seed にフォールバックする。

**Architecture:** GitHub Search API の HTTP 処理とクエリ構築は `server/src/github.rs` に閉じ込める。`DiscoveryService` は `Arc<dyn GitHubDiscoveryClient>` を任意で受け取り、queue が空のときだけ実 discovery を試してから既存の `enqueue_candidates` に渡す。route は API 契約を変えず、`AppState` に保持した client を `DiscoveryService` へ渡すだけにする。

**Tech Stack:** Rust 2021, axum, reqwest, sqlx SQLite, chrono, async-trait, tokio, cargo test

---

## File Structure

- Modify: `server/src/github.rs`
  - `GitHubError` に HTTP 系エラーを追加する。
  - `GitHubDiscoveryClient` trait を定義し、`GitHubClient` が実装する。
  - `build_recently_updated_search_query(today: NaiveDate) -> String` を公開し、90 日前の `pushed:>` 条件を組み立てる。
  - `recently_updated_search_query() -> String` と `search_recently_updated_repositories()` を追加する。
- Modify: `server/src/discovery.rs`
  - `DiscoveryService` に任意の GitHub client を保持させる。
  - `with_github_client()` と `ensure_candidates()` を追加する。
  - `seed_if_empty()` は既存テストと呼び出し互換のため残し、実装を `ensure_candidates()` に委譲する。
- Modify: `server/src/app.rs`
  - `AppState` に `github_client: Option<Arc<dyn GitHubDiscoveryClient>>` を追加する。
  - `Config.github_token` から `GitHubClient` を作り、token なしのテスト・ローカルでは `None` にする。
- Modify: `server/src/routes/reel.rs`
  - `DiscoveryService::new(...).with_github_client(state.github_client.clone()).ensure_candidates()` を `current` と `next` で使う。
- Modify: `server/tests/github_fixtures.rs`
  - Search API fixture 変換テストを維持し、クエリ構築テストを追加する。
- Modify: `server/tests/api_flow.rs`
  - fake GitHub client による実候補 enqueue、GitHub discovery failure 時の seed fallback、token/client なし fallback を検証する。

---

### Task 1: GitHub Search Query Builder

**Files:**
- Modify: `server/src/github.rs:1-93`
- Test: `server/tests/github_fixtures.rs:1-31`

- [ ] **Step 1: Write the failing query builder test**

Add `chrono::NaiveDate` to the imports and append this test to `server/tests/github_fixtures.rs`:

```rust
use chrono::NaiveDate;
use git_reel_server::github::{
    build_recently_updated_search_query, parse_graphql_readme_preview, parse_search_response,
};

#[test]
fn builds_recently_updated_live_search_query() {
    let query = build_recently_updated_search_query(NaiveDate::from_ymd_opt(2026, 5, 28).unwrap());

    assert_eq!(
        query,
        "stars:10..5000 fork:false archived:false pushed:>2026-02-27 sort:updated-desc"
    );
}
```

After editing, the top of `server/tests/github_fixtures.rs` should be:

```rust
use chrono::NaiveDate;
use git_reel_server::github::{
    build_recently_updated_search_query, parse_graphql_readme_preview, parse_search_response,
};
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml builds_recently_updated_live_search_query`

Expected: FAIL with an unresolved import error for `build_recently_updated_search_query`.

- [ ] **Step 3: Add the minimal query builder**

In `server/src/github.rs`, replace the imports at the top:

```rust
use crate::models::NewRepository;
use chrono::{Duration, NaiveDate};
use serde::Deserialize;
```

Add this function above `parse_search_response`:

```rust
pub fn build_recently_updated_search_query(today: NaiveDate) -> String {
    let pushed_after = today - Duration::days(90);
    format!(
        "stars:10..5000 fork:false archived:false pushed:>{} sort:updated-desc",
        pushed_after.format("%Y-%m-%d")
    )
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml builds_recently_updated_live_search_query`

Expected: PASS for `builds_recently_updated_live_search_query`.

- [ ] **Step 5: Run existing fixture tests**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: PASS for all tests in `server/tests/github_fixtures.rs`.

- [ ] **Step 6: Commit**

```bash
git add server/src/github.rs server/tests/github_fixtures.rs
git commit -m "test: cover live discovery query"
```

---

### Task 2: Live GitHub Search Client

**Files:**
- Modify: `server/src/github.rs:1-120`
- Test: `server/tests/github_fixtures.rs:1-45`

- [ ] **Step 1: Write the failing trait and client construction test**

Append this test to `server/tests/github_fixtures.rs`:

```rust
#[test]
fn creates_github_client_from_token() {
    let client = git_reel_server::github::GitHubClient::new("secret-token".to_string());

    assert_eq!(client.token(), "secret-token");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml creates_github_client_from_token`

Expected: FAIL with an unresolved type error for `GitHubClient`.

- [ ] **Step 3: Add GitHub client, trait, and HTTP error types**

In `server/src/github.rs`, replace the current `GitHubError` enum with:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("github http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("github http status: {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
```

Add these imports to `server/src/github.rs`:

```rust
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
```

The complete import block should be:

```rust
use crate::models::NewRepository;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
```

Add this code above the response structs:

```rust
#[async_trait]
pub trait GitHubDiscoveryClient: Send + Sync {
    async fn search_recently_updated_repositories(&self) -> Result<(String, Vec<NewRepository>), GitHubError>;
}

pub struct GitHubClient {
    token: String,
    http: reqwest::Client,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            http: reqwest::Client::new(),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}

#[async_trait]
impl GitHubDiscoveryClient for GitHubClient {
    async fn search_recently_updated_repositories(&self) -> Result<(String, Vec<NewRepository>), GitHubError> {
        let query = recently_updated_search_query();
        let response = self
            .http
            .get("https://api.github.com/search/repositories")
            .header(USER_AGENT, "git-reel")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .query(&[("q", query.as_str()), ("per_page", "30")])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(GitHubError::HttpStatus(status));
        }

        let body = response.text().await?;
        let repositories = parse_search_response(&body)?;
        Ok((query, repositories))
    }
}

fn recently_updated_search_query() -> String {
    build_recently_updated_search_query(Utc::now().date_naive())
}
```

- [ ] **Step 4: Run client construction test**

Run: `cargo test --manifest-path server/Cargo.toml creates_github_client_from_token`

Expected: PASS for `creates_github_client_from_token`.

- [ ] **Step 5: Run all GitHub fixture tests**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: PASS for all tests in `server/tests/github_fixtures.rs`. No live GitHub request is made by these tests.

- [ ] **Step 6: Commit**

```bash
git add server/src/github.rs server/tests/github_fixtures.rs
git commit -m "feat: add github search client"
```

---

### Task 3: Discovery Service Live Candidate Fallback

**Files:**
- Modify: `server/src/discovery.rs:1-104`
- Test: `server/tests/api_flow.rs:1-372`

- [ ] **Step 1: Write fake GitHub client helpers**

Update the imports at the top of `server/tests/api_flow.rs` to include the trait and `Arc`:

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use git_reel_server::{
    config::Config,
    db::connect,
    discovery::{DiscoveryCandidate, DiscoveryService},
    github::{GitHubDiscoveryClient, GitHubError},
    models::{NewRepository, RepoEventKind},
    repositories::RepositoryStore,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
```

Add this helper code after `sample_repo`:

```rust
struct FakeGitHubClient {
    result: Result<(String, Vec<NewRepository>), GitHubError>,
}

#[async_trait::async_trait]
impl GitHubDiscoveryClient for FakeGitHubClient {
    async fn search_recently_updated_repositories(&self) -> Result<(String, Vec<NewRepository>), GitHubError> {
        match &self.result {
            Ok((query, repositories)) => Ok((query.clone(), repositories.clone())),
            Err(_) => Err(GitHubError::HttpStatus(StatusCode::FORBIDDEN)),
        }
    }
}
```

- [ ] **Step 2: Write failing test for live candidates entering the queue**

Append this test to `server/tests/api_flow.rs`:

```rust
#[tokio::test]
async fn discovery_uses_github_candidates_when_queue_is_empty() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let service = DiscoveryService::new(store.clone()).with_github_client(Some(Arc::new(FakeGitHubClient {
        result: Ok((
            "stars:10..5000 fork:false archived:false pushed:>2026-02-27 sort:updated-desc".to_string(),
            vec![sample_repo("acme/live", 401)],
        )),
    })));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/live");
}
```

- [ ] **Step 3: Write failing tests for fallback cases**

Append these tests to `server/tests/api_flow.rs`:

```rust
#[tokio::test]
async fn discovery_falls_back_to_seed_without_github_client() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let service = DiscoveryService::new(store.clone());

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "rust-lang/rust");
}

#[tokio::test]
async fn discovery_falls_back_to_seed_when_github_fails() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let service = DiscoveryService::new(store.clone()).with_github_client(Some(Arc::new(FakeGitHubClient {
        result: Err(GitHubError::HttpStatus(StatusCode::FORBIDDEN)),
    })));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "rust-lang/rust");
}

#[tokio::test]
async fn discovery_falls_back_to_seed_when_github_returns_no_accepted_candidates() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool.clone());
    let service = DiscoveryService::new(store.clone()).with_github_client(Some(Arc::new(FakeGitHubClient {
        result: Ok((
            "stars:10..5000 fork:false archived:false pushed:>2026-02-27 sort:updated-desc".to_string(),
            Vec::new(),
        )),
    })));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "rust-lang/rust");
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test --manifest-path server/Cargo.toml discovery_`

Expected: FAIL for the newly added discovery tests with unresolved method errors for `with_github_client` and `ensure_candidates`.

- [ ] **Step 5: Update `DiscoveryService` structure and constructor**

In `server/src/discovery.rs`, replace the imports with:

```rust
use crate::{
    error::ApiError,
    github::GitHubDiscoveryClient,
    models::NewRepository,
    repositories::RepositoryStore,
};
use std::{collections::HashSet, sync::Arc};
```

Replace the `DiscoveryService` struct with:

```rust
#[derive(Clone)]
pub struct DiscoveryService {
    store: RepositoryStore,
    github_client: Option<Arc<dyn GitHubDiscoveryClient>>,
}
```

Replace `DiscoveryService::new` with:

```rust
pub fn new(store: RepositoryStore) -> Self {
    Self {
        store,
        github_client: None,
    }
}

pub fn with_github_client(mut self, github_client: Option<Arc<dyn GitHubDiscoveryClient>>) -> Self {
    self.github_client = github_client;
    self
}
```

- [ ] **Step 6: Add live discovery before seed fallback**

Replace `seed_if_empty` in `server/src/discovery.rs` with these methods:

```rust
pub async fn ensure_candidates(&self) -> Result<(), ApiError> {
    if self.store.next_queued_repository().await?.is_some() {
        return Ok(());
    }

    if let Some(github_client) = self.github_client.as_ref() {
        match github_client.search_recently_updated_repositories().await {
            Ok((query, repositories)) => {
                let candidates = repositories
                    .into_iter()
                    .map(DiscoveryCandidate::from_new_repository)
                    .collect();
                let accepted = self
                    .enqueue_candidates("recently_updated_live_search", &query, candidates)
                    .await?;
                if accepted > 0 {
                    return Ok(());
                }
            }
            Err(error) => {
                tracing::warn!(?error, "github discovery failed; falling back to seed repositories");
            }
        }
    }

    self.enqueue_seed_candidates().await?;
    Ok(())
}

pub async fn seed_if_empty(&self) -> Result<(), ApiError> {
    self.ensure_candidates().await
}

async fn enqueue_seed_candidates(&self) -> Result<usize, ApiError> {
    self.enqueue_candidates(
        "seed",
        "local fixture seed",
        vec![
            seed_repo("rust-lang/rust", 1, "Rust", 98000),
            seed_repo("tauri-apps/tauri", 2, "Rust", 88000),
            seed_repo("sqlite/sqlite", 3, "C", 7000),
        ],
    )
    .await
}
```

- [ ] **Step 7: Run focused discovery tests**

Run: `cargo test --manifest-path server/Cargo.toml discovery_`

Expected: PASS for discovery tests, including the new live-candidate and fallback tests.

- [ ] **Step 8: Run existing queue tests**

Run: `cargo test --manifest-path server/Cargo.toml discovery_queue`

Expected: PASS for `discovery_queue_excludes_viewed_and_skipped_repositories` and `discovery_queue_deduplicates_candidates_in_one_batch`.

Run: `cargo test --manifest-path server/Cargo.toml claim_next_queued_repository_consumes_each_row_once`

Expected: PASS for `claim_next_queued_repository_consumes_each_row_once`.

Run: `cargo test --manifest-path server/Cargo.toml reel_next_requires_auth_before_consuming_queue`

Expected: PASS. `reel_next_requires_auth_before_consuming_queue` still returns `auth_required` before any queue mutation and still returns `rust-lang/rust` after dev connect when no GitHub client is configured.

- [ ] **Step 9: Commit**

```bash
git add server/src/discovery.rs server/tests/api_flow.rs
git commit -m "feat: use github discovery with seed fallback"
```

---

### Task 4: Wire GitHub Client Into App State And Reel Routes

**Files:**
- Modify: `server/src/app.rs:1-39`
- Modify: `server/src/routes/reel.rs:1-68`
- Test: `server/tests/api_flow.rs:193-264`

- [ ] **Step 1: Update AppState to hold the optional client**

In `server/src/app.rs`, replace the imports with:

```rust
use crate::{
    config::Config,
    db::connect,
    github::{GitHubClient, GitHubDiscoveryClient},
    repositories::RepositoryStore,
    routes,
};
use axum::{routing::get, Router};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
```

Replace `AppState` with:

```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub repositories: RepositoryStore,
    pub github_client: Option<Arc<dyn GitHubDiscoveryClient>>,
}
```

In `build_app_with_config`, replace the `state` construction with:

```rust
let github_client = config
    .github_token
    .clone()
    .map(|token| Arc::new(GitHubClient::new(token)) as Arc<dyn GitHubDiscoveryClient>);
let state = AppState {
    repositories: RepositoryStore::new(pool.clone()),
    github_client,
    pool,
};
```

- [ ] **Step 2: Update reel routes to use `ensure_candidates`**

In `server/src/routes/reel.rs`, replace the `DiscoveryService` calls in `current` and `next`.

For `current`, replace lines 32-34 with:

```rust
DiscoveryService::new(state.repositories.clone())
    .with_github_client(state.github_client.clone())
    .ensure_candidates()
    .await?;
```

For `next`, replace lines 55-57 with:

```rust
DiscoveryService::new(state.repositories.clone())
    .with_github_client(state.github_client.clone())
    .ensure_candidates()
    .await?;
```

- [ ] **Step 3: Run route-level regression tests**

Run: `cargo test --manifest-path server/Cargo.toml reel_`

Expected: PASS for reel route tests. Response bodies keep the existing `repository` and `empty_reason` shape.

- [ ] **Step 4: Run server tests**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: PASS for all server tests. No test should perform a live GitHub request because `Config::test().github_token` is `None` and service tests use `FakeGitHubClient`.

- [ ] **Step 5: Commit**

```bash
git add server/src/app.rs server/src/routes/reel.rs server/tests/api_flow.rs
git commit -m "feat: wire github discovery into reel routes"
```

---

### Task 5: Final Verification

**Files:**
- Verify: `server/src/github.rs`
- Verify: `server/src/discovery.rs`
- Verify: `server/src/app.rs`
- Verify: `server/src/routes/reel.rs`
- Verify: `server/tests/github_fixtures.rs`
- Verify: `server/tests/api_flow.rs`

- [ ] **Step 1: Run backend test suite**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: PASS for all Rust tests.

- [ ] **Step 2: Run full default test suite**

Run: `npm test`

Expected: PASS for `npm run test:web` and `npm run test:server`. E2E is not included in `npm test` by repository convention.

- [ ] **Step 3: Check API contract manually from tests**

Inspect `server/tests/api_flow.rs` and confirm these assertions still exist and pass:

```rust
assert!(payload["repository"].is_null());
assert_eq!(payload["empty_reason"], "auth_required");
assert_eq!(payload["repository"]["full_name"], "rust-lang/rust");
```

- [ ] **Step 4: Check live discovery code path without committing secrets**

Run this command only if `GITHUB_TOKEN` is already present in the shell. Do not paste or commit token values.

Run: `GIT_REEL_DATABASE_URL=sqlite::memory: cargo run --manifest-path server/Cargo.toml`

Expected: server starts and logs `git-reel server listening on http://127.0.0.1:4317`. Stop it with `Ctrl-C` after startup confirmation.

- [ ] **Step 5: Commit verification-only fixes if any were needed**

If verification required code changes, commit only those files:

```bash
git add server/src/github.rs server/src/discovery.rs server/src/app.rs server/src/routes/reel.rs server/tests/github_fixtures.rs server/tests/api_flow.rs
git commit -m "fix: stabilize github discovery tests"
```

If no files changed during verification, do not create a commit.

---

## Self-Review Notes

- Spec coverage: tokenありの GitHub Search API 取得は Task 2 と Task 4、tokenなし fallback は Task 3、API失敗 fallback は Task 3、queue・batch・重複排除の再利用は `enqueue_candidates` を通す Task 3、live API 非依存テストは Task 2 と Task 3、フロントエンド API 契約維持は Task 4 と Task 5 で扱う。
- Placeholder scan: スキルで禁止されている未決定表現を避けた。各コード変更ステップには具体的な Rust コードと実行コマンドを記載した。
- Type consistency: `GitHubDiscoveryClient::search_recently_updated_repositories` は Task 2 で定義し、Task 3 の fake client と `DiscoveryService` が同じシグネチャを使う。`DiscoveryService::with_github_client` は `Option<Arc<dyn GitHubDiscoveryClient>>` を受け取り、Task 4 の `AppState.github_client` と一致する。
