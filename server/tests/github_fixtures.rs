use git_reel_server::github::{parse_graphql_readme_preview, parse_search_response};

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
