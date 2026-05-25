export type AuthState = {
  connected: boolean;
  username: string | null;
};

export type Repository = {
  id: number;
  github_id: number | null;
  owner: string;
  name: string;
  full_name: string;
  description: string | null;
  primary_language: string | null;
  stars: number;
  forks: number;
  license: string | null;
  updated_at: string;
  topics: string[];
  html_url: string;
  readme_preview: string | null;
};

export type ReelResponse = {
  repository: Repository | null;
  empty_reason?: "auth_required" | "queue_empty" | null;
};

export type RepoEventKind = "viewed" | "saved" | "skipped" | "returned" | "detail_opened";

export type HistoryItem = {
  repository: Repository;
  latest_event: RepoEventKind;
  latest_event_at: string;
};

export type SavedRepository = {
  repository: Repository;
  memo: string | null;
  tags: string[];
  saved_at: string;
};

export type RepositoryDetail = {
  repository_id: number;
  memo: string;
  tags: string[];
  readme_preview: string | null;
  detail_error: string | null;
};

export type SettingsSummary = {
  auth_connected: boolean;
  username: string | null;
  discovery_mix: string[];
  database: string;
};
