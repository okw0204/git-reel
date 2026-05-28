use crate::models::NewRepository;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use std::time::Duration as StdDuration;

const GITHUB_HTTP_TIMEOUT: StdDuration = StdDuration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("github http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("github http status: {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
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
        let repositories = parse_search_response(&body)?;
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
        .map(|object| object.text))
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
