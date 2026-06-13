use crate::{
    error::ApiError,
    github::{GitHubClient, GitHubDiscoveryClient},
    models::NewRepository,
    repositories::RepositoryStore,
};
use std::{collections::HashSet, sync::Arc};

pub type GitHubClientFactory = Arc<dyn Fn(String) -> Arc<dyn GitHubDiscoveryClient> + Send + Sync>;

#[derive(Clone)]
pub struct DiscoveryService {
    store: RepositoryStore,
    oauth_github_client_factory: GitHubClientFactory,
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
        Self {
            store,
            oauth_github_client_factory: Arc::new(|token| Arc::new(GitHubClient::new(token))),
        }
    }

    pub fn with_oauth_github_client_factory(mut self, factory: GitHubClientFactory) -> Self {
        self.oauth_github_client_factory = factory;
        self
    }

    pub async fn enqueue_candidates(
        &self,
        strategy: &str,
        query: &str,
        candidates: Vec<DiscoveryCandidate>,
    ) -> Result<usize, ApiError> {
        // DB への upsert 後の id で判定し、GitHub id と owner/name の両方の重複を吸収する。
        let mut accepted_ids = Vec::new();
        let mut accepted_seen = HashSet::new();
        for candidate in candidates.iter() {
            let repo = self
                .store
                .upsert_repository(candidate.repository.clone())
                .await?;
            if accepted_seen.contains(&repo.id) {
                continue;
            }
            // 既に見た・保存した・スキップした候補は通常のリールへ戻さない。
            if !self.store.has_prior_interaction(repo.id).await? {
                accepted_seen.insert(repo.id);
                accepted_ids.push(repo.id);
            }
        }
        // 候補数と採用数を残しておくと、後から発見ロジックの偏りを確認しやすい。
        let batch_id = self
            .store
            .create_discovery_batch(
                strategy,
                query,
                candidates.len() as i64,
                accepted_ids.len() as i64,
            )
            .await?;
        for repository_id in accepted_ids.iter() {
            self.store
                .enqueue_repository(*repository_id, batch_id)
                .await?;
        }
        Ok(accepted_ids.len())
    }

    async fn try_github_discovery(
        &self,
        strategy: &str,
        github_client: Arc<dyn GitHubDiscoveryClient>,
    ) -> Result<Option<usize>, ApiError> {
        match github_client.search_recently_updated_repositories().await {
            Ok((query, repositories)) => {
                let candidates = repositories
                    .into_iter()
                    .map(DiscoveryCandidate::from_new_repository)
                    .collect();
                let accepted = self
                    .enqueue_candidates(strategy, &query, candidates)
                    .await?;
                Ok(Some(accepted))
            }
            Err(error) => {
                // GitHub 側の一時失敗でリール全体を止めず、次の補充元へフォールバックする。
                tracing::warn!(?error, strategy, "github discovery failed; trying fallback");
                Ok(None)
            }
        }
    }

    pub async fn ensure_candidates(&self) -> Result<(), ApiError> {
        // 候補が残っている間は補充せず、空になった時だけ保存済み OAuth token で補充を試す。
        if self.store.next_queued_repository().await?.is_some() {
            return Ok(());
        }

        if let Some(token) = self.store.auth_access_token().await? {
            let github_client = (self.oauth_github_client_factory)(token);
            if let Some(accepted) = self
                .try_github_discovery("recently_updated_oauth_search", github_client)
                .await?
            {
                if accepted > 0 {
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}
