use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use git_reel_server::{
    config::Config,
    db::connect,
    discovery::{DiscoveryCandidate, DiscoveryService},
    models::{NewRepository, RepoEventKind},
    repositories::RepositoryStore,
};
use serde_json::Value;
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

    store.record_event(repo.id, RepoEventKind::Viewed).await.unwrap();
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
    assert_eq!(app.clone().oneshot(save).await.unwrap().status(), StatusCode::OK);

    let skip = Request::post(format!("/api/reel/{id}/skip"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.oneshot(skip).await.unwrap().status(), StatusCode::OK);
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
        .replace_tags(
            repo.id,
            vec!["rust".to_string(), "local-first".to_string()],
        )
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
