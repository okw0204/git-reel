use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct NewRepository {
    pub github_id: Option<i64>,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub primary_language: Option<String>,
    pub stars: i64,
    pub forks: i64,
    pub license: Option<String>,
    pub updated_at: String,
    pub topics: Vec<String>,
    pub html_url: String,
    pub readme_preview: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Repository {
    pub id: i64,
    pub github_id: Option<i64>,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub primary_language: Option<String>,
    pub stars: i64,
    pub forks: i64,
    pub license: Option<String>,
    pub updated_at: String,
    pub topics: Vec<String>,
    pub html_url: String,
    pub readme_preview: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoEventKind {
    Viewed,
    Saved,
    Skipped,
    Returned,
    DetailOpened,
}

impl RepoEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RepoEventKind::Viewed => "viewed",
            RepoEventKind::Saved => "saved",
            RepoEventKind::Skipped => "skipped",
            RepoEventKind::Returned => "returned",
            RepoEventKind::DetailOpened => "detail_opened",
        }
    }
}

impl TryFrom<String> for RepoEventKind {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "viewed" => Ok(Self::Viewed),
            "saved" => Ok(Self::Saved),
            "skipped" => Ok(Self::Skipped),
            "returned" => Ok(Self::Returned),
            "detail_opened" => Ok(Self::DetailOpened),
            other => Err(format!("unknown event kind: {other}")),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HistoryItem {
    pub repository: Repository,
    pub latest_event: RepoEventKind,
    pub latest_event_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReelResponse {
    pub repository: Option<Repository>,
    pub empty_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SavedRepository {
    pub repository: Repository,
    pub memo: Option<String>,
    pub tags: Vec<String>,
    pub saved_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NoteRequest {
    pub body: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TagsRequest {
    pub tags: Vec<String>,
}
