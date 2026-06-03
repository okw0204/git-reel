# Starred repositories 起点 Discovery 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** OAuth 接続済みユーザーの starred repositories の language/topic 傾向から GitHub Search query を生成し、既存の discovery queue に候補を補充する。

**Architecture:** `DiscoveryService::ensure_candidates()` の補充優先順位は、保存済み OAuth token による starred 起点 Discovery、`GITHUB_TOKEN` 由来 recently updated Search、ローカル seed の順にする。`GitHubClient` は `/user/starred` の取得、傾向集計、Search query 生成、Search API 実行、README preview 補完を担当し、`RepositoryStore` とフロントエンド API 契約は変更しない。通常テストは pure function と fake `GitHubDiscoveryClient` で行い、live GitHub API には依存しない。

**Tech Stack:** Rust, axum, reqwest, sqlx SQLite, async-trait, serde, chrono, tokio, React, TypeScript, Vite

---

## ファイル構成

- Create: `server/tests/fixtures/starred_repositories.json`
- 役割: starred repositories の language/topic 集計と query 生成を live GitHub API なしで検証する fixture。
- Modify: `server/src/github.rs`
- 役割: `GitHubDiscoveryClient` に starred 起点メソッドを追加し、`GitHubClient` に `/user/starred` 取得、傾向集計、近傍 topic 展開、Search query 生成、Search API 実行を追加する。recently updated Search は `GITHUB_TOKEN` fallback 用に維持する。
- Modify: `server/src/discovery.rs`
- 役割: OAuth token がある場合は `search_starred_repositories()` を `starred_oauth_search` strategy で呼び、`recently_updated_oauth_search` 経路を削除する。
- Modify: `server/tests/github_fixtures.rs`
- 役割: starred fixture から query を生成できること、近傍 topic が含まれること、Search API fixture から候補を作れることを検証する。
- Modify: `server/tests/api_flow.rs`
- 役割: fake client で OAuth starred 優先、recently updated OAuth 非呼び出し、fallback、discovery batch の strategy/query 保存を検証する。
- Modify: `README.md`
- 役割: OAuth 接続後の候補補充が starred repositories の language/topic 傾向を使うこと、GitHub への書き込みを行わないこと、失敗時に fallback することを説明する。
- Modify: `AGENTS.md`
- 役割: 開発者・エージェント向けの実行時注意と構成メモにある discovery 優先順位を、starred 起点 Discovery に合わせる。
- No change: `web/src/**`
- 理由: フロントエンド API 契約と候補カード表示は変更しないため、UI 実装とフロントエンドテストは触らない。
- No change: `server/migrations/**`
- 理由: 関心プロファイルや候補理由を保存しないため、DB schema 追加は不要。既存の `discovery_batches.strategy` と `discovery_batches.query` に strategy と query を保存する。

## タスク

### Task 1: Starred fixture と query 生成 pure function

**Files:**
- Create: `server/tests/fixtures/starred_repositories.json`
- Modify: `server/src/github.rs:1-330`
- Test: `server/tests/github_fixtures.rs`

- [ ] **Step 1: starred repositories fixture を追加する**

`server/tests/fixtures/starred_repositories.json` を作成する。

```json
[
  {
    "id": 1001,
    "name": "react",
    "full_name": "facebook/react",
    "language": "JavaScript",
    "topics": ["react", "javascript", "frontend"]
  },
  {
    "id": 1002,
    "name": "vite",
    "full_name": "vitejs/vite",
    "language": "TypeScript",
    "topics": ["vite", "frontend", "typescript"]
  },
  {
    "id": 1003,
    "name": "rust",
    "full_name": "rust-lang/rust",
    "language": "Rust",
    "topics": ["rust", "compiler", "systems-programming"]
  },
  {
    "id": 1004,
    "name": "ripgrep",
    "full_name": "BurntSushi/ripgrep",
    "language": "Rust",
    "topics": ["rust", "cli", "search"]
  }
]
```

- [ ] **Step 2: 失敗する fixture テストを追加する**

`server/tests/github_fixtures.rs` の import を変更する。

```rust
use chrono::NaiveDate;
use git_reel_server::github::{
    build_recently_updated_search_query, build_starred_discovery_search_query,
    parse_graphql_readme_preview, parse_oauth_token_response, parse_search_response,
    parse_starred_response, parse_user_response,
};
```

同じファイルの `builds_recently_updated_live_search_query` の後に追加する。

```rust
#[test]
fn builds_starred_discovery_query_from_language_and_topics() {
    let fixture = include_str!("fixtures/starred_repositories.json");
    let starred = parse_starred_response(fixture).unwrap();
    let query = build_starred_discovery_search_query(
        &starred,
        NaiveDate::from_ymd_opt(2026, 5, 28).unwrap(),
    )
    .unwrap();

    assert!(query.starts_with(
        "stars:10..5000 fork:false archived:false pushed:>2026-02-27 ("
    ));
    assert!(query.ends_with(") sort:updated-desc"));
    assert!(query.contains("language:Rust"));
    assert!(query.contains("topic:rust"));
    assert!(query.contains("topic:cli"));
    assert!(query.contains("topic:wasm"));
    assert!(query.contains("topic:frontend"));
}

#[test]
fn returns_none_when_starred_repositories_have_no_interests() {
    let starred = parse_starred_response(
        r#"[
            {"id":1,"name":"empty","full_name":"acme/empty","language":null,"topics":[]}
        ]"#,
    )
    .unwrap();

    let query = build_starred_discovery_search_query(
        &starred,
        NaiveDate::from_ymd_opt(2026, 5, 28).unwrap(),
    );

    assert_eq!(query, None);
}
```

- [ ] **Step 3: 失敗を確認する**

Run: `cargo test --manifest-path server/Cargo.toml starred_discovery_query --test github_fixtures`

Expected: `unresolved imports git_reel_server::github::build_starred_discovery_search_query, git_reel_server::github::parse_starred_response` で失敗する。

- [ ] **Step 4: starred response 型と parser を追加する**

`server/src/github.rs` の `SearchLicense` の後に追加する。

```rust
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct StarredRepositoryInterest {
    pub language: Option<String>,
    pub topics: Vec<String>,
}
```

`server/src/github.rs` の `parse_search_response()` の後に追加する。

```rust
pub fn parse_starred_response(body: &str) -> Result<Vec<StarredRepositoryInterest>, GitHubError> {
    let repositories: Vec<StarredRepositoryInterest> = serde_json::from_str(body)?;
    Ok(repositories)
}
```

- [ ] **Step 5: query 生成 function を追加する**

`server/src/github.rs` の `build_recently_updated_search_query()` の後に追加する。

```rust
pub fn build_starred_discovery_search_query(
    starred: &[StarredRepositoryInterest],
    today: NaiveDate,
) -> Option<String> {
    let mut language_counts = std::collections::HashMap::<String, usize>::new();
    let mut topic_counts = std::collections::HashMap::<String, usize>::new();

    for repository in starred {
        if let Some(language) = repository.language.as_deref().filter(|value| !value.is_empty()) {
            *language_counts.entry(language.to_string()).or_default() += 1;
        }
        for topic in repository.topics.iter().filter(|value| !value.is_empty()) {
            *topic_counts.entry(topic.to_ascii_lowercase()).or_default() += 1;
        }
    }

    let mut languages = ranked_keys(language_counts);
    languages.truncate(2);
    let mut topics = ranked_keys(topic_counts);
    topics.truncate(4);

    let mut qualifiers = Vec::new();
    for language in languages {
        push_unique(&mut qualifiers, format!("language:{language}"));
        for neighbor in topic_neighbors(&language.to_ascii_lowercase()) {
            push_unique(&mut qualifiers, format!("topic:{neighbor}"));
        }
    }
    for topic in topics {
        push_unique(&mut qualifiers, format!("topic:{topic}"));
        for neighbor in topic_neighbors(&topic) {
            push_unique(&mut qualifiers, format!("topic:{neighbor}"));
        }
    }
    qualifiers.truncate(8);

    if qualifiers.is_empty() {
        return None;
    }

    let pushed_after = today - Duration::days(90);
    Some(format!(
        "stars:10..5000 fork:false archived:false pushed:>{} ({}) sort:updated-desc",
        pushed_after.format("%Y-%m-%d"),
        qualifiers.join(" OR ")
    ))
}

fn ranked_keys(counts: std::collections::HashMap<String, usize>) -> Vec<String> {
    let mut entries = counts.into_iter().collect::<Vec<_>>();
    entries.sort_by(|(left_key, left_count), (right_key, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_key.cmp(right_key))
    });
    entries.into_iter().map(|(key, _)| key).collect()
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn topic_neighbors(topic: &str) -> &'static [&'static str] {
    match topic {
        "react" => &["vite", "frontend", "typescript"],
        "rust" => &["cli", "wasm", "systems-programming"],
        "typescript" => &["frontend", "nodejs", "web"],
        "python" => &["machine-learning", "data-science", "automation"],
        "cli" => &["terminal", "developer-tools", "rust"],
        _ => &[],
    }
}
```

- [ ] **Step 6: test を通す**

Run: `cargo test --manifest-path server/Cargo.toml starred_discovery_query --test github_fixtures`

Expected: `test result: ok. 2 passed` を含む出力で成功する。

- [ ] **Step 7: commit する**

```bash
git add server/src/github.rs server/tests/github_fixtures.rs server/tests/fixtures/starred_repositories.json
git commit -m "test: cover starred discovery query generation"
```

### Task 2: GitHubClient に starred 起点 Discovery を追加

**Files:**
- Modify: `server/src/github.rs:37-160`
- Test: `server/tests/github_fixtures.rs`

- [ ] **Step 1: Search API fixture から候補を作る既存テストを明示する**

`server/tests/github_fixtures.rs` には既に次のテストがある。存在しない場合は `extracts_graphql_readme_preview` の前に追加する。

```rust
#[test]
fn converts_search_response_to_new_repository() {
    let fixture = include_str!("fixtures/search_repositories.json");
    let repositories = parse_search_response(fixture).unwrap();
    assert_eq!(repositories.len(), 1);
    assert_eq!(repositories[0].full_name, "okw0204/git-reel");
    assert_eq!(repositories[0].primary_language.as_deref(), Some("Rust"));
    assert_eq!(
        repositories[0].topics,
        vec!["github".to_string(), "discovery".to_string()]
    );
    assert_eq!(repositories[0].readme_preview, None);
}
```

- [ ] **Step 2: trait に starred 起点メソッドを追加する**

`server/src/github.rs` の `GitHubDiscoveryClient` を置き換える。

```rust
#[async_trait]
pub trait GitHubDiscoveryClient: Send + Sync {
    async fn search_recently_updated_repositories(
        &self,
    ) -> Result<(String, Vec<NewRepository>), GitHubError>;

    async fn search_starred_repositories(&self) -> Result<(String, Vec<NewRepository>), GitHubError>;
}
```

- [ ] **Step 3: Search API 実行を private helper に分離する**

`server/src/github.rs` の `impl GitHubClient` 内、`readme_preview()` の後に追加する。

```rust
    async fn search_repositories(
        &self,
        query: String,
    ) -> Result<(String, Vec<NewRepository>), GitHubError> {
        // Search API で候補一覧を取り、README preview は各候補の補助情報として後段で足す。
        let response = self
            .http
            .get("https://api.github.com/search/repositories")
            .header(USER_AGENT, "git-reel")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .query(&recently_updated_search_params(&query))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(GitHubError::HttpStatus(status));
        }

        let body = response.text().await?;
        let mut repositories = parse_search_response(&body)?;
        let readme_requests = repositories
            .iter()
            .map(|repository| {
                let owner = repository.owner.clone();
                let name = repository.name.clone();
                async move { timeout(README_PREVIEW_TIMEOUT, self.readme_preview(&owner, &name)).await }
            })
            .collect::<Vec<_>>();

        for (repository, preview) in repositories.iter_mut().zip(join_all(readme_requests).await) {
            match preview {
                Ok(Ok(preview)) => {
                    repository.readme_preview = preview;
                }
                Ok(Err(error)) => {
                    tracing::warn!(
                        ?error,
                        repository = %repository.full_name,
                        "github readme preview failed; keeping repository without preview"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        repository = %repository.full_name,
                        "github readme preview timed out; keeping repository without preview"
                    );
                }
            }
        }

        Ok((query, repositories))
    }
```

- [ ] **Step 4: starred 取得 helper を追加する**

`server/src/github.rs` の `impl GitHubClient` 内、`search_repositories()` の後に追加する。

```rust
    async fn starred_repositories(&self) -> Result<Vec<StarredRepositoryInterest>, GitHubError> {
        let response = self
            .http
            .get("https://api.github.com/user/starred")
            .header(USER_AGENT, "git-reel")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .query(&[("per_page", "50")])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(GitHubError::HttpStatus(status));
        }

        let body = response.text().await?;
        parse_starred_response(&body)
    }
```

- [ ] **Step 5: recently updated 実装を helper 利用に置き換え、starred 実装を追加する**

`server/src/github.rs` の `impl GitHubDiscoveryClient for GitHubClient` 全体を置き換える。

```rust
#[async_trait]
impl GitHubDiscoveryClient for GitHubClient {
    async fn search_recently_updated_repositories(
        &self,
    ) -> Result<(String, Vec<NewRepository>), GitHubError> {
        self.search_repositories(recently_updated_search_query()).await
    }

    async fn search_starred_repositories(&self) -> Result<(String, Vec<NewRepository>), GitHubError> {
        let starred = self.starred_repositories().await?;
        let Some(query) = build_starred_discovery_search_query(&starred, Utc::now().date_naive()) else {
            return Ok(("starred repositories did not include language or topic interests".to_string(), Vec::new()));
        };

        self.search_repositories(query).await
    }
}
```

- [ ] **Step 6: test を実行する**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: `test result: ok` を含む出力で成功する。

- [ ] **Step 7: commit する**

```bash
git add server/src/github.rs server/tests/github_fixtures.rs
git commit -m "feat: add starred repository discovery client"
```

### Task 3: DiscoveryService の OAuth 経路を starred 起点へ差し替え

**Files:**
- Modify: `server/src/discovery.rs:92-145`
- Test: `server/tests/api_flow.rs`

- [ ] **Step 1: fake client を recently updated と starred で分ける**

`server/tests/api_flow.rs` の `FakeGitHubClient` を置き換える。

```rust
struct FakeGitHubClient {
    recently_updated_result: Result<(String, Vec<NewRepository>), GitHubError>,
    starred_result: Result<(String, Vec<NewRepository>), GitHubError>,
}

impl FakeGitHubClient {
    fn recently_updated(result: Result<(String, Vec<NewRepository>), GitHubError>) -> Self {
        Self {
            recently_updated_result: result,
            starred_result: Ok(("unused starred query".to_string(), Vec::new())),
        }
    }

    fn starred(result: Result<(String, Vec<NewRepository>), GitHubError>) -> Self {
        Self {
            recently_updated_result: Err(GitHubError::HttpStatus(StatusCode::IM_A_TEAPOT)),
            starred_result: result,
        }
    }
}

#[async_trait::async_trait]
impl GitHubDiscoveryClient for FakeGitHubClient {
    async fn search_recently_updated_repositories(
        &self,
    ) -> Result<(String, Vec<NewRepository>), GitHubError> {
        match &self.recently_updated_result {
            Ok((query, repositories)) => Ok((query.clone(), repositories.clone())),
            Err(error) => Err(error.clone()),
        }
    }

    async fn search_starred_repositories(&self) -> Result<(String, Vec<NewRepository>), GitHubError> {
        match &self.starred_result {
            Ok((query, repositories)) => Ok((query.clone(), repositories.clone())),
            Err(error) => Err(error.clone()),
        }
    }
}
```

既存テスト内の `FakeGitHubClient { result: ... }` は、`with_github_client` 用なら `FakeGitHubClient::recently_updated(...)`、OAuth factory 用なら `FakeGitHubClient::starred(...)` に置き換える。

- [ ] **Step 2: fake client 単体テストを更新する**

`fake_github_discovery_client_returns_configured_http_status_error` を置き換える。

```rust
#[tokio::test]
async fn fake_github_discovery_client_returns_configured_http_status_error() {
    let client = FakeGitHubClient::starred(Err(GitHubError::HttpStatus(StatusCode::UNAUTHORIZED)));

    let error = client.search_starred_repositories().await.unwrap_err();

    assert!(matches!(
        error,
        GitHubError::HttpStatus(StatusCode::UNAUTHORIZED)
    ));
}
```

`fake_github_discovery_client_returns_configured_json_error` を置き換える。

```rust
#[tokio::test]
async fn fake_github_discovery_client_returns_configured_json_error() {
    let configured = serde_json::from_str::<Value>("not json").unwrap_err();
    let client = FakeGitHubClient::recently_updated(Err(GitHubError::Json(Arc::new(configured))));

    let error = client
        .search_recently_updated_repositories()
        .await
        .unwrap_err();

    assert!(matches!(error, GitHubError::Json(_)));
}
```

- [ ] **Step 3: OAuth token がある場合に starred が使われる失敗テストを追加する**

`discovery_prefers_oauth_token_client_when_auth_token_exists` を置き換える。

```rust
#[tokio::test]
async fn discovery_prefers_starred_oauth_client_when_auth_token_exists() {
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

    let fallback_client = Arc::new(FakeGitHubClient::recently_updated(Ok((
        "fallback query".to_string(),
        vec![sample_repo("acme/fallback", 402)],
    ))));
    let service = DiscoveryService::new(store.clone())
        .with_github_client(Some(fallback_client))
        .with_oauth_github_client_factory(Arc::new(|token| {
            assert_eq!(token, "gho_oauth_token");
            Arc::new(FakeGitHubClient::starred(Ok((
                "starred oauth query".to_string(),
                vec![sample_repo("acme/oauth-starred", 403)],
            ))))
        }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/oauth-starred");
}
```

- [ ] **Step 4: recently updated OAuth が呼ばれない失敗テストを追加する**

`discovery_prefers_starred_oauth_client_when_auth_token_exists` の後に追加する。

```rust
#[tokio::test]
async fn discovery_does_not_call_recently_updated_for_oauth_token() {
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

    let service = DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(|_| {
        Arc::new(FakeGitHubClient {
            recently_updated_result: Err(GitHubError::HttpStatus(StatusCode::IM_A_TEAPOT)),
            starred_result: Ok((
                "starred oauth query".to_string(),
                vec![sample_repo("acme/oauth-starred-only", 406)],
            )),
        })
    }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/oauth-starred-only");
}
```

- [ ] **Step 5: 失敗を確認する**

Run: `cargo test --manifest-path server/Cargo.toml oauth --test api_flow`

Expected: `discovery_prefers_starred_oauth_client_when_auth_token_exists` が `acme/fallback` または `rust-lang/rust` になって失敗する。理由は `DiscoveryService` がまだ `search_recently_updated_repositories()` を OAuth 経路で呼ぶため。

- [ ] **Step 6: DiscoveryService に starred 用 helper を追加する**

`server/src/discovery.rs` の `try_github_discovery()` の後に追加する。

```rust
    async fn try_starred_github_discovery(
        &self,
        strategy: &str,
        github_client: Arc<dyn GitHubDiscoveryClient>,
    ) -> Result<Option<usize>, ApiError> {
        match github_client.search_starred_repositories().await {
            Ok((query, repositories)) => {
                let candidates = repositories
                    .into_iter()
                    .map(DiscoveryCandidate::from_new_repository)
                    .collect();
                let accepted = self.enqueue_candidates(strategy, &query, candidates).await?;
                Ok(Some(accepted))
            }
            Err(error) => {
                // GitHub 側の一時失敗でリール全体を止めず、次の補充元へフォールバックする。
                tracing::warn!(?error, strategy, "github discovery failed; trying fallback");
                Ok(None)
            }
        }
    }
```

- [ ] **Step 7: OAuth 経路を starred strategy に差し替える**

`server/src/discovery.rs` の OAuth token ブロックを置き換える。

```rust
        if let Some(token) = self.store.auth_access_token().await? {
            // OAuth token はユーザー接続に紐づくため、starred repositories の傾向を優先して使う。
            let github_client = (self.oauth_github_client_factory)(token);
            if let Some(accepted) = self
                .try_starred_github_discovery("starred_oauth_search", github_client)
                .await?
            {
                if accepted > 0 {
                    return Ok(());
                }
            }
        }
```

- [ ] **Step 8: test を通す**

Run: `cargo test --manifest-path server/Cargo.toml oauth --test api_flow`

Expected: `test result: ok` を含む出力で成功する。

- [ ] **Step 9: commit する**

```bash
git add server/src/discovery.rs server/tests/api_flow.rs
git commit -m "feat: prefer starred discovery for oauth users"
```

### Task 4: fallback と discovery batch 保存を検証する

**Files:**
- Modify: `server/tests/api_flow.rs`
- Test: `server/tests/api_flow.rs`

- [ ] **Step 1: starred 失敗時に fallback client を使うテストを更新する**

`discovery_falls_back_to_github_token_client_when_oauth_client_fails` の OAuth factory を次の形にする。

```rust
    let service = DiscoveryService::new(store.clone())
        .with_github_client(Some(fallback_client))
        .with_oauth_github_client_factory(Arc::new(|_| {
            Arc::new(FakeGitHubClient::starred(Err(GitHubError::HttpStatus(
                StatusCode::UNAUTHORIZED,
            ))))
        }));
```

このテスト全体の期待値は維持する。

```rust
    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/fallback");
```

- [ ] **Step 2: starred 採用候補 0 件時に seed へ fallback するテストを追加する**

`discovery_falls_back_to_seed_when_github_returns_no_accepted_candidates` の後に追加する。

```rust
#[tokio::test]
async fn discovery_falls_back_to_seed_when_starred_oauth_returns_no_accepted_candidates() {
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

    let service = DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(|_| {
        Arc::new(FakeGitHubClient::starred(Ok((
            "starred repositories did not include language or topic interests".to_string(),
            Vec::new(),
        ))))
    }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "rust-lang/rust");
}
```

- [ ] **Step 3: discovery batch に starred strategy と query が保存されるテストを追加する**

`discovery_does_not_call_recently_updated_for_oauth_token` の後に追加する。

```rust
#[tokio::test]
async fn discovery_records_starred_oauth_strategy_and_query() {
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

    let service = DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(|_| {
        Arc::new(FakeGitHubClient::starred(Ok((
            "stars:10..5000 fork:false archived:false pushed:>2026-02-27 (language:Rust OR topic:rust OR topic:cli) sort:updated-desc".to_string(),
            vec![sample_repo("acme/starred-batch", 407)],
        ))))
    }));

    service.ensure_candidates().await.unwrap();

    let row: (String, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT strategy, query, candidate_count, accepted_count
        FROM discovery_batches
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "starred_oauth_search");
    assert_eq!(
        row.1,
        "stars:10..5000 fork:false archived:false pushed:>2026-02-27 (language:Rust OR topic:rust OR topic:cli) sort:updated-desc"
    );
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 1);
}
```

- [ ] **Step 4: focused test を実行する**

Run: `cargo test --manifest-path server/Cargo.toml starred_oauth --test api_flow`

Expected: `test result: ok` を含む出力で成功する。

- [ ] **Step 5: api_flow 全体を実行する**

Run: `cargo test --manifest-path server/Cargo.toml --test api_flow`

Expected: `test result: ok` を含む出力で成功する。

- [ ] **Step 6: commit する**

```bash
git add server/tests/api_flow.rs
git commit -m "test: cover starred discovery fallback and batch records"
```

### Task 5: README と AGENTS.md を実挙動に合わせる

**Files:**
- Modify: `README.md:3-16`
- Modify: `README.md:79-82`
- Modify: `README.md:151-157`
- Modify: `AGENTS.md:16-30`

- [ ] **Step 1: README 冒頭の OAuth Discovery 説明を置き換える**

`README.md` 5 行目を次に置き換える。

```markdown
現在の MVP はローカル開発向けです。GitHub への書き込みは行わず、OAuth 接続または開発用接続でリール操作、保存、スキップ、履歴、メモ、タグを確認できます。OAuth 接続済みの場合は保存済み OAuth token で starred repositories を読み取り、language と topic の傾向から GitHub Search API の候補を補充します。未接続または取得失敗時は `GITHUB_TOKEN`、最後にシードされた候補リポジトリへフォールバックします。
```

- [ ] **Step 2: 主な機能の候補補充説明を置き換える**

`README.md` 15 行目を次に置き換える。

```markdown
- OAuth 接続済み token による starred repositories 起点の候補補充、または `GITHUB_TOKEN` 設定時の GitHub Search API fallback
```

- [ ] **Step 3: 開発サーバー説明の OAuth 接続後文言を置き換える**

`README.md` 81 行目を次に置き換える。

```markdown
`GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定して起動すると、開発用接続は無効になり、リール画面の接続ボタンは「GitHubに接続」になります。OAuth 接続後は保存済み OAuth token で starred repositories を読み取り、その language/topic 傾向から候補を取得します。starred 取得や Search が失敗した場合は `GITHUB_TOKEN`、最後にローカル seed へフォールバックします。
```

- [ ] **Step 4: 設計メモの OAuth Discovery 説明を置き換える**

`README.md` 156 行目を次に置き換える。

```markdown
- OAuth 接続済みの場合は保存済み OAuth token で starred repositories の language/topic 傾向を読み取り、OAuth token がない場合や取得失敗時は `GITHUB_TOKEN`、最後にローカルのシード候補へフォールバックします。
```

- [ ] **Step 5: AGENTS.md の実行時注意を置き換える**

`AGENTS.md` 21 行目を次に置き換える。

```markdown
- 候補が空のときは、OAuth 接続済みなら保存済み `auth_state.access_token` で starred repositories を読み取り、language/topic 傾向から GitHub Search API の候補を補充する。OAuth token がない場合や取得失敗時は `GITHUB_TOKEN`、最後に開発用シード候補へフォールバックする。
```

- [ ] **Step 6: AGENTS.md の構成メモを置き換える**

`AGENTS.md` 29 行目を次に置き換える。

```markdown
- 空のリール候補は `DiscoveryService::ensure_candidates` が保存済み OAuth token による starred 起点 Discovery、`GITHUB_TOKEN` 由来 client、固定の開発用候補の順に補充を試す。候補追加や重複排除の挙動変更は `server/src/discovery.rs`、`server/src/github.rs`、repository store を確認する。
```

- [ ] **Step 7: README と AGENTS.md 内に古い recently updated OAuth 説明が残っていないことを確認する**

Run: `rg "保存済み OAuth token を優先して GitHub Search API|最近更新されたリポジトリ|recently updated" README.md AGENTS.md`

Expected: exit code 1。該当する古い説明が残っていない。

- [ ] **Step 8: commit する**

```bash
git add README.md AGENTS.md
git commit -m "docs: describe starred discovery for oauth users"
```

### Task 6: 全体検証と cleanup

**Files:**
- Modify: `server/src/github.rs`
- Modify: `server/src/discovery.rs`
- Modify: `server/tests/github_fixtures.rs`
- Modify: `server/tests/api_flow.rs`
- Modify: `README.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: 削除対象 strategy が残っていないことを確認する**

Run: `rg "recently_updated_oauth_search" server README.md AGENTS.md docs/superpowers/plans/2026-06-03-starred-discovery.md`

Expected: この計画ファイル内の説明以外に一致しない。`server`、`README.md`、`AGENTS.md` に一致が出た場合は、OAuth 経路の古い strategy 名を `starred_oauth_search` に置き換える。

- [ ] **Step 2: サーバーテストを実行する**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: `test result: ok` を含む出力で成功する。

- [ ] **Step 3: Web build を実行して API 契約変更がないことを確認する**

Run: `npm --workspace web run build`

Expected: `built in` を含む出力で成功する。

- [ ] **Step 4: 通常テストを実行する**

Run: `npm test`

Expected: `npm run test:web && npm run test:server` が成功する。

- [ ] **Step 5: 差分を確認する**

Run: `git diff --stat`

Expected: 差分が `server/src/github.rs`, `server/src/discovery.rs`, `server/tests/github_fixtures.rs`, `server/tests/api_flow.rs`, `server/tests/fixtures/starred_repositories.json`, `README.md`, `AGENTS.md` に限定される。`web/src/**` と `server/migrations/**` に差分がない。

- [ ] **Step 6: final commit する**

```bash
git add server/src/github.rs server/src/discovery.rs server/tests/github_fixtures.rs server/tests/api_flow.rs server/tests/fixtures/starred_repositories.json README.md AGENTS.md
git commit -m "feat: discover repositories from oauth starred interests"
```

### Task 7: PR を作成して関連 issue を close する

**Files:**
- No change: repository files
- 操作: `gh` で関連 issue を特定し、PR body に close keyword を入れて PR を作成する。

- [ ] **Step 1: 関連 issue を特定する**

Run:

```bash
ISSUE_NUMBER="$(gh issue list --repo okw0204/life --state open --search "starred discovery OR starred repositories OR Git Reel" --json number,title --jq '.[0].number')"
test -n "$ISSUE_NUMBER"
```

Expected: exit code 0。`ISSUE_NUMBER` に `okw0204/life` の関連 issue 番号が入る。exit code 1 の場合は、誤った issue を close しないため実行を止めてユーザーに issue 番号を確認する。

- [ ] **Step 2: PR 作成前の git 状態を確認する**

Run: `git status --short`

Expected: 出力なし。未コミット差分がある場合は、Task 6 の commit 対象に含めるべき差分か確認してから commit する。

- [ ] **Step 3: 直近 commit を確認する**

Run: `git log --oneline -10`

Expected: starred Discovery 実装、README/AGENTS.md 更新、テスト追加の commit が含まれる。

- [ ] **Step 4: base branch との差分を確認する**

Run: `git diff --stat origin/main...HEAD`

Expected: 差分が `server/src/github.rs`, `server/src/discovery.rs`, `server/tests/github_fixtures.rs`, `server/tests/api_flow.rs`, `server/tests/fixtures/starred_repositories.json`, `README.md`, `AGENTS.md` に限定される。`web/src/**` と `server/migrations/**` に差分がない。

- [ ] **Step 5: PR を作成する**

Run:

```bash
gh pr create \
  --base main \
  --title "OAuth starred repositories から候補を補充する" \
  --body "## Summary
- OAuth 接続済みユーザーの starred repositories から language/topic 傾向を集計
- starred 起点 query で discovery queue を補充
- starred Discovery 失敗時は GITHUB_TOKEN または seed にフォールバック
- README と AGENTS.md を実挙動に合わせて更新

## Tests
- cargo test --manifest-path server/Cargo.toml
- npm --workspace web run build
- npm test

Closes okw0204/life#$ISSUE_NUMBER"
```

Expected: PR URL が出力される。PR body に `Closes okw0204/life#$ISSUE_NUMBER` が含まれる。

- [ ] **Step 6: PR body の close keyword を確認する**

Run: `gh pr view --json url,body --jq '.url, .body'`

Expected: PR URL と、`Closes okw0204/life#` で始まる close keyword が出力される。

## Self-Review

- Spec coverage: OAuth 接続時の starred 起点 Discovery、language/topic 集計、近傍 mapping、`starred_oauth_search` と query の batch 保存、`GITHUB_TOKEN`/seed fallback、live API 非依存テスト、GitHub 書き込みなし、README/AGENTS.md 更新、PR 経由の関連 issue close を各タスクに含めた。
- Placeholder scan: 禁止されている未記入表現は本文に含めていない。各コード変更 step には具体的なコードまたは具体的な置換文を入れた。
- Type consistency: `GitHubDiscoveryClient::search_starred_repositories()`、`StarredRepositoryInterest`、`parse_starred_response()`、`build_starred_discovery_search_query()` の名前を全タスクで統一した。strategy 名は `starred_oauth_search`、fallback strategy は既存の `recently_updated_live_search` のまま統一した。
