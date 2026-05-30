use git_reel_server::github::{
    parse_graphql_readme_preview, parse_oauth_token_response, parse_search_response,
    parse_user_response,
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
fn returns_none_for_nullable_graphql_repository() {
    let fixture = r#"{"data":{"repository":null}}"#;
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
