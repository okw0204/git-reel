use crate::config::Config;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;

pub async fn connect(config: &Config) -> sqlx::Result<SqlitePool> {
    let max_connections = if config.database_url == "sqlite::memory:" {
        1
    } else {
        5
    };
    let options = SqliteConnectOptions::from_str(&config.database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
