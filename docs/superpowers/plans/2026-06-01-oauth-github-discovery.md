# OAuth トークンによる GitHub Discovery 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** OAuth 接続済みユーザーの保存済み `auth_state.access_token` を優先して GitHub Discovery を実行し、Search API 候補に README preview を補完する。

**Architecture:** `routes/reel.rs` は変更せず、既存通り `DiscoveryService::ensure_candidates()` を呼ぶ。`DiscoveryService` が queue 空時に `RepositoryStore` から OAuth token を読み、OAuth token 用の一時 `GitHubClient`、`GITHUB_TOKEN` 由来 fallback client、ローカル seed の順に候補取得を試す。GitHub HTTP 呼び出しと JSON 変換は `github.rs` に閉じ込め、Web と README は実挙動に合わせて文言だけ更新する。

**Tech Stack:** Rust, axum, reqwest, sqlx SQLite, async-trait, serde, React, TypeScript, Vitest, Vite

---

## ファイル構成

- Modify: `server/src/repositories.rs`
- 役割: `auth_state` の id 1 から、接続済みかつ `access_token IS NOT NULL` の OAuth token だけを返す `auth_access_token()` を追加する。
- Modify: `server/src/discovery.rs`
- 役割: OAuth token を優先して GitHub discovery を実行し、失敗または採用候補 0 件の場合は既存の `GITHUB_TOKEN` client、最後に seed へフォールバックする。
- Modify: `server/src/github.rs`
- 役割: Search API の結果を `NewRepository` に変換した後、GraphQL API で README preview を候補ごとに補完する。README 取得失敗は候補取得全体の失敗にしない。
- Modify: `server/tests/api_flow.rs`
- 役割: OAuth token 取得、Discovery の token 優先順位、fallback 挙動を live GitHub API なしで検証する。
- Modify: `server/tests/github_fixtures.rs`
- 役割: Search API fixture と GraphQL README fixture の JSON 変換を明示的に検証する。
- Modify: `web/src/screens/ReelScreen.tsx`
- 役割: OAuth 接続後も seed を使うという古い未接続説明文を、実データ取得に合わせて更新する。
- Modify: `web/src/App.test.tsx`
- 役割: 未接続画面の説明文が新しい挙動と矛盾しないことを検証する。
- Modify: `README.md`
- 役割: `GITHUB_TOKEN` は fallback 用であり、OAuth 接続済みの場合は保存済み OAuth token が優先されることを明記する。
- Modify: `AGENTS.md`
- 役割: 実行時の注意と構成メモにある discovery の token 優先順位を、OAuth token 優先の新しい挙動に合わせる。
- No change: `Makefile`
- 理由: 既存の `dev`, `build`, `test`, `test-web`, `test-server`, `test-e2e` は今回の OAuth token discovery 変更後も同じコマンドで足りる。新しいターゲットや環境変数注入は不要。

## タスク

### Task 1: 保存済み OAuth token の取得

**Files:**
- Modify: `server/src/repositories.rs:12-17`
- Test: `server/tests/api_flow.rs`

- [ ] **Step 1: 失敗する store テストを追加する**

`server/tests/api_flow.rs` の `auth_state_starts_disconnected_and_dev_connect_sets_user` の後に追加する。

```rust
#[tokio::test]
async fn store_returns_connected_auth_access_token() {
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

    let token = store.auth_access_token().await.unwrap();

    assert_eq!(token.as_deref(), Some("gho_oauth_token"));
}

#[tokio::test]
async fn store_does_not_return_auth_access_token_when_disconnected_or_missing() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool.clone());

    let missing = store.auth_access_token().await.unwrap();
    assert_eq!(missing, None);

    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username, access_token)
        VALUES (1, 0, 'octocat', 'gho_disconnected_token')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let disconnected = store.auth_access_token().await.unwrap();
    assert_eq!(disconnected, None);

    sqlx::query(
        r#"
        UPDATE auth_state
        SET connected = 1, access_token = NULL
        WHERE id = 1
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let tokenless = store.auth_access_token().await.unwrap();
    assert_eq!(tokenless, None);
}
```

- [ ] **Step 2: 失敗を確認する**

Run: `cargo test --manifest-path server/Cargo.toml auth_access_token`

Expected: `no method named 'auth_access_token' found for struct 'RepositoryStore'` で失敗する。

- [ ] **Step 3: `RepositoryStore::auth_access_token()` を追加する**

`server/src/repositories.rs` の `impl RepositoryStore` 内、`new()` の直後に追加する。

```rust
    pub async fn auth_access_token(&self) -> Result<Option<String>, ApiError> {
        let token = sqlx::query_scalar(
            r#"
            SELECT access_token
            FROM auth_state
            WHERE id = 1
              AND connected = 1
              AND access_token IS NOT NULL
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }
```

- [ ] **Step 4: store テストの成功を確認する**

Run: `cargo test --manifest-path server/Cargo.toml auth_access_token`

Expected: 追加した 2 件のテストが PASS する。

- [ ] **Step 5: コミットする**

```bash
git add server/src/repositories.rs server/tests/api_flow.rs
git commit -m "feat: read oauth token for discovery"
```

### Task 2: Discovery の OAuth token 優先と fallback

**Files:**
- Modify: `server/src/discovery.rs:1-129`
- Modify: `server/tests/api_flow.rs:36-50,232-295`
- Test: `server/tests/api_flow.rs`

- [ ] **Step 1: 失敗する Discovery 優先順位テストを追加する**

`server/tests/api_flow.rs` の `discovery_uses_github_candidates_when_queue_is_empty` の後に追加する。

```rust
#[tokio::test]
async fn discovery_prefers_oauth_token_client_when_auth_token_exists() {
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

    let fallback_client = Arc::new(FakeGitHubClient {
        result: Ok(("fallback query".to_string(), vec![sample_repo("acme/fallback", 402)])),
    });
    let service = DiscoveryService::new(store.clone())
        .with_github_client(Some(fallback_client))
        .with_oauth_github_client_factory(Arc::new(|token| {
            assert_eq!(token, "gho_oauth_token");
            Arc::new(FakeGitHubClient {
                result: Ok(("oauth query".to_string(), vec![sample_repo("acme/oauth", 403)])),
            })
        }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/oauth");
}

#[tokio::test]
async fn discovery_falls_back_to_github_token_client_when_oauth_client_fails() {
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

    let fallback_client = Arc::new(FakeGitHubClient {
        result: Ok(("fallback query".to_string(), vec![sample_repo("acme/fallback", 404)])),
    });
    let service = DiscoveryService::new(store.clone())
        .with_github_client(Some(fallback_client))
        .with_oauth_github_client_factory(Arc::new(|_| {
            Arc::new(FakeGitHubClient {
                result: Err(GitHubError::HttpStatus(StatusCode::UNAUTHORIZED)),
            })
        }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/fallback");
}

#[tokio::test]
async fn discovery_uses_github_token_client_when_oauth_token_is_missing() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let fallback_client = Arc::new(FakeGitHubClient {
        result: Ok(("fallback query".to_string(), vec![sample_repo("acme/fallback-only", 405)])),
    });
    let service = DiscoveryService::new(store.clone())
        .with_github_client(Some(fallback_client))
        .with_oauth_github_client_factory(Arc::new(|_| {
            panic!("OAuth client factory should not be called without a saved token")
        }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/fallback-only");
}
```

- [ ] **Step 2: 失敗を確認する**

Run: `cargo test --manifest-path server/Cargo.toml discovery_`

Expected: `no method named 'with_oauth_github_client_factory' found for struct 'DiscoveryService'` で失敗する。

- [ ] **Step 3: `DiscoveryService` に OAuth client factory を追加する**

`server/src/discovery.rs` の import と構造体定義を次の形にする。

```rust
use crate::{
    error::ApiError,
    github::{GitHubClient, GitHubDiscoveryClient},
    models::NewRepository,
    repositories::RepositoryStore,
};
use std::{collections::HashSet, sync::Arc};

pub type GitHubClientFactory = Arc<dyn Fn(String) -> Arc<dyn GitHubDiscoveryClient> + Send + Sync>;

#[derive(Clone)]
pub struct DiscoveryService {
    store: RepositoryStore,
    github_client: Option<Arc<dyn GitHubDiscoveryClient>>,
    oauth_github_client_factory: GitHubClientFactory,
}
```

`DiscoveryService::new()` と builder メソッド群を次の形にする。

```rust
    pub fn new(store: RepositoryStore) -> Self {
        Self {
            store,
            github_client: None,
            oauth_github_client_factory: Arc::new(|token| Arc::new(GitHubClient::new(token))),
        }
    }

    pub fn with_github_client(
        mut self,
        github_client: Option<Arc<dyn GitHubDiscoveryClient>>,
    ) -> Self {
        self.github_client = github_client;
        self
    }

    pub fn with_oauth_github_client_factory(
        mut self,
        factory: GitHubClientFactory,
    ) -> Self {
        self.oauth_github_client_factory = factory;
        self
    }
```

- [ ] **Step 4: GitHub discovery の共通 helper を追加する**

`server/src/discovery.rs` の `ensure_candidates()` の前に追加する。

```rust
    async fn try_github_discovery(
        &self,
        strategy: &str,
        github_client: Arc<dyn GitHubDiscoveryClient>,
    ) -> Result<Option<usize>, ApiError> {
        match github_client.search_recently_updated_repositories().await {
            Ok((query, repositories)) => {
                let candidates = repositories
                    .into_iter()
                    .map(DiscoveryCandidate::from_new_repository)
                    .collect();
                let accepted = self.enqueue_candidates(strategy, &query, candidates).await?;
                Ok(Some(accepted))
            }
            Err(error) => {
                tracing::warn!(?error, strategy, "github discovery failed; trying fallback");
                Ok(None)
            }
        }
    }
```

- [ ] **Step 5: `ensure_candidates()` を token 優先順に置き換える**

`server/src/discovery.rs` の `ensure_candidates()` 全体を次の内容に置き換える。

```rust
    pub async fn ensure_candidates(&self) -> Result<(), ApiError> {
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

        if let Some(github_client) = self.github_client.as_ref() {
            if let Some(accepted) = self
                .try_github_discovery("recently_updated_live_search", github_client.clone())
                .await?
            {
                if accepted > 0 {
                    return Ok(());
                }
            }
        }

        self.enqueue_seed_candidates().await?;
        Ok(())
    }
```

- [ ] **Step 6: Discovery テストの成功を確認する**

Run: `cargo test --manifest-path server/Cargo.toml discovery_`

Expected: 既存の `discovery_...` テストと追加した 3 件が PASS する。

- [ ] **Step 7: コミットする**

```bash
git add server/src/discovery.rs server/tests/api_flow.rs
git commit -m "feat: prefer oauth token for discovery"
```

### Task 3: README preview の GraphQL 補完

**Files:**
- Modify: `server/src/github.rs:1-225`
- Modify: `server/tests/github_fixtures.rs:1-67`
- Test: `server/tests/github_fixtures.rs`

- [ ] **Step 1: Search fixture の README 未補完状態を明示するテストを追加する**

`server/tests/github_fixtures.rs` の `converts_search_response_to_new_repository` に次の assertion を追加する。

```rust
    assert_eq!(repositories[0].readme_preview, None);
```

- [ ] **Step 2: GraphQL README fixture の object null ケースを追加する**

`server/tests/github_fixtures.rs` の `returns_none_for_nullable_graphql_repository` の後に追加する。

```rust
#[test]
fn returns_none_for_nullable_graphql_readme_object() {
    let fixture = r#"{"data":{"repository":{"object":null}}}"#;
    let preview = parse_graphql_readme_preview(fixture).unwrap();
    assert_eq!(preview, None);
}
```

- [ ] **Step 3: fixture テストの成功を確認する**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: Search fixture と GraphQL fixture の既存変換が PASS し、README 未補完状態と `object: null` も PASS する。

- [ ] **Step 4: GraphQL request/response 型を追加する**

`server/src/github.rs` の import を次の形にする。

```rust
use crate::models::NewRepository;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
```

`ReadmeObject` の後に追加する。

```rust
#[derive(Serialize)]
struct GraphQlRequest<'a> {
    query: &'a str,
    variables: GraphQlReadmeVariables<'a>,
}

#[derive(Serialize)]
struct GraphQlReadmeVariables<'a> {
    owner: &'a str,
    name: &'a str,
}

const README_PREVIEW_QUERY: &str = r#"
query RepositoryReadme($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    object(expression: "HEAD:README.md") {
      ... on Blob {
        text
      }
    }
  }
}
"#;
```

- [ ] **Step 5: `GitHubClient` に README GraphQL 呼び出しを追加する**

`impl GitHubClient` 内、`token()` の後に追加する。

```rust
    async fn readme_preview(&self, owner: &str, name: &str) -> Result<Option<String>, GitHubError> {
        let response = self
            .http
            .post("https://api.github.com/graphql")
            .header(USER_AGENT, "git-reel")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .json(&GraphQlRequest {
                query: README_PREVIEW_QUERY,
                variables: GraphQlReadmeVariables { owner, name },
            })
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(GitHubError::HttpStatus(status));
        }

        let body = response.text().await?;
        parse_graphql_readme_preview(&body)
    }
```

- [ ] **Step 6: Search API 結果に README preview を補完する**

`GitHubDiscoveryClient for GitHubClient` の `search_recently_updated_repositories()` の末尾を次の形に置き換える。

```rust
        let body = response.text().await?;
        let mut repositories = parse_search_response(&body)?;
        for repository in repositories.iter_mut() {
            match self.readme_preview(&repository.owner, &repository.name).await {
                Ok(preview) => {
                    repository.readme_preview = preview;
                }
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        repository = %repository.full_name,
                        "github readme preview failed; keeping repository without preview"
                    );
                }
            }
        }
        Ok((query, repositories))
```

- [ ] **Step 7: GitHub fixture と github unit test を実行する**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures && cargo test --manifest-path server/Cargo.toml github_`

Expected: fixture テストと `github.rs` の unit test が PASS する。live GitHub API には接続しない。

- [ ] **Step 8: コミットする**

```bash
git add server/src/github.rs server/tests/github_fixtures.rs
git commit -m "feat: enrich discovery candidates with readme previews"
```

### Task 4: UI と README の文言更新

**Files:**
- Modify: `web/src/screens/ReelScreen.tsx:108-121`
- Modify: `web/src/App.test.tsx:41-73`
- Modify: `README.md:3-130`
- Modify: `AGENTS.md:16-30`
- No change: `Makefile`
- Test: `web/src/App.test.tsx`

- [ ] **Step 1: 失敗する UI 文言テストを追加する**

`web/src/App.test.tsx` の `未接続時に GitHub OAuth 接続へ遷移できる` で、`await screen.findByText("GitHubに接続するとリールを開始できます");` の直後に追加する。

```tsx
    expect(screen.getByText("GitHub OAuth で接続すると、保存済み OAuth token を使って実リポジトリ候補を取得します。")).toBeInTheDocument();
```

`OAuth 未設定のローカル環境では開発用接続でリールを表示できる` で、`await screen.findByText("GitHubに接続するとリールを開始できます");` の直後に追加する。

```tsx
    expect(screen.getByText("OAuth 未設定のローカル環境では開発用接続でシード候補を使います。")).toBeInTheDocument();
```

- [ ] **Step 2: 失敗を確認する**

Run: `npm --workspace web run test -- --run web/src/App.test.tsx`

Expected: OAuth 設定ありケースで、期待した新しい説明文が見つからず FAIL する。

- [ ] **Step 3: 未接続画面の OAuth 説明文を更新する**

`web/src/screens/ReelScreen.tsx` の説明文 ternary を次の内容に置き換える。

```tsx
          {auth.oauth_configured
            ? "GitHub OAuth で接続すると、保存済み OAuth token を使って実リポジトリ候補を取得します。"
            : "OAuth 未設定のローカル環境では開発用接続でシード候補を使います。"}
```

- [ ] **Step 4: README の概要と機能を更新する**

`README.md:5` を次の内容に置き換える。

```markdown
現在の MVP はローカル開発向けです。GitHub への書き込みは行わず、OAuth 接続または開発用接続でリール操作、保存、スキップ、履歴、メモ、タグを確認できます。OAuth 接続済みの場合は保存済み OAuth token を優先して GitHub Search API から候補を補充し、未接続または取得失敗時は `GITHUB_TOKEN`、最後にシードされた候補リポジトリへフォールバックします。
```

`README.md:15` を次の内容に置き換える。

```markdown
- OAuth 接続済み token または `GITHUB_TOKEN` 設定時の GitHub Search API による候補補充
```

- [ ] **Step 5: README の開発サーバー説明を更新する**

`README.md:53-55` を次の内容に置き換える。

```markdown
ブラウザで Vite が表示する URL を開き、リール画面の「開発用に接続」からローカルのシード候補を閲覧できます。

`GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定して起動すると、リール画面の接続ボタンは GitHub OAuth 接続になります。OAuth のコールバック URL は既定で `http://127.0.0.1:4317/api/auth/github/callback` です。OAuth 接続後は保存済み OAuth token を優先して GitHub Search API から候補を取得します。
```

- [ ] **Step 6: README の環境変数説明と設計メモを更新する**

`README.md:74` を次の内容に置き換える。

```markdown
| `GITHUB_TOKEN` | OAuth token がない場合の GitHub Search API fallback 用トークン | 未設定 |
```

`README.md:130` を次の内容に置き換える。

```markdown
- OAuth 接続済みの場合は保存済み OAuth token を優先して候補を取得し、OAuth token がない場合や取得失敗時は `GITHUB_TOKEN`、最後にローカルのシード候補へフォールバックします。
```

- [ ] **Step 7: AGENTS.md の discovery 説明を更新する**

`AGENTS.md:21` を次の内容に置き換える。

```markdown
- 候補が空のときは、OAuth 接続済みなら保存済み `auth_state.access_token` を優先して GitHub Search API から最近更新されたリポジトリを補充する。OAuth token がない場合や取得失敗時は `GITHUB_TOKEN`、最後に開発用シード候補へフォールバックする。
```

`AGENTS.md:29` を次の内容に置き換える。

```markdown
- 空のリール候補は `DiscoveryService::ensure_candidates` が保存済み OAuth token、`GITHUB_TOKEN` 由来 client、固定の開発用候補の順に補充を試す。候補追加や重複排除の挙動変更は `server/src/discovery.rs`、`server/src/github.rs`、repository store を確認する。
```

- [ ] **Step 8: Makefile は変更不要であることを確認する**

Run: `git diff -- Makefile`

Expected: 出力なし。今回の変更では `make dev`, `make test`, `make build` の実行内容を変えない。

- [ ] **Step 9: Web テストの成功を確認する**

Run: `npm --workspace web run test -- --run web/src/App.test.tsx`

Expected: `web/src/App.test.tsx` が PASS する。

- [ ] **Step 10: コミットする**

```bash
git add web/src/screens/ReelScreen.tsx web/src/App.test.tsx README.md AGENTS.md
git commit -m "docs: describe oauth discovery priority"
```

### Task 5: 全体検証

**Files:**
- Verify: `server/src/repositories.rs`
- Verify: `server/src/discovery.rs`
- Verify: `server/src/github.rs`
- Verify: `server/tests/api_flow.rs`
- Verify: `server/tests/github_fixtures.rs`
- Verify: `web/src/screens/ReelScreen.tsx`
- Verify: `web/src/App.test.tsx`
- Verify: `README.md`
- Verify: `AGENTS.md`
- Verify no change: `Makefile`

- [ ] **Step 1: サーバーテスト全体を実行する**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: すべての server test が PASS する。live GitHub API には依存しない。

- [ ] **Step 2: Web テスト全体を実行する**

Run: `npm run test:web`

Expected: すべての Vitest が PASS する。

- [ ] **Step 3: Web build を実行する**

Run: `npm --workspace web run build`

Expected: TypeScript build と Vite build が PASS する。

- [ ] **Step 4: 通常テストをまとめて実行する**

Run: `npm test`

Expected: `npm run test:web` と `npm run test:server` が PASS する。E2E はこのコマンドには含まれない。

- [ ] **Step 5: 差分を確認する**

Run: `git diff --stat && git diff -- server/src/repositories.rs server/src/discovery.rs server/src/github.rs server/tests/api_flow.rs server/tests/github_fixtures.rs web/src/screens/ReelScreen.tsx web/src/App.test.tsx README.md AGENTS.md Makefile`

Expected: OAuth token の読み取り、Discovery 優先順位、README preview 補完、文言更新、`AGENTS.md` の運用メモ更新だけが含まれている。`Makefile` の差分はない。

- [ ] **Step 6: 最終コミットする**

未コミット差分がある場合だけ実行する。

```bash
git add server/src/repositories.rs server/src/discovery.rs server/src/github.rs server/tests/api_flow.rs server/tests/github_fixtures.rs web/src/screens/ReelScreen.tsx web/src/App.test.tsx README.md AGENTS.md
git commit -m "test: verify oauth discovery flow"
```

Expected: 既にタスクごとにコミット済みなら、この step では commit しない。

## 自己レビュー

- 仕様の目的は Task 1 から Task 4 で網羅する。OAuth token 優先は Task 2、README preview 補完は Task 3、既存 queue/batch/重複排除の再利用は `enqueue_candidates()` をそのまま使う Task 2、fallback は Task 2、live API 非依存テストは Task 2 と Task 3、フロントエンド API 契約維持は Task 4 の文言更新のみで満たす。
- ドキュメント更新は `README.md` と `AGENTS.md` を含める。`Makefile` は実行コマンドが変わらないため変更不要として Task 4 と Task 5 で確認する。
- 対象外の GitHub 書き込み、認証ユーザー文脈の新候補戦略、手動同期 API、rate limit UI、複数ユーザー対応は含めていない。
- 型名とメソッド名は一貫して `auth_access_token()`, `with_oauth_github_client_factory()`, `try_github_discovery()`, `readme_preview()` を使う。
- プレースホルダー、`TODO`、未定義の後続作業は含めていない。
