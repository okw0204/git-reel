#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub public_base_url: String,
    pub public_app_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("GIT_REEL_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:git-reel.db".to_string()),
            github_client_id: optional_env("GITHUB_CLIENT_ID"),
            github_client_secret: optional_env("GITHUB_CLIENT_SECRET"),
            public_base_url: std::env::var("GIT_REEL_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:4317".to_string()),
            public_app_url: std::env::var("GIT_REEL_PUBLIC_APP_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:5173".to_string()),
        }
    }

    pub fn test() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            github_client_id: None,
            github_client_secret: None,
            public_base_url: "http://127.0.0.1:4317".to_string(),
            public_app_url: "http://127.0.0.1:5173".to_string(),
        }
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}
