use git_reel_server::build_app;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = build_app().await?;
    let addr = SocketAddr::from(([127, 0, 0, 1], 4317));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("git-reel server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
