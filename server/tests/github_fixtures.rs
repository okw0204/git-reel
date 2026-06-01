use chrono::NaiveDate;
use git_reel_server::github::{
    build_recently_updated_search_query, parse_graphql_readme_preview, parse_oauth_token_response,
    parse_search_response, parse_user_response,
};

#[test]
fn converts_search_response_to_new_repository() {
    let fixture = include_str!("fixtures/search_repositories.json");
    let repositories = parse_search_response(fixture).unwrap();
    assert_eq!(repositories.len(), 1);
    assert_eq!(repositories[0].full_name, "okw0204/git-reel");
    assert_eq!(repositories[0].primary_language.as_deref(), Some("Rust"));
    assert_eq!(
        repositories[0].topics,
        vec!["github".to_string(), "discovery".to_string()]
    );
    assert_eq!(repositories[0].readme_preview, None);
}

#[test]
fn extracts_graphql_readme_preview() {
    let fixture = include_str!("fixtures/graphql_repository.json");
    let preview = parse_graphql_readme_preview(fixture).unwrap();
    assert_eq!(
        preview,
        Some("# Git Reel\n\nA local-first discovery app.".to_string())
    );
}

#[test]
fn truncates_graphql_readme_preview() {
    let readme = "a".repeat(1_001);
    let fixture = serde_json::json!({
        "data": {
            "repository": {
                "object": {
                    "text": readme,
                },
            },
        },
    })
    .to_string();

    let preview = parse_graphql_readme_preview(&fixture).unwrap().unwrap();

    assert_eq!(preview.len(), 1_000);
}

#[test]
fn returns_none_for_nullable_graphql_repository() {
    let fixture = r#"{"data":{"repository":null}}"#;
    let preview = parse_graphql_readme_preview(fixture).unwrap();
    assert_eq!(preview, None);
}

#[test]
fn returns_none_for_nullable_graphql_readme_object() {
    let fixture = r#"{"data":{"repository":{"object":null}}}"#;
    let preview = parse_graphql_readme_preview(fixture).unwrap();
    assert_eq!(preview, None);
}

#[test]
fn extracts_oauth_access_token() {
    let token = parse_oauth_token_response(
        r#"{"access_token":"gho_example","token_type":"bearer","scope":"read:user"}"#,
    )
    .unwrap();
    assert_eq!(token, "gho_example");
}

#[test]
fn extracts_github_user_login() {
    let login = parse_user_response(r#"{"login":"okw0204","id":12345}"#).unwrap();
    assert_eq!(login, "okw0204");
}

#[test]
fn builds_recently_updated_live_search_query() {
    let query = build_recently_updated_search_query(NaiveDate::from_ymd_opt(2026, 5, 28).unwrap());

    assert_eq!(
        query,
        "stars:10..5000 fork:false archived:false pushed:>2026-02-27 sort:updated-desc"
    );
}

#[test]
fn creates_github_client_from_token() {
    let client = git_reel_server::github::GitHubClient::new("secret-token".to_string());

    assert_eq!(client.token(), "secret-token");
    assert!(client.readme_preview_enabled());
}

#[test]
fn creates_github_client_without_readme_preview() {
    let client = git_reel_server::github::GitHubClient::new_without_readme_preview(
        "oauth-token".to_string(),
    );

    assert_eq!(client.token(), "oauth-token");
    assert!(!client.readme_preview_enabled());
}
