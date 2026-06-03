#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub github_token: Option<String>,
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
            github_token: optional_env("GITHUB_TOKEN"),
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
            github_token: None,
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

#[cfg(test)]
mod tests {
    use super::Config;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        name: &'static str,
        previous_value: Option<String>,
    }

    impl EnvVarGuard {
        fn set(name: &'static str, value: &str) -> Self {
            let previous_value = std::env::var(name).ok();
            std::env::set_var(name, value);

            Self {
                name,
                previous_value,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous_value {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    #[test]
    fn from_env_treats_empty_github_token_as_unset() {
        let _lock = env_lock().lock().expect("env lock poisoned");
        let _github_token = EnvVarGuard::set("GITHUB_TOKEN", "");

        let config = Config::from_env();

        assert_eq!(config.github_token, None);
    }
}
