use crate::models::NewRepository;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, Utc};
use futures_util::future::join_all;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration as StdDuration};

pub(crate) const GITHUB_HTTP_TIMEOUT: StdDuration = StdDuration::from_secs(10);
const README_PREVIEW_MAX_CHARS: usize = 1_000;

#[derive(Clone, Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("github http error: {0}")]
    Http(Arc<reqwest::Error>),
    #[error("github http status: {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("json error: {0}")]
    Json(Arc<serde_json::Error>),
}

impl From<reqwest::Error> for GitHubError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(Arc::new(error))
    }
}

impl From<serde_json::Error> for GitHubError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(Arc::new(error))
    }
}

#[async_trait]
pub trait GitHubDiscoveryClient: Send + Sync {
    async fn search_recently_updated_repositories(
        &self,
    ) -> Result<(String, Vec<NewRepository>), GitHubError>;
}

pub struct GitHubClient {
    token: String,
    http: reqwest::Client,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            http: reqwest::Client::builder()
                .timeout(GITHUB_HTTP_TIMEOUT)
                .build()
                .expect("github HTTP client configuration should be valid"),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    async fn readme_preview(&self, owner: &str, name: &str) -> Result<Option<String>, GitHubError> {
        let response = self
            .http
            .post("https://api.github.com/graphql")
            .header(USER_AGENT, "git-reel")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .json(&GraphQlRequest {
                query: README_PREVIEW_QUERY,
                variables: GraphQlReadmeVariables { owner, name },
            })
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(GitHubError::HttpStatus(status));
        }

        let body = response.text().await?;
        parse_graphql_readme_preview(&body)
    }
}

#[async_trait]
impl GitHubDiscoveryClient for GitHubClient {
    async fn search_recently_updated_repositories(
        &self,
    ) -> Result<(String, Vec<NewRepository>), GitHubError> {
        let query = recently_updated_search_query();
        let response = self
            .http
            .get("https://api.github.com/search/repositories")
            .header(USER_AGENT, "git-reel")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .query(&recently_updated_search_params(&query))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(GitHubError::HttpStatus(status));
        }

        let body = response.text().await?;
        let mut repositories = parse_search_response(&body)?;
        let readme_requests = repositories
            .iter()
            .map(|repository| {
                let owner = repository.owner.clone();
                let name = repository.name.clone();
                async move { self.readme_preview(&owner, &name).await }
            })
            .collect::<Vec<_>>();

        for (repository, preview) in repositories.iter_mut().zip(join_all(readme_requests).await) {
            match preview {
                Ok(preview) => {
                    repository.readme_preview = preview;
                }
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        repository = %repository.full_name,
                        "github readme preview failed; keeping repository without preview"
                    );
                }
            }
        }
        Ok((query, repositories))
    }
}

fn recently_updated_search_query() -> String {
    build_recently_updated_search_query(Utc::now().date_naive())
}

fn recently_updated_search_params(query: &str) -> Vec<(&'static str, String)> {
    vec![
        ("q", query.to_string()),
        ("per_page", "30".to_string()),
        ("sort", "updated".to_string()),
        ("order", "desc".to_string()),
    ]
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    items: Vec<SearchRepository>,
}

#[derive(Deserialize)]
struct SearchRepository {
    id: i64,
    name: String,
    full_name: String,
    owner: SearchOwner,
    html_url: String,
    description: Option<String>,
    stargazers_count: i64,
    forks_count: i64,
    language: Option<String>,
    license: Option<SearchLicense>,
    topics: Vec<String>,
    updated_at: String,
}

#[derive(Deserialize)]
struct SearchOwner {
    login: String,
}

#[derive(Deserialize)]
struct SearchLicense {
    spdx_id: String,
}

#[derive(Deserialize)]
struct GraphQlResponse {
    data: GraphQlData,
}

#[derive(Deserialize)]
struct GraphQlData {
    repository: Option<GraphQlRepository>,
}

#[derive(Deserialize)]
struct GraphQlRepository {
    object: Option<ReadmeObject>,
}

#[derive(Deserialize)]
struct ReadmeObject {
    text: String,
}

#[derive(Serialize)]
struct GraphQlRequest<'a> {
    query: &'a str,
    variables: GraphQlReadmeVariables<'a>,
}

#[derive(Serialize)]
struct GraphQlReadmeVariables<'a> {
    owner: &'a str,
    name: &'a str,
}

const README_PREVIEW_QUERY: &str = r#"
query RepositoryReadme($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    object(expression: "HEAD:README.md") {
      ... on Blob {
        text
      }
    }
  }
}
"#;

pub fn build_recently_updated_search_query(today: NaiveDate) -> String {
    let pushed_after = today - Duration::days(90);
    format!(
        "stars:10..5000 fork:false archived:false pushed:>{} sort:updated-desc",
        pushed_after.format("%Y-%m-%d")
    )
}

// GitHub のレスポンス型を API 境界で NewRepository に寄せ、DB 層を外部 API の形から切り離す。
pub fn parse_search_response(body: &str) -> Result<Vec<NewRepository>, GitHubError> {
    let response: SearchResponse = serde_json::from_str(body)?;
    Ok(response
        .items
        .into_iter()
        .map(|item| NewRepository {
            github_id: Some(item.id),
            owner: item.owner.login,
            name: item.name,
            full_name: item.full_name,
            description: item.description,
            primary_language: item.language,
            stars: item.stargazers_count,
            forks: item.forks_count,
            license: item.license.map(|license| license.spdx_id),
            updated_at: item.updated_at,
            topics: item.topics,
            html_url: item.html_url,
            readme_preview: None,
        })
        .collect())
}

// README は存在しない・取得できないケースが普通にあるため、失敗ではなく None として扱える形にする。
pub fn parse_graphql_readme_preview(body: &str) -> Result<Option<String>, GitHubError> {
    let response: GraphQlResponse = serde_json::from_str(body)?;
    Ok(response
        .data
        .repository
        .and_then(|repository| repository.object)
        .map(|object| object.text.chars().take(README_PREVIEW_MAX_CHARS).collect()))
}

pub fn parse_oauth_token_response(body: &str) -> Result<String, GitHubError> {
    let response: OAuthTokenResponse = serde_json::from_str(body)?;
    Ok(response.access_token)
}

pub fn parse_user_response(body: &str) -> Result<String, GitHubError> {
    let response: UserResponse = serde_json::from_str(body)?;
    Ok(response.login)
}

#[cfg(test)]
mod tests {
    use super::{recently_updated_search_params, GITHUB_HTTP_TIMEOUT};
    use std::time::Duration;

    #[test]
    fn github_http_timeout_is_explicit() {
        assert_eq!(GITHUB_HTTP_TIMEOUT, Duration::from_secs(10));
    }

    #[test]
    fn recently_updated_search_params_request_updated_sort_order() {
        let params = recently_updated_search_params("stars:10..5000");

        assert_eq!(
            params,
            vec![
                ("q", "stars:10..5000".to_string()),
                ("per_page", "30".to_string()),
                ("sort", "updated".to_string()),
                ("order", "desc".to_string()),
            ]
        );
    }
}
