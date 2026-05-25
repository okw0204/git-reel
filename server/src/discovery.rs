use crate::{error::ApiError, models::NewRepository, repositories::RepositoryStore};

#[derive(Clone)]
pub struct DiscoveryService {
    store: RepositoryStore,
}

#[derive(Clone)]
pub struct DiscoveryCandidate {
    repository: NewRepository,
}

impl DiscoveryCandidate {
    pub fn from_new_repository(repository: NewRepository) -> Self {
        Self { repository }
    }
}

impl DiscoveryService {
    pub fn new(store: RepositoryStore) -> Self {
        Self { store }
    }

    pub async fn enqueue_candidates(
        &self,
        strategy: &str,
        query: &str,
        candidates: Vec<DiscoveryCandidate>,
    ) -> Result<usize, ApiError> {
        let mut accepted_ids = Vec::new();
        for candidate in candidates.iter() {
            let repo = self.store.upsert_repository(candidate.repository.clone()).await?;
            if !self.store.has_prior_interaction(repo.id).await? {
                accepted_ids.push(repo.id);
            }
        }
        let batch_id = self
            .store
            .create_discovery_batch(strategy, query, candidates.len() as i64, accepted_ids.len() as i64)
            .await?;
        for repository_id in accepted_ids.iter() {
            self.store.enqueue_repository(*repository_id, batch_id).await?;
        }
        Ok(accepted_ids.len())
    }

    pub async fn seed_if_empty(&self) -> Result<(), ApiError> {
        if self.store.next_queued_repository().await?.is_some() {
            return Ok(());
        }
        self.enqueue_candidates(
            "seed",
            "local fixture seed",
            vec![
                seed_repo("rust-lang/rust", 1, "Rust", 98000),
                seed_repo("tauri-apps/tauri", 2, "Rust", 88000),
                seed_repo("sqlite/sqlite", 3, "C", 7000),
            ],
        )
        .await?;
        Ok(())
    }
}

fn seed_repo(full_name: &str, github_id: i64, language: &str, stars: i64) -> DiscoveryCandidate {
    let (owner, name) = full_name.split_once('/').unwrap();
    DiscoveryCandidate::from_new_repository(NewRepository {
        github_id: Some(github_id),
        owner: owner.to_string(),
        name: name.to_string(),
        full_name: full_name.to_string(),
        description: Some("ローカル開発用の候補リポジトリです".to_string()),
        primary_language: Some(language.to_string()),
        stars,
        forks: stars / 12,
        license: Some("MIT".to_string()),
        updated_at: "2026-05-25T00:00:00Z".to_string(),
        topics: vec!["developer-tools".to_string(), "open-source".to_string()],
        html_url: format!("https://github.com/{full_name}"),
        readme_preview: Some("This repository is included in the development discovery seed.".to_string()),
    })
}
