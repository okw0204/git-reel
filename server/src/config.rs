#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub github_token: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("GIT_REEL_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:git-reel.db".to_string()),
            github_token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn test() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            github_token: None,
        }
    }
}
