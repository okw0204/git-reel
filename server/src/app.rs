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

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub repositories: RepositoryStore,
    pub config: Config,
    pub github_client: Option<Arc<dyn GitHubDiscoveryClient>>,
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
    let github_client = config
        .github_token
        .clone()
        .map(|token| Arc::new(GitHubClient::new(token)) as Arc<dyn GitHubDiscoveryClient>);
    let state = AppState {
        repositories: RepositoryStore::new(pool.clone()),
        github_client,
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
