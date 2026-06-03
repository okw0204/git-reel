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

fn sample_repo(full_name: &str, github_id: i64) -> NewRepository {
    let (owner, name) = full_name.split_once('/').unwrap();
    NewRepository {
        github_id: Some(github_id),
        owner: owner.to_string(),
        name: name.to_string(),
        full_name: full_name.to_string(),
        description: Some("A useful local-first tool".to_string()),
        primary_language: Some("Rust".to_string()),
        stars: 42,
        forks: 3,
        license: Some("MIT".to_string()),
        updated_at: "2026-05-20T10:00:00Z".to_string(),
        topics: vec!["cli".to_string(), "sqlite".to_string()],
        html_url: format!("https://github.com/{full_name}"),
        readme_preview: Some("README preview".to_string()),
    }
}

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

    async fn search_starred_repositories(
        &self,
    ) -> Result<(String, Vec<NewRepository>), GitHubError> {
        match &self.starred_result {
            Ok((query, repositories)) => Ok((query.clone(), repositories.clone())),
            Err(error) => Err(error.clone()),
        }
    }
}

#[tokio::test]
async fn fake_github_discovery_client_returns_configured_http_status_error() {
    let client = FakeGitHubClient::starred(Err(GitHubError::HttpStatus(StatusCode::UNAUTHORIZED)));

    let error = client.search_starred_repositories().await.unwrap_err();

    assert!(matches!(
        error,
        GitHubError::HttpStatus(StatusCode::UNAUTHORIZED)
    ));
}

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

#[tokio::test]
async fn upserts_repositories_by_github_id() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);

    let first = store
        .upsert_repository(sample_repo("acme/first", 100))
        .await
        .unwrap();
    let second = store
        .upsert_repository(sample_repo("acme/renamed", 100))
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.full_name, "acme/renamed");
}

#[tokio::test]
async fn records_history_events() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let repo = store
        .upsert_repository(sample_repo("acme/history", 101))
        .await
        .unwrap();

    store
        .record_event(repo.id, RepoEventKind::Viewed)
        .await
        .unwrap();
    store
        .record_event(repo.id, RepoEventKind::Skipped)
        .await
        .unwrap();

    let history = store.history().await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].repository.full_name, "acme/history");
    assert_eq!(history[0].latest_event, RepoEventKind::Skipped);
}

#[tokio::test]
async fn auth_state_starts_disconnected_and_dev_connect_sets_user() {
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
    assert_eq!(response.status(), StatusCode::OK);
}

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
    assert_eq!(
        response.headers().get("location").unwrap(),
        "http://127.0.0.1:5173"
    );

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

#[tokio::test]
async fn discovery_queue_excludes_viewed_and_skipped_repositories() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool.clone());
    let service = DiscoveryService::new(store.clone());

    let viewed = store
        .upsert_repository(sample_repo("acme/viewed", 201))
        .await
        .unwrap();
    store
        .record_event(viewed.id, RepoEventKind::Viewed)
        .await
        .unwrap();

    let accepted = service
        .enqueue_candidates(
            "test-strategy",
            "stars:10..200 pushed:>2026-01-01",
            vec![
                DiscoveryCandidate::from_new_repository(sample_repo("acme/viewed", 201)),
                DiscoveryCandidate::from_new_repository(sample_repo("acme/fresh", 202)),
            ],
        )
        .await
        .unwrap();

    assert_eq!(accepted, 1);
    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/fresh");
}

#[tokio::test]
async fn discovery_queue_deduplicates_candidates_in_one_batch() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let service = DiscoveryService::new(store.clone());

    let accepted = service
        .enqueue_candidates(
            "test-strategy",
            "duplicate candidates",
            vec![
                DiscoveryCandidate::from_new_repository(sample_repo("acme/duplicate", 203)),
                DiscoveryCandidate::from_new_repository(sample_repo("acme/duplicate", 203)),
                DiscoveryCandidate::from_new_repository(sample_repo("acme/fresh", 204)),
            ],
        )
        .await
        .unwrap();

    assert_eq!(accepted, 2);
    let first = store.claim_next_queued_repository().await.unwrap().unwrap();
    let second = store.claim_next_queued_repository().await.unwrap().unwrap();
    let empty = store.claim_next_queued_repository().await.unwrap();

    assert_eq!(first.full_name, "acme/duplicate");
    assert_eq!(second.full_name, "acme/fresh");
    assert!(empty.is_none());
}

#[tokio::test]
async fn discovery_uses_github_candidates_when_queue_is_empty() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let service = DiscoveryService::new(store.clone()).with_github_client(Some(Arc::new(
        FakeGitHubClient::recently_updated(Ok((
            "stars:10..5000 fork:false archived:false pushed:>2026-02-27 sort:updated-desc"
                .to_string(),
            vec![sample_repo("acme/live", 401)],
        ))),
    )));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/live");
}

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

    let service =
        DiscoveryService::new(store.clone()).with_oauth_github_client_factory(Arc::new(|_| {
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

    let fallback_client = Arc::new(FakeGitHubClient::recently_updated(Ok((
        "fallback query".to_string(),
        vec![sample_repo("acme/fallback", 404)],
    ))));
    let service = DiscoveryService::new(store.clone())
        .with_github_client(Some(fallback_client))
        .with_oauth_github_client_factory(Arc::new(|_| {
            Arc::new(FakeGitHubClient::starred(Err(GitHubError::HttpStatus(
                StatusCode::UNAUTHORIZED,
            ))))
        }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/fallback");
}

#[tokio::test]
async fn discovery_uses_github_token_client_when_oauth_token_is_missing() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let fallback_client = Arc::new(FakeGitHubClient::recently_updated(Ok((
        "fallback query".to_string(),
        vec![sample_repo("acme/fallback-only", 405)],
    ))));
    let service = DiscoveryService::new(store.clone())
        .with_github_client(Some(fallback_client))
        .with_oauth_github_client_factory(Arc::new(|_| {
            panic!("OAuth client factory should not be called without a saved token")
        }));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/fallback-only");
}

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
    let service = DiscoveryService::new(store.clone()).with_github_client(Some(Arc::new(
        FakeGitHubClient::recently_updated(Err(GitHubError::HttpStatus(StatusCode::FORBIDDEN))),
    )));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "rust-lang/rust");
}

#[tokio::test]
async fn discovery_falls_back_to_seed_when_github_returns_no_accepted_candidates() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool.clone());
    let service = DiscoveryService::new(store.clone()).with_github_client(Some(Arc::new(
        FakeGitHubClient::recently_updated(Ok((
            "stars:10..5000 fork:false archived:false pushed:>2026-02-27 sort:updated-desc"
                .to_string(),
            Vec::new(),
        ))),
    )));

    service.ensure_candidates().await.unwrap();

    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "rust-lang/rust");
}

#[tokio::test]
async fn claim_next_queued_repository_consumes_each_row_once() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let service = DiscoveryService::new(store.clone());

    service
        .enqueue_candidates(
            "test-strategy",
            "atomic claim",
            vec![
                DiscoveryCandidate::from_new_repository(sample_repo("acme/first-claim", 205)),
                DiscoveryCandidate::from_new_repository(sample_repo("acme/second-claim", 206)),
            ],
        )
        .await
        .unwrap();

    let first = store.claim_next_queued_repository().await.unwrap().unwrap();
    let second = store.claim_next_queued_repository().await.unwrap().unwrap();
    let empty = store.claim_next_queued_repository().await.unwrap();

    assert_eq!(first.full_name, "acme/first-claim");
    assert_eq!(second.full_name, "acme/second-claim");
    assert_ne!(first.id, second.id);
    assert!(empty.is_none());
}

#[tokio::test]
async fn reel_next_save_and_skip_record_events() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let connect = Request::post("/api/auth/dev-connect")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"username":"local-dev"}"#))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(connect).await.unwrap().status(),
        StatusCode::OK
    );

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
}

#[tokio::test]
async fn reel_next_requires_auth_before_consuming_queue() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let response = app
        .clone()
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

    let connect = Request::post("/api/auth/dev-connect")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"username":"local-dev"}"#))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(connect).await.unwrap().status(),
        StatusCode::OK
    );

    let response = app
        .oneshot(Request::post("/api/reel/next").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["repository"]["full_name"], "rust-lang/rust");
}

#[tokio::test]
async fn reel_previous_walks_back_through_view_history() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let connect = Request::post("/api/auth/dev-connect")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"username":"local-dev"}"#))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(connect).await.unwrap().status(),
        StatusCode::OK
    );

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

    let previous_response = app
        .clone()
        .oneshot(
            Request::post("/api/reel/previous")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let previous_body = axum::body::to_bytes(previous_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let previous_payload: Value = serde_json::from_slice(&previous_body).unwrap();
    assert_eq!(
        previous_payload["repository"]["id"],
        second_payload["repository"]["id"]
    );

    let previous_response = app
        .oneshot(
            Request::post("/api/reel/previous")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let previous_body = axum::body::to_bytes(previous_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let previous_payload: Value = serde_json::from_slice(&previous_body).unwrap();
    assert_eq!(
        previous_payload["repository"]["id"],
        first_payload["repository"]["id"]
    );
    assert_ne!(
        third_payload["repository"]["id"],
        previous_payload["repository"]["id"]
    );
}

#[tokio::test]
async fn saved_repositories_support_notes_and_tags() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let repo = store
        .upsert_repository(sample_repo("acme/saved", 301))
        .await
        .unwrap();
    store.save_repository(repo.id).await.unwrap();
    store.set_note(repo.id, "週末に試す").await.unwrap();
    store
        .replace_tags(repo.id, vec!["rust".to_string(), "local-first".to_string()])
        .await
        .unwrap();

    let saved = store.saved("").await.unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].repository.full_name, "acme/saved");
    assert_eq!(saved[0].memo.as_deref(), Some("週末に試す"));
    assert_eq!(
        saved[0].tags,
        vec!["local-first".to_string(), "rust".to_string()]
    );
}
