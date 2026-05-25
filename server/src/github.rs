use crate::models::NewRepository;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
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
    repository: GraphQlRepository,
}

#[derive(Deserialize)]
struct GraphQlRepository {
    object: Option<ReadmeObject>,
}

#[derive(Deserialize)]
struct ReadmeObject {
    text: String,
}

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

pub fn parse_graphql_readme_preview(body: &str) -> Result<Option<String>, GitHubError> {
    let response: GraphQlResponse = serde_json::from_str(body)?;
    Ok(response.data.repository.object.map(|object| object.text))
}
