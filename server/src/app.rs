use crate::{config::Config, db::connect, repositories::RepositoryStore, routes};
use axum::{routing::get, Router};
use sqlx::SqlitePool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub repositories: RepositoryStore,
}

pub async fn build_app() -> anyhow::Result<Router> {
    build_app_with_config(Config::from_env()).await
}

pub async fn build_test_app() -> anyhow::Result<Router> {
    build_app_with_config(Config::test()).await
}

async fn build_app_with_config(config: Config) -> anyhow::Result<Router> {
    let pool = connect(&config).await?;
    let state = AppState {
        repositories: RepositoryStore::new(pool.clone()),
        pool,
    };

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
