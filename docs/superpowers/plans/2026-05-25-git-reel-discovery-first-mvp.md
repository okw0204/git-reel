# Git Reel Discovery-First MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local-first GitHub repository discovery MVP with a Rust API, SQLite persistence, and a Japanese React/Vite UI for reel browsing, saved repositories, history, and settings.

**Architecture:** Create a single repository with `server/` for the Rust local API and `web/` for the React frontend. The Rust API owns SQLite, OAuth state, GitHub API access, discovery queue generation, and repository actions; the React app owns navigation, keyboard interaction, and rendering. The first implementation uses mocked development auth and fixture-backed GitHub tests by default, while keeping live GitHub integration behind environment variables.

**Tech Stack:** Rust 2021, Axum, Tokio, SQLx with SQLite, Reqwest, Serde, React 19, Vite, TypeScript, Vitest, Testing Library, Playwright, npm workspaces.

---

## Scope Check

The spec spans backend persistence, backend GitHub integration, frontend screens, and end-to-end flow. Keep this as one MVP plan because each task produces an incrementally working local app, and the backend/frontend contract is small enough to stabilize in one vertical slice. Defer packaged desktop/Tauri, production token storage, GitHub write actions, and recommendation quality optimization.

## File Structure

- Create `package.json`: root npm workspace scripts for frontend and E2E.
- Create `.gitignore`: ignores Rust, Node, SQLite, and test artifacts.
- Create `README.md`: local setup and run commands for the new app.
- Create `server/Cargo.toml`: Rust crate dependencies and test features.
- Create `server/migrations/0001_initial.sql`: SQLite schema for repositories, events, saved state, notes, tags, batches, queue, and auth.
- Create `server/src/main.rs`: server entrypoint.
- Create `server/src/lib.rs`: module wiring and app factory export for tests.
- Create `server/src/app.rs`: Axum router, shared state, CORS.
- Create `server/src/config.rs`: environment-driven config.
- Create `server/src/error.rs`: API error type and HTTP mapping.
- Create `server/src/models.rs`: request/response/domain structs.
- Create `server/src/db.rs`: SQLite pool creation and migration runner.
- Create `server/src/repositories.rs`: repository upsert, dedupe, notes, tags, saved, history, and queue persistence.
- Create `server/src/discovery.rs`: discovery strategy definitions and candidate filtering.
- Create `server/src/github.rs`: GitHub Search/GraphQL client and fixture-tested conversions.
- Create `server/src/routes/auth.rs`: development auth and auth state endpoints.
- Create `server/src/routes/reel.rs`: current/next/previous/save/skip/detail endpoints.
- Create `server/src/routes/saved.rs`: saved list, note, and tag endpoints.
- Create `server/src/routes/history.rs`: history endpoint.
- Create `server/src/routes/settings.rs`: settings summary endpoint.
- Create `server/tests/api_flow.rs`: backend integration tests against in-memory SQLite.
- Create `server/tests/github_fixtures.rs`: GitHub response conversion tests.
- Create `server/tests/fixtures/search_repositories.json`: Search API fixture.
- Create `server/tests/fixtures/graphql_repository.json`: GraphQL fixture.
- Create `web/package.json`: frontend dependencies and scripts.
- Create `web/index.html`: Vite entry HTML.
- Create `web/tsconfig.json`: TypeScript config.
- Create `web/vite.config.ts`: Vite and Vitest config.
- Create `web/src/main.tsx`: React entrypoint.
- Create `web/src/App.tsx`: top-level shell and route selection.
- Create `web/src/api/client.ts`: typed API wrapper.
- Create `web/src/types.ts`: frontend contract types.
- Create `web/src/hooks/useKeyboardShortcuts.ts`: reel keyboard shortcut wiring.
- Create `web/src/components/AppShell.tsx`: navigation shell.
- Create `web/src/components/RepoCard.tsx`: repository card.
- Create `web/src/components/DetailDrawer.tsx`: README preview, memo, and tags.
- Create `web/src/screens/ReelScreen.tsx`: primary reel workflow.
- Create `web/src/screens/SavedScreen.tsx`: saved repositories and filtering.
- Create `web/src/screens/HistoryScreen.tsx`: viewed/saved/skipped history.
- Create `web/src/screens/SettingsScreen.tsx`: OAuth state and local controls summary.
- Create `web/src/styles.css`: responsive app styling.
- Create `web/src/test/setup.ts`: Testing Library setup.
- Create `web/src/**/*.test.tsx`: focused frontend tests.
- Create `e2e/git-reel.spec.ts`: mocked local E2E flow.
- Create `e2e/playwright.config.ts`: Playwright config.

## API Contract

Use these paths for the MVP:

- `GET /api/auth/state` returns `{ "connected": boolean, "username": string | null }`.
- `POST /api/auth/dev-connect` stores development auth `{ "username": "local-dev" }`.
- `GET /api/reel/current` returns the current repository or `{ "repository": null, "emptyReason": "auth_required" | "queue_empty" }`.
- `POST /api/reel/next` advances and records a viewed event.
- `POST /api/reel/previous` returns the previous viewed repository and records a returned event.
- `POST /api/reel/:id/save` saves a repository and records a saved event.
- `POST /api/reel/:id/skip` records a skipped event and advances.
- `GET /api/reel/:id/detail` returns README preview, tags, memo, and extra metadata.
- `GET /api/saved?query=` returns saved repositories.
- `PATCH /api/saved/:id/note` updates memo text.
- `PUT /api/saved/:id/tags` replaces user tags.
- `GET /api/history` returns local history ordered by newest event.
- `GET /api/settings` returns auth and discovery summary.

## Keyboard Shortcuts

- `j` or `ArrowRight`: next repository.
- `k` or `ArrowLeft`: previous repository.
- `s`: save current repository.
- `x`: skip current repository.
- `d`: toggle detail drawer.

---

### Task 1: Workspace Scaffold

**Files:**
- Create: `package.json`
- Create: `.gitignore`
- Modify: `README.md`
- Create: `server/Cargo.toml`
- Create: `server/src/lib.rs`
- Create: `server/src/main.rs`
- Create: `web/package.json`
- Create: `web/index.html`
- Create: `web/tsconfig.json`
- Create: `web/vite.config.ts`
- Create: `web/src/main.tsx`
- Create: `web/src/App.tsx`
- Create: `web/src/styles.css`

- [ ] **Step 1: Create root workspace files**

Write `package.json`:

```json
{
  "name": "git-reel",
  "private": true,
  "workspaces": ["web"],
  "scripts": {
    "dev:web": "npm --workspace web run dev",
    "test:web": "npm --workspace web run test -- --run",
    "test:e2e": "playwright test --config e2e/playwright.config.ts",
    "test:server": "cargo test --manifest-path server/Cargo.toml",
    "test": "npm run test:web && npm run test:server"
  },
  "devDependencies": {
    "@playwright/test": "^1.52.0"
  }
}
```

Write `.gitignore`:

```gitignore
/node_modules/
/web/node_modules/
/web/dist/
/server/target/
*.db
*.db-shm
*.db-wal
/.env
/playwright-report/
/test-results/
```

- [ ] **Step 2: Create the Rust crate**

Write `server/Cargo.toml`:

```toml
[package]
name = "git-reel-server"
version = "0.1.0"
edition = "2021"

[lib]
name = "git_reel_server"
path = "src/lib.rs"

[[bin]]
name = "git-reel-server"
path = "src/main.rs"

[dependencies]
anyhow = "1"
async-trait = "0.1"
axum = { version = "0.7", features = ["macros"] }
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid"] }
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["serde", "v4"] }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
```

Write `server/src/lib.rs`:

```rust
pub mod app;

pub use app::build_app;
```

Write `server/src/main.rs`:

```rust
use git_reel_server::build_app;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = build_app().await?;
    let addr = SocketAddr::from(([127, 0, 0, 1], 4317));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("git-reel server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 3: Create the frontend scaffold**

Write `web/package.json`:

```json
{
  "name": "git-reel-web",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite --host 127.0.0.1 --port 5173",
    "build": "tsc -b && vite build",
    "test": "vitest"
  },
  "dependencies": {
    "@vitejs/plugin-react": "^4.4.1",
    "lucide-react": "^0.511.0",
    "react": "^19.1.0",
    "react-dom": "^19.1.0",
    "vite": "^6.3.5"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.6.3",
    "@testing-library/react": "^16.3.0",
    "@testing-library/user-event": "^14.6.1",
    "jsdom": "^26.1.0",
    "typescript": "^5.8.3",
    "vitest": "^3.1.4"
  }
}
```

Write `web/index.html`:

```html
<!doctype html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Git Reel</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Write `web/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src", "vite.config.ts"]
}
```

Write `web/vite.config.ts`:

```ts
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": "http://127.0.0.1:4317"
    }
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
    globals: true
  }
});
```

Write `web/src/main.tsx`:

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App />
  </StrictMode>
);
```

Write `web/src/App.tsx`:

```tsx
export default function App() {
  return (
    <main className="app">
      <h1>Git Reel</h1>
      <p>GitHubに接続するとリールを開始できます</p>
    </main>
  );
}
```

Write `web/src/styles.css`:

```css
:root {
  color: #172026;
  background: #f6f7f2;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif;
}

body {
  margin: 0;
}

.app {
  min-height: 100vh;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 12px;
}
```

- [ ] **Step 4: Update README**

Replace `README.md` with:

```markdown
# git-reel

Git Reel is a local-first GitHub repository discovery app.

## Development

Install frontend dependencies:

```bash
npm install
```

Run the local API:

```bash
cargo run --manifest-path server/Cargo.toml
```

Run the web app:

```bash
npm run dev:web
```

Run tests:

```bash
npm test
```
```

- [ ] **Step 5: Run scaffold checks**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: PASS with zero or one crate-level smoke test, and no compile errors.

Run: `npm install`

Expected: dependencies install successfully and `package-lock.json` is created.

Run: `npm run test:web`

Expected: Vitest starts and reports no test files found or passes after the setup file is added in Task 7.

- [ ] **Step 6: Commit**

```bash
git add .gitignore README.md package.json package-lock.json server web
git commit -m "chore: scaffold git reel workspace"
```

---

### Task 2: SQLite Schema And Repository Persistence

**Files:**
- Create: `server/migrations/0001_initial.sql`
- Modify: `server/src/lib.rs`
- Create: `server/src/config.rs`
- Create: `server/src/db.rs`
- Create: `server/src/error.rs`
- Create: `server/src/models.rs`
- Create: `server/src/repositories.rs`
- Create: `server/tests/api_flow.rs`

- [ ] **Step 1: Write failing persistence tests**

Create `server/tests/api_flow.rs`:

```rust
use git_reel_server::{
    config::Config,
    db::connect,
    models::{NewRepository, RepoEventKind},
    repositories::RepositoryStore,
};

fn sample_repo(full_name: &str, github_id: i64) -> NewRepository {
    let (owner, name) = full_name.split_once('/').unwrap();
    NewRepository {
        github_id: Some(github_id),
        owner: owner.to_string(),
        name: name.to_string(),
        full_name: full_name.to_string(),
        description: Some("A useful local-first tool".to_string()),
        primary_language: Some("Rust".to_string()),
        stars: 42,
        forks: 3,
        license: Some("MIT".to_string()),
        updated_at: "2026-05-20T10:00:00Z".to_string(),
        topics: vec!["cli".to_string(), "sqlite".to_string()],
        html_url: format!("https://github.com/{full_name}"),
        readme_preview: Some("README preview".to_string()),
    }
}

#[tokio::test]
async fn upserts_repositories_by_github_id() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);

    let first = store.upsert_repository(sample_repo("acme/first", 100)).await.unwrap();
    let second = store.upsert_repository(sample_repo("acme/renamed", 100)).await.unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.full_name, "acme/renamed");
}

#[tokio::test]
async fn records_history_events() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let repo = store.upsert_repository(sample_repo("acme/history", 101)).await.unwrap();

    store.record_event(repo.id, RepoEventKind::Viewed).await.unwrap();
    store.record_event(repo.id, RepoEventKind::Skipped).await.unwrap();

    let history = store.history().await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].repository.full_name, "acme/history");
    assert_eq!(history[0].latest_event, RepoEventKind::Skipped);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path server/Cargo.toml upserts_repositories_by_github_id records_history_events`

Expected: FAIL because `config`, `db`, `models`, and `repositories` modules do not exist.

- [ ] **Step 3: Create schema and database modules**

Write `server/migrations/0001_initial.sql`:

```sql
CREATE TABLE repositories (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  github_id INTEGER UNIQUE,
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  full_name TEXT NOT NULL,
  normalized_full_name TEXT NOT NULL UNIQUE,
  description TEXT,
  primary_language TEXT,
  stars INTEGER NOT NULL DEFAULT 0,
  forks INTEGER NOT NULL DEFAULT 0,
  license TEXT,
  updated_at TEXT NOT NULL,
  topics_json TEXT NOT NULL DEFAULT '[]',
  html_url TEXT NOT NULL,
  readme_preview TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TEXT
);

CREATE TABLE repo_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('viewed', 'saved', 'skipped', 'returned', 'detail_opened')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE saved_repositories (
  repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
  saved_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE repo_notes (
  repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
  body TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE repo_tags (
  repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
  tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (repository_id, tag_id)
);

CREATE TABLE discovery_batches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  strategy TEXT NOT NULL,
  query TEXT NOT NULL,
  source_api TEXT NOT NULL,
  candidate_count INTEGER NOT NULL,
  accepted_count INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE discovery_queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
  batch_id INTEGER REFERENCES discovery_batches(id) ON DELETE SET NULL,
  position INTEGER NOT NULL,
  consumed_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE auth_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  connected INTEGER NOT NULL DEFAULT 0,
  username TEXT,
  access_token TEXT,
  expires_at TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_repo_events_repository_created ON repo_events(repository_id, created_at);
CREATE INDEX idx_discovery_queue_consumed_position ON discovery_queue(consumed_at, position);
```

Write `server/src/config.rs`:

```rust
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub github_token: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("GIT_REEL_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:git-reel.db".to_string()),
            github_token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn test() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            github_token: None,
        }
    }
}
```

Write `server/src/db.rs`:

```rust
use crate::config::Config;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub async fn connect(config: &Config) -> sqlx::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
```

Write `server/src/error.rs`:

```rust
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("not found")]
    NotFound,
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Database(_) | ApiError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(ErrorBody {
            message: self.to_string(),
        });
        (status, body).into_response()
    }
}
```

- [ ] **Step 4: Create models and persistence implementation**

Write `server/src/models.rs`:

```rust
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

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
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
```

Write `server/src/repositories.rs` with upsert, event, and history methods:

```rust
use crate::{
    error::ApiError,
    models::{HistoryItem, NewRepository, RepoEventKind, Repository},
};
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct RepositoryStore {
    pool: SqlitePool,
}

impl RepositoryStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_repository(&self, repo: NewRepository) -> Result<Repository, ApiError> {
        let normalized = normalize_full_name(&repo.full_name);
        let topics_json = serde_json::to_string(&repo.topics)?;

        sqlx::query(
            r#"
            INSERT INTO repositories (
              github_id, owner, name, full_name, normalized_full_name, description,
              primary_language, stars, forks, license, updated_at, topics_json,
              html_url, readme_preview, last_seen_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(github_id) WHERE github_id IS NOT NULL DO UPDATE SET
              owner = excluded.owner,
              name = excluded.name,
              full_name = excluded.full_name,
              normalized_full_name = excluded.normalized_full_name,
              description = excluded.description,
              primary_language = excluded.primary_language,
              stars = excluded.stars,
              forks = excluded.forks,
              license = excluded.license,
              updated_at = excluded.updated_at,
              topics_json = excluded.topics_json,
              html_url = excluded.html_url,
              readme_preview = excluded.readme_preview,
              last_seen_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(repo.github_id)
        .bind(&repo.owner)
        .bind(&repo.name)
        .bind(&repo.full_name)
        .bind(&normalized)
        .bind(&repo.description)
        .bind(&repo.primary_language)
        .bind(repo.stars)
        .bind(repo.forks)
        .bind(&repo.license)
        .bind(&repo.updated_at)
        .bind(&topics_json)
        .bind(&repo.html_url)
        .bind(&repo.readme_preview)
        .execute(&self.pool)
        .await?;

        self.find_by_normalized_name(&normalized).await
    }

    pub async fn record_event(&self, repository_id: i64, kind: RepoEventKind) -> Result<(), ApiError> {
        sqlx::query("INSERT INTO repo_events (repository_id, kind) VALUES (?, ?)")
            .bind(repository_id)
            .bind(kind.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn history(&self) -> Result<Vec<HistoryItem>, ApiError> {
        let rows = sqlx::query(
            r#"
            SELECT r.*, e.kind AS latest_kind, e.created_at AS latest_event_at
            FROM repositories r
            JOIN (
              SELECT repository_id, MAX(id) AS latest_event_id
              FROM repo_events
              GROUP BY repository_id
            ) latest ON latest.repository_id = r.id
            JOIN repo_events e ON e.id = latest.latest_event_id
            ORDER BY e.id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let repository = repository_from_row(&row)?;
                let latest_event = RepoEventKind::try_from(row.get::<String, _>("latest_kind"))
                    .map_err(|_| ApiError::NotFound)?;
                Ok(HistoryItem {
                    repository,
                    latest_event,
                    latest_event_at: row.get("latest_event_at"),
                })
            })
            .collect()
    }

    async fn find_by_normalized_name(&self, normalized: &str) -> Result<Repository, ApiError> {
        let row = sqlx::query("SELECT * FROM repositories WHERE normalized_full_name = ?")
            .bind(normalized)
            .fetch_one(&self.pool)
            .await?;
        repository_from_row(&row)
    }
}

fn normalize_full_name(full_name: &str) -> String {
    full_name.trim().to_ascii_lowercase()
}

fn repository_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Repository, ApiError> {
    let topics_json: String = row.get("topics_json");
    Ok(Repository {
        id: row.get("id"),
        github_id: row.get("github_id"),
        owner: row.get("owner"),
        name: row.get("name"),
        full_name: row.get("full_name"),
        description: row.get("description"),
        primary_language: row.get("primary_language"),
        stars: row.get("stars"),
        forks: row.get("forks"),
        license: row.get("license"),
        updated_at: row.get("updated_at"),
        topics: serde_json::from_str(&topics_json)?,
        html_url: row.get("html_url"),
        readme_preview: row.get("readme_preview"),
    })
}
```

- [ ] **Step 5: Export modules**

Modify `server/src/lib.rs`:

```rust
pub mod app;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod repositories;

pub use app::build_app;
```

- [ ] **Step 6: Run persistence tests**

Run: `cargo test --manifest-path server/Cargo.toml upserts_repositories_by_github_id records_history_events`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add server
git commit -m "feat: add sqlite repository persistence"
```

---

### Task 3: Local API Router, Auth State, And Settings

**Files:**
- Modify: `server/src/app.rs`
- Modify: `server/src/lib.rs`
- Create: `server/src/routes/mod.rs`
- Create: `server/src/routes/auth.rs`
- Create: `server/src/routes/settings.rs`
- Modify: `server/tests/api_flow.rs`

- [ ] **Step 1: Add failing API tests**

Append to `server/tests/api_flow.rs`:

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn auth_state_starts_disconnected_and_dev_connect_sets_user() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let response = app
        .clone()
        .oneshot(Request::get("/api/auth/state").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let state: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(state["connected"], false);

    let response = app
        .oneshot(
            Request::post("/api/auth/dev-connect")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"local-dev"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path server/Cargo.toml auth_state_starts_disconnected_and_dev_connect_sets_user`

Expected: FAIL because the router and `build_test_app` do not exist.

- [ ] **Step 3: Implement app state and router**

Write `server/src/app.rs`:

```rust
use crate::{config::Config, db::connect, repositories::RepositoryStore, routes};
use axum::{routing::get, Router};
use sqlx::SqlitePool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub repositories: RepositoryStore,
}

pub async fn build_app() -> anyhow::Result<Router> {
    build_app_with_config(Config::from_env()).await
}

pub async fn build_test_app() -> anyhow::Result<Router> {
    build_app_with_config(Config::test()).await
}

async fn build_app_with_config(config: Config) -> anyhow::Result<Router> {
    let pool = connect(&config).await?;
    let state = AppState {
        repositories: RepositoryStore::new(pool.clone()),
        pool,
    };

    Ok(Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .nest("/api/auth", routes::auth::router())
        .nest("/api/settings", routes::settings::router())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http()))
}
```

Write `server/src/routes/mod.rs`:

```rust
pub mod auth;
pub mod settings;
```

- [ ] **Step 4: Implement auth and settings routes**

Write `server/src/routes/auth.rs`:

```rust
use crate::{app::AppState, error::ApiError};
use axum::{extract::State, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/state", get(auth_state))
        .route("/dev-connect", post(dev_connect))
}

#[derive(Serialize)]
struct AuthStateResponse {
    connected: bool,
    username: Option<String>,
}

#[derive(Deserialize)]
struct DevConnectRequest {
    username: String,
}

async fn auth_state(State(state): State<AppState>) -> Result<Json<AuthStateResponse>, ApiError> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT connected, username FROM auth_state WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;
    Ok(Json(AuthStateResponse {
        connected: row.as_ref().map(|r| r.0 == 1).unwrap_or(false),
        username: row.and_then(|r| r.1),
    }))
}

async fn dev_connect(
    State(state): State<AppState>,
    Json(payload): Json<DevConnectRequest>,
) -> Result<Json<AuthStateResponse>, ApiError> {
    sqlx::query(
        r#"
        INSERT INTO auth_state (id, connected, username)
        VALUES (1, 1, ?)
        ON CONFLICT(id) DO UPDATE SET
          connected = 1,
          username = excluded.username,
          updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&payload.username)
    .execute(&state.pool)
    .await?;

    Ok(Json(AuthStateResponse {
        connected: true,
        username: Some(payload.username),
    }))
}
```

Write `server/src/routes/settings.rs`:

```rust
use crate::{app::AppState, error::ApiError};
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(settings))
}

#[derive(Serialize)]
struct SettingsResponse {
    auth_connected: bool,
    username: Option<String>,
    discovery_mix: Vec<&'static str>,
    database: &'static str,
}

async fn settings(State(state): State<AppState>) -> Result<Json<SettingsResponse>, ApiError> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT connected, username FROM auth_state WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;

    Ok(Json(SettingsResponse {
        auth_connected: row.as_ref().map(|r| r.0 == 1).unwrap_or(false),
        username: row.and_then(|r| r.1),
        discovery_mix: vec!["recently_updated", "recently_created", "language_rotation"],
        database: "sqlite",
    }))
}
```

- [ ] **Step 5: Export routes and test app**

Modify `server/src/lib.rs`:

```rust
pub mod app;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod repositories;
pub mod routes;

pub use app::{build_app, build_test_app};
```

- [ ] **Step 6: Run API tests**

Run: `cargo test --manifest-path server/Cargo.toml auth_state_starts_disconnected_and_dev_connect_sets_user`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add server
git commit -m "feat: add local auth and settings api"
```

---

### Task 4: Discovery Queue And Reel Actions

**Files:**
- Modify: `server/src/models.rs`
- Modify: `server/src/repositories.rs`
- Create: `server/src/discovery.rs`
- Create: `server/src/routes/reel.rs`
- Modify: `server/src/routes/mod.rs`
- Modify: `server/src/app.rs`
- Modify: `server/tests/api_flow.rs`

- [ ] **Step 1: Add failing discovery and reel tests**

Append to `server/tests/api_flow.rs`:

```rust
use git_reel_server::discovery::{DiscoveryCandidate, DiscoveryService};

#[tokio::test]
async fn discovery_queue_excludes_viewed_and_skipped_repositories() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool.clone());
    let service = DiscoveryService::new(store.clone());

    let viewed = store.upsert_repository(sample_repo("acme/viewed", 201)).await.unwrap();
    store.record_event(viewed.id, RepoEventKind::Viewed).await.unwrap();

    let accepted = service
        .enqueue_candidates(
            "test-strategy",
            "stars:10..200 pushed:>2026-01-01",
            vec![
                DiscoveryCandidate::from_new_repository(sample_repo("acme/viewed", 201)),
                DiscoveryCandidate::from_new_repository(sample_repo("acme/fresh", 202)),
            ],
        )
        .await
        .unwrap();

    assert_eq!(accepted, 1);
    let next = store.next_queued_repository().await.unwrap().unwrap();
    assert_eq!(next.full_name, "acme/fresh");
}

#[tokio::test]
async fn reel_next_save_and_skip_record_events() {
    let app = git_reel_server::build_test_app().await.unwrap();

    let connect = Request::post("/api/auth/dev-connect")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"username":"local-dev"}"#))
        .unwrap();
    assert_eq!(app.clone().oneshot(connect).await.unwrap().status(), StatusCode::OK);

    let next = Request::post("/api/reel/next").body(Body::empty()).unwrap();
    let response = app.clone().oneshot(next).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let id = payload["repository"]["id"].as_i64().unwrap();

    let save = Request::post(format!("/api/reel/{id}/save"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.clone().oneshot(save).await.unwrap().status(), StatusCode::OK);

    let skip = Request::post(format!("/api/reel/{id}/skip"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.oneshot(skip).await.unwrap().status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path server/Cargo.toml discovery_queue_excludes_viewed_and_skipped_repositories reel_next_save_and_skip_record_events`

Expected: FAIL because discovery service, queue methods, and reel routes do not exist.

- [ ] **Step 3: Add queue models and repository methods**

Append to `server/src/models.rs`:

```rust
#[derive(Clone, Debug, Serialize)]
pub struct ReelResponse {
    pub repository: Option<Repository>,
    pub empty_reason: Option<String>,
}
```

Add these methods to `impl RepositoryStore` in `server/src/repositories.rs`:

```rust
pub async fn has_prior_interaction(&self, repository_id: i64) -> Result<bool, ApiError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM repo_events WHERE repository_id = ?")
        .bind(repository_id)
        .fetch_one(&self.pool)
        .await?;
    Ok(count > 0)
}

pub async fn create_discovery_batch(
    &self,
    strategy: &str,
    query: &str,
    candidate_count: i64,
    accepted_count: i64,
) -> Result<i64, ApiError> {
    let result = sqlx::query(
        "INSERT INTO discovery_batches (strategy, query, source_api, candidate_count, accepted_count) VALUES (?, ?, 'search', ?, ?)",
    )
    .bind(strategy)
    .bind(query)
    .bind(candidate_count)
    .bind(accepted_count)
    .execute(&self.pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn enqueue_repository(&self, repository_id: i64, batch_id: i64) -> Result<(), ApiError> {
    let next_position: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(position), 0) + 1 FROM discovery_queue")
        .fetch_one(&self.pool)
        .await?;
    sqlx::query("INSERT INTO discovery_queue (repository_id, batch_id, position) VALUES (?, ?, ?)")
        .bind(repository_id)
        .bind(batch_id)
        .bind(next_position)
        .execute(&self.pool)
        .await?;
    Ok(())
}

pub async fn next_queued_repository(&self) -> Result<Option<Repository>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT r.*
        FROM discovery_queue q
        JOIN repositories r ON r.id = q.repository_id
        WHERE q.consumed_at IS NULL
        ORDER BY q.position ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(&self.pool)
    .await?;
    match row {
        Some(row) => Ok(Some(repository_from_row(&row)?)),
        None => Ok(None),
    }
}

pub async fn consume_repository(&self, repository_id: i64) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE discovery_queue SET consumed_at = CURRENT_TIMESTAMP WHERE repository_id = ? AND consumed_at IS NULL",
    )
    .bind(repository_id)
    .execute(&self.pool)
    .await?;
    Ok(())
}

pub async fn save_repository(&self, repository_id: i64) -> Result<(), ApiError> {
    sqlx::query("INSERT OR IGNORE INTO saved_repositories (repository_id) VALUES (?)")
        .bind(repository_id)
        .execute(&self.pool)
        .await?;
    self.record_event(repository_id, RepoEventKind::Saved).await
}
```

- [ ] **Step 4: Implement discovery service with seed candidates**

Write `server/src/discovery.rs`:

```rust
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
```

- [ ] **Step 5: Implement reel routes**

Write `server/src/routes/reel.rs`:

```rust
use crate::{
    app::AppState,
    discovery::DiscoveryService,
    error::ApiError,
    models::{ReelResponse, RepoEventKind, Repository},
};
use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/current", get(current))
        .route("/next", post(next))
        .route("/previous", post(previous))
        .route("/:id/save", post(save))
        .route("/:id/skip", post(skip))
        .route("/:id/detail", get(detail))
}

async fn current(State(state): State<AppState>) -> Result<Json<ReelResponse>, ApiError> {
    let connected = auth_connected(&state).await?;
    if !connected {
        return Ok(Json(ReelResponse {
            repository: None,
            empty_reason: Some("auth_required".to_string()),
        }));
    }
    DiscoveryService::new(state.repositories.clone()).seed_if_empty().await?;
    Ok(Json(ReelResponse {
        repository: state.repositories.next_queued_repository().await?,
        empty_reason: None,
    }))
}

async fn next(State(state): State<AppState>) -> Result<Json<ReelResponse>, ApiError> {
    DiscoveryService::new(state.repositories.clone()).seed_if_empty().await?;
    let repository = state.repositories.next_queued_repository().await?;
    if let Some(repo) = repository.as_ref() {
        state.repositories.consume_repository(repo.id).await?;
        state.repositories.record_event(repo.id, RepoEventKind::Viewed).await?;
    }
    Ok(Json(ReelResponse {
        repository,
        empty_reason: None,
    }))
}

async fn previous(State(state): State<AppState>) -> Result<Json<ReelResponse>, ApiError> {
    let history = state.repositories.history().await?;
    let repository = history.first().map(|item| item.repository.clone());
    if let Some(repo) = repository.as_ref() {
        state.repositories.record_event(repo.id, RepoEventKind::Returned).await?;
    }
    Ok(Json(ReelResponse {
        repository,
        empty_reason: None,
    }))
}

async fn save(State(state): State<AppState>, Path(id): Path<i64>) -> Result<Json<ActionResponse>, ApiError> {
    state.repositories.save_repository(id).await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn skip(State(state): State<AppState>, Path(id): Path<i64>) -> Result<Json<ActionResponse>, ApiError> {
    state.repositories.record_event(id, RepoEventKind::Skipped).await?;
    state.repositories.consume_repository(id).await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn detail(State(state): State<AppState>, Path(id): Path<i64>) -> Result<Json<DetailResponse>, ApiError> {
    state.repositories.record_event(id, RepoEventKind::DetailOpened).await?;
    Ok(Json(DetailResponse {
        repository_id: id,
        memo: String::new(),
        tags: Vec::new(),
        readme_preview: None,
        detail_error: None,
    }))
}

async fn auth_connected(state: &AppState) -> Result<bool, ApiError> {
    let connected: Option<i64> = sqlx::query_scalar("SELECT connected FROM auth_state WHERE id = 1")
        .fetch_optional(&state.pool)
        .await?;
    Ok(connected.unwrap_or(0) == 1)
}

#[derive(Serialize)]
struct ActionResponse {
    ok: bool,
}

#[derive(Serialize)]
struct DetailResponse {
    repository_id: i64,
    memo: String,
    tags: Vec<String>,
    readme_preview: Option<String>,
    detail_error: Option<String>,
}
```

Modify `server/src/routes/mod.rs`:

```rust
pub mod auth;
pub mod reel;
pub mod settings;
```

Modify router in `server/src/app.rs` to include:

```rust
.nest("/api/reel", routes::reel::router())
```

Modify `server/src/lib.rs` to include:

```rust
pub mod discovery;
```

- [ ] **Step 6: Run reel tests**

Run: `cargo test --manifest-path server/Cargo.toml discovery_queue_excludes_viewed_and_skipped_repositories reel_next_save_and_skip_record_events`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add server
git commit -m "feat: add discovery queue and reel actions"
```

---

### Task 5: Saved, Notes, Tags, And History APIs

**Files:**
- Modify: `server/src/models.rs`
- Modify: `server/src/repositories.rs`
- Create: `server/src/routes/saved.rs`
- Create: `server/src/routes/history.rs`
- Modify: `server/src/routes/mod.rs`
- Modify: `server/src/app.rs`
- Modify: `server/tests/api_flow.rs`

- [ ] **Step 1: Add failing saved/history tests**

Append to `server/tests/api_flow.rs`:

```rust
#[tokio::test]
async fn saved_repositories_support_notes_and_tags() {
    let pool = connect(&Config::test()).await.unwrap();
    let store = RepositoryStore::new(pool);
    let repo = store.upsert_repository(sample_repo("acme/saved", 301)).await.unwrap();
    store.save_repository(repo.id).await.unwrap();
    store.set_note(repo.id, "週末に試す").await.unwrap();
    store.replace_tags(repo.id, vec!["rust".to_string(), "local-first".to_string()]).await.unwrap();

    let saved = store.saved("").await.unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].repository.full_name, "acme/saved");
    assert_eq!(saved[0].memo.as_deref(), Some("週末に試す"));
    assert_eq!(saved[0].tags, vec!["local-first".to_string(), "rust".to_string()]);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path server/Cargo.toml saved_repositories_support_notes_and_tags`

Expected: FAIL because saved note/tag methods do not exist.

- [ ] **Step 3: Add saved models**

Append to `server/src/models.rs`:

```rust
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
```

- [ ] **Step 4: Implement saved repository methods**

Add to `impl RepositoryStore`:

```rust
pub async fn set_note(&self, repository_id: i64, body: &str) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO repo_notes (repository_id, body)
        VALUES (?, ?)
        ON CONFLICT(repository_id) DO UPDATE SET
          body = excluded.body,
          updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(repository_id)
    .bind(body)
    .execute(&self.pool)
    .await?;
    Ok(())
}

pub async fn replace_tags(&self, repository_id: i64, tags: Vec<String>) -> Result<(), ApiError> {
    let mut tx = self.pool.begin().await?;
    sqlx::query("DELETE FROM repo_tags WHERE repository_id = ?")
        .bind(repository_id)
        .execute(&mut *tx)
        .await?;
    for tag in tags {
        let normalized = tag.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)")
            .bind(&normalized)
            .execute(&mut *tx)
            .await?;
        let tag_id: i64 = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?")
            .bind(&normalized)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO repo_tags (repository_id, tag_id) VALUES (?, ?)")
            .bind(repository_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn saved(&self, query: &str) -> Result<Vec<crate::models::SavedRepository>, ApiError> {
    let like = format!("%{}%", query.to_ascii_lowercase());
    let rows = sqlx::query(
        r#"
        SELECT r.*, s.saved_at, n.body AS memo
        FROM saved_repositories s
        JOIN repositories r ON r.id = s.repository_id
        LEFT JOIN repo_notes n ON n.repository_id = r.id
        WHERE LOWER(r.full_name) LIKE ? OR LOWER(COALESCE(r.description, '')) LIKE ?
        ORDER BY s.saved_at DESC
        "#,
    )
    .bind(&like)
    .bind(&like)
    .fetch_all(&self.pool)
    .await?;

    let mut items = Vec::new();
    for row in rows {
        let repository = repository_from_row(&row)?;
        let tags = self.tags_for(repository.id).await?;
        items.push(crate::models::SavedRepository {
            repository,
            memo: row.get("memo"),
            tags,
            saved_at: row.get("saved_at"),
        });
    }
    Ok(items)
}

async fn tags_for(&self, repository_id: i64) -> Result<Vec<String>, ApiError> {
    let tags = sqlx::query_scalar(
        r#"
        SELECT t.name
        FROM tags t
        JOIN repo_tags rt ON rt.tag_id = t.id
        WHERE rt.repository_id = ?
        ORDER BY t.name ASC
        "#,
    )
    .bind(repository_id)
    .fetch_all(&self.pool)
    .await?;
    Ok(tags)
}
```

- [ ] **Step 5: Implement saved and history routes**

Write `server/src/routes/saved.rs`:

```rust
use crate::{
    app::AppState,
    error::ApiError,
    models::{NoteRequest, SavedRepository, TagsRequest},
};
use axum::{extract::{Path, Query, State}, routing::{get, patch, put}, Json, Router};
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(saved))
        .route("/:id/note", patch(note))
        .route("/:id/tags", put(tags))
}

#[derive(Deserialize)]
struct SavedQuery {
    query: Option<String>,
}

#[derive(Serialize)]
struct ActionResponse {
    ok: bool,
}

async fn saved(
    State(state): State<AppState>,
    Query(query): Query<SavedQuery>,
) -> Result<Json<Vec<SavedRepository>>, ApiError> {
    Ok(Json(state.repositories.saved(query.query.as_deref().unwrap_or("")).await?))
}

async fn note(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<NoteRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    state.repositories.set_note(id, &payload.body).await?;
    Ok(Json(ActionResponse { ok: true }))
}

async fn tags(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<TagsRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    state.repositories.replace_tags(id, payload.tags).await?;
    Ok(Json(ActionResponse { ok: true }))
}
```

Write `server/src/routes/history.rs`:

```rust
use crate::{app::AppState, error::ApiError, models::HistoryItem};
use axum::{extract::State, routing::get, Json, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(history))
}

async fn history(State(state): State<AppState>) -> Result<Json<Vec<HistoryItem>>, ApiError> {
    Ok(Json(state.repositories.history().await?))
}
```

Modify `server/src/routes/mod.rs`:

```rust
pub mod auth;
pub mod history;
pub mod reel;
pub mod saved;
pub mod settings;
```

Modify router in `server/src/app.rs` to include:

```rust
.nest("/api/saved", routes::saved::router())
.nest("/api/history", routes::history::router())
```

- [ ] **Step 6: Run saved/history tests**

Run: `cargo test --manifest-path server/Cargo.toml saved_repositories_support_notes_and_tags`

Expected: PASS.

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add server
git commit -m "feat: add saved notes tags and history api"
```

---

### Task 6: GitHub Client Fixture Conversion

**Files:**
- Create: `server/src/github.rs`
- Modify: `server/src/lib.rs`
- Create: `server/tests/github_fixtures.rs`
- Create: `server/tests/fixtures/search_repositories.json`
- Create: `server/tests/fixtures/graphql_repository.json`

- [ ] **Step 1: Create fixtures and failing tests**

Write `server/tests/fixtures/search_repositories.json`:

```json
{
  "total_count": 1,
  "incomplete_results": false,
  "items": [
    {
      "id": 42,
      "name": "git-reel",
      "full_name": "okw0204/git-reel",
      "owner": { "login": "okw0204" },
      "html_url": "https://github.com/okw0204/git-reel",
      "description": "Repository discovery reel",
      "fork": false,
      "archived": false,
      "stargazers_count": 25,
      "forks_count": 2,
      "language": "Rust",
      "license": { "spdx_id": "MIT" },
      "topics": ["github", "discovery"],
      "updated_at": "2026-05-25T00:00:00Z"
    }
  ]
}
```

Write `server/tests/fixtures/graphql_repository.json`:

```json
{
  "data": {
    "repository": {
      "object": {
        "text": "# Git Reel\n\nA local-first discovery app."
      }
    }
  }
}
```

Write `server/tests/github_fixtures.rs`:

```rust
use git_reel_server::github::{parse_graphql_readme_preview, parse_search_response};

#[test]
fn converts_search_response_to_new_repository() {
    let fixture = include_str!("fixtures/search_repositories.json");
    let repositories = parse_search_response(fixture).unwrap();
    assert_eq!(repositories.len(), 1);
    assert_eq!(repositories[0].full_name, "okw0204/git-reel");
    assert_eq!(repositories[0].primary_language.as_deref(), Some("Rust"));
    assert_eq!(repositories[0].topics, vec!["github".to_string(), "discovery".to_string()]);
}

#[test]
fn extracts_graphql_readme_preview() {
    let fixture = include_str!("fixtures/graphql_repository.json");
    let preview = parse_graphql_readme_preview(fixture).unwrap();
    assert_eq!(preview, Some("# Git Reel\n\nA local-first discovery app.".to_string()));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: FAIL because `github` module does not exist.

- [ ] **Step 3: Implement GitHub fixture conversion**

Write `server/src/github.rs`:

```rust
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
```

Modify `server/src/lib.rs`:

```rust
pub mod github;
```

- [ ] **Step 4: Run GitHub fixture tests**

Run: `cargo test --manifest-path server/Cargo.toml --test github_fixtures`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add server
git commit -m "feat: add github fixture conversion"
```

---

### Task 7: Frontend API Client, Shell, And Reel Screen

**Files:**
- Create: `web/src/test/setup.ts`
- Create: `web/src/types.ts`
- Create: `web/src/api/client.ts`
- Create: `web/src/hooks/useKeyboardShortcuts.ts`
- Create: `web/src/components/AppShell.tsx`
- Create: `web/src/components/RepoCard.tsx`
- Create: `web/src/components/DetailDrawer.tsx`
- Modify: `web/src/App.tsx`
- Create: `web/src/screens/ReelScreen.tsx`
- Create: `web/src/screens/ReelScreen.test.tsx`
- Modify: `web/src/styles.css`

- [ ] **Step 1: Create failing Reel screen test**

Write `web/src/test/setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

Write `web/src/screens/ReelScreen.test.tsx`:

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ReelScreen from "./ReelScreen";

const repository = {
  id: 1,
  github_id: 42,
  owner: "okw0204",
  name: "git-reel",
  full_name: "okw0204/git-reel",
  description: "Repository discovery reel",
  primary_language: "Rust",
  stars: 25,
  forks: 2,
  license: "MIT",
  updated_at: "2026-05-25T00:00:00Z",
  topics: ["github", "discovery"],
  html_url: "https://github.com/okw0204/git-reel",
  readme_preview: "# Git Reel"
};

describe("ReelScreen", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/api/auth/state")) {
        return Response.json({ connected: true, username: "local-dev" });
      }
      if (url.endsWith("/api/reel/current")) {
        return Response.json({ repository, empty_reason: null });
      }
      if (url.endsWith("/api/reel/next")) {
        return Response.json({ repository, empty_reason: null });
      }
      if (url.endsWith("/api/reel/1/save") || url.endsWith("/api/reel/1/skip")) {
        return Response.json({ ok: true });
      }
      if (url.endsWith("/api/reel/1/detail")) {
        return Response.json({ repository_id: 1, memo: "", tags: [], readme_preview: "# Git Reel", detail_error: null });
      }
      return Response.json({}, { status: 404 });
    }));
  });

  it("renders a repository and supports visible actions", async () => {
    render(<ReelScreen />);
    expect(await screen.findByText("okw0204/git-reel")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    await userEvent.click(screen.getByRole("button", { name: "詳細" }));
    expect(await screen.findByText("# Git Reel")).toBeInTheDocument();
  });

  it("shows auth prompt when GitHub is disconnected", async () => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/api/auth/state")) {
        return Response.json({ connected: false, username: null });
      }
      return Response.json({ repository: null, empty_reason: "auth_required" });
    }));
    render(<ReelScreen />);
    await waitFor(() => {
      expect(screen.getByText("GitHubに接続するとリールを開始できます")).toBeInTheDocument();
    });
  });
});
```

- [ ] **Step 2: Run test to verify failure**

Run: `npm run test:web -- ReelScreen`

Expected: FAIL because frontend modules do not exist.

- [ ] **Step 3: Create frontend types and API client**

Write `web/src/types.ts`:

```ts
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

export type AuthState = {
  connected: boolean;
  username: string | null;
};

export type ReelResponse = {
  repository: Repository | null;
  empty_reason: "auth_required" | "queue_empty" | null;
};

export type DetailResponse = {
  repository_id: number;
  memo: string;
  tags: string[];
  readme_preview: string | null;
  detail_error: string | null;
};
```

Write `web/src/api/client.ts`:

```ts
import type { AuthState, DetailResponse, ReelResponse } from "../types";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: { "content-type": "application/json", ...init?.headers },
    ...init
  });
  if (!response.ok) {
    throw new Error(`API request failed: ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export const api = {
  authState: () => request<AuthState>("/api/auth/state"),
  devConnect: () =>
    request<AuthState>("/api/auth/dev-connect", {
      method: "POST",
      body: JSON.stringify({ username: "local-dev" })
    }),
  current: () => request<ReelResponse>("/api/reel/current"),
  next: () => request<ReelResponse>("/api/reel/next", { method: "POST" }),
  previous: () => request<ReelResponse>("/api/reel/previous", { method: "POST" }),
  save: (id: number) => request<{ ok: boolean }>(`/api/reel/${id}/save`, { method: "POST" }),
  skip: (id: number) => request<{ ok: boolean }>(`/api/reel/${id}/skip`, { method: "POST" }),
  detail: (id: number) => request<DetailResponse>(`/api/reel/${id}/detail`)
};
```

Write `web/src/hooks/useKeyboardShortcuts.ts`:

```ts
import { useEffect } from "react";

type ShortcutHandlers = {
  onNext: () => void;
  onPrevious: () => void;
  onSave: () => void;
  onSkip: () => void;
  onDetail: () => void;
};

export function useKeyboardShortcuts(handlers: ShortcutHandlers) {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }
      if (event.key === "j" || event.key === "ArrowRight") handlers.onNext();
      if (event.key === "k" || event.key === "ArrowLeft") handlers.onPrevious();
      if (event.key === "s") handlers.onSave();
      if (event.key === "x") handlers.onSkip();
      if (event.key === "d") handlers.onDetail();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [handlers]);
}
```

- [ ] **Step 4: Create shell and reel components**

Write `web/src/components/AppShell.tsx`:

```tsx
type AppShellProps = {
  active: "reel" | "saved" | "history" | "settings";
  onNavigate: (screen: AppShellProps["active"]) => void;
  children: React.ReactNode;
};

const labels = {
  reel: "リール",
  saved: "保存",
  history: "履歴",
  settings: "設定"
};

export default function AppShell({ active, onNavigate, children }: AppShellProps) {
  return (
    <div className="shell">
      <nav className="nav" aria-label="メイン">
        <strong>Git Reel</strong>
        {Object.entries(labels).map(([key, label]) => (
          <button
            key={key}
            className={active === key ? "nav-active" : ""}
            onClick={() => onNavigate(key as AppShellProps["active"])}
          >
            {label}
          </button>
        ))}
      </nav>
      <main className="main">{children}</main>
    </div>
  );
}
```

Write `web/src/components/RepoCard.tsx`:

```tsx
import { ExternalLink } from "lucide-react";
import type { Repository } from "../types";

type Props = {
  repository: Repository;
  onNext: () => void;
  onPrevious: () => void;
  onSave: () => void;
  onSkip: () => void;
  onDetail: () => void;
};

export default function RepoCard({ repository, onNext, onPrevious, onSave, onSkip, onDetail }: Props) {
  return (
    <section className="repo-card" aria-label={repository.full_name}>
      <div className="repo-card-header">
        <div>
          <p className="owner">{repository.owner}</p>
          <h2>{repository.full_name}</h2>
        </div>
        <a className="icon-link" href={repository.html_url} target="_blank" rel="noreferrer" aria-label="GitHubで開く">
          <ExternalLink size={20} />
        </a>
      </div>
      <p className="description">{repository.description ?? "説明はありません"}</p>
      <dl className="stats">
        <div><dt>言語</dt><dd>{repository.primary_language ?? "不明"}</dd></div>
        <div><dt>Stars</dt><dd>{repository.stars.toLocaleString()}</dd></div>
        <div><dt>Forks</dt><dd>{repository.forks.toLocaleString()}</dd></div>
        <div><dt>License</dt><dd>{repository.license ?? "不明"}</dd></div>
      </dl>
      <div className="topics">
        {repository.topics.map((topic) => <span key={topic}>{topic}</span>)}
      </div>
      <div className="actions">
        <button onClick={onPrevious}>前へ</button>
        <button onClick={onSave}>保存</button>
        <button onClick={onSkip}>スキップ</button>
        <button onClick={onDetail}>詳細</button>
        <button onClick={onNext}>次へ</button>
      </div>
    </section>
  );
}
```

Write `web/src/components/DetailDrawer.tsx`:

```tsx
import type { DetailResponse } from "../types";

export default function DetailDrawer({ detail }: { detail: DetailResponse | null }) {
  if (!detail) return null;
  return (
    <aside className="drawer" aria-label="詳細">
      {detail.detail_error ? <p className="error">{detail.detail_error}</p> : null}
      <h2>README</h2>
      <pre>{detail.readme_preview ?? "READMEを取得できませんでした"}</pre>
      <h2>メモ</h2>
      <p>{detail.memo || "メモはまだありません"}</p>
      <h2>タグ</h2>
      <div className="topics">
        {detail.tags.length === 0 ? <span>タグなし</span> : detail.tags.map((tag) => <span key={tag}>{tag}</span>)}
      </div>
    </aside>
  );
}
```

- [ ] **Step 5: Implement Reel screen and App navigation**

Write `web/src/screens/ReelScreen.tsx`:

```tsx
import { useCallback, useEffect, useState } from "react";
import { api } from "../api/client";
import DetailDrawer from "../components/DetailDrawer";
import RepoCard from "../components/RepoCard";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import type { AuthState, DetailResponse, Repository } from "../types";

export default function ReelScreen() {
  const [auth, setAuth] = useState<AuthState | null>(null);
  const [repository, setRepository] = useState<Repository | null>(null);
  const [detail, setDetail] = useState<DetailResponse | null>(null);
  const [message, setMessage] = useState("読み込み中です");

  const load = useCallback(async () => {
    const authState = await api.authState();
    setAuth(authState);
    if (!authState.connected) {
      setMessage("GitHubに接続するとリールを開始できます");
      return;
    }
    const current = await api.current();
    setRepository(current.repository);
    setMessage(current.repository ? "" : "候補がありません。条件をゆるめて再試行してください");
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const next = useCallback(async () => {
    const response = await api.next();
    setRepository(response.repository);
    setDetail(null);
  }, []);

  const previous = useCallback(async () => {
    const response = await api.previous();
    setRepository(response.repository);
    setDetail(null);
  }, []);

  const save = useCallback(async () => {
    if (repository) await api.save(repository.id);
  }, [repository]);

  const skip = useCallback(async () => {
    if (!repository) return;
    await api.skip(repository.id);
    await next();
  }, [next, repository]);

  const toggleDetail = useCallback(async () => {
    if (!repository) return;
    if (detail) {
      setDetail(null);
      return;
    }
    setDetail(await api.detail(repository.id));
  }, [detail, repository]);

  useKeyboardShortcuts({ onNext: next, onPrevious: previous, onSave: save, onSkip: skip, onDetail: toggleDetail });

  if (auth && !auth.connected) {
    return (
      <section className="empty">
        <p>{message}</p>
        <button onClick={async () => { await api.devConnect(); await load(); }}>開発用に接続</button>
      </section>
    );
  }

  if (!repository) {
    return <section className="empty"><p>{message}</p></section>;
  }

  return (
    <div className="reel-layout">
      <RepoCard
        repository={repository}
        onNext={next}
        onPrevious={previous}
        onSave={save}
        onSkip={skip}
        onDetail={toggleDetail}
      />
      <DetailDrawer detail={detail} />
    </div>
  );
}
```

Replace `web/src/App.tsx`:

```tsx
import { useState } from "react";
import AppShell from "./components/AppShell";
import ReelScreen from "./screens/ReelScreen";

type Screen = "reel" | "saved" | "history" | "settings";

export default function App() {
  const [screen, setScreen] = useState<Screen>("reel");
  return (
    <AppShell active={screen} onNavigate={setScreen}>
      {screen === "reel" ? <ReelScreen /> : <p>この画面は次のタスクで追加します</p>}
    </AppShell>
  );
}
```

- [ ] **Step 6: Replace CSS with app layout**

Replace `web/src/styles.css` with:

```css
:root {
  color: #172026;
  background: #f6f7f2;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

body { margin: 0; }
button, input, textarea { font: inherit; }
button {
  border: 1px solid #b9c0b5;
  background: #ffffff;
  border-radius: 8px;
  padding: 10px 14px;
  cursor: pointer;
}
.shell { min-height: 100vh; display: grid; grid-template-columns: 220px 1fr; }
.nav { background: #16221d; color: #fff; padding: 20px; display: flex; flex-direction: column; gap: 10px; }
.nav button { color: #fff; background: transparent; border-color: #3b4d45; text-align: left; }
.nav .nav-active { background: #2f6f5e; }
.main { padding: 28px; }
.reel-layout { display: grid; grid-template-columns: minmax(320px, 720px) minmax(280px, 420px); gap: 20px; align-items: start; }
.repo-card, .drawer, .empty { background: #fff; border: 1px solid #d7ddd2; border-radius: 8px; padding: 24px; }
.repo-card-header { display: flex; justify-content: space-between; gap: 16px; }
.owner { margin: 0; color: #557064; }
h2 { margin: 4px 0 12px; font-size: 28px; }
.description { font-size: 18px; line-height: 1.6; }
.stats { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; }
.stats div { border-top: 1px solid #d7ddd2; padding-top: 10px; }
.stats dt { color: #557064; font-size: 12px; }
.stats dd { margin: 4px 0 0; font-weight: 700; }
.topics { display: flex; flex-wrap: wrap; gap: 8px; margin: 16px 0; }
.topics span { border-radius: 999px; background: #e8eee8; padding: 6px 10px; font-size: 13px; }
.actions { display: flex; flex-wrap: wrap; gap: 10px; }
.icon-link { color: #172026; display: grid; place-items: center; }
.drawer pre { white-space: pre-wrap; background: #f4f6f1; padding: 12px; border-radius: 6px; overflow: auto; }
.error { color: #a23b30; }
@media (max-width: 840px) {
  .shell { grid-template-columns: 1fr; }
  .nav { flex-direction: row; overflow-x: auto; }
  .reel-layout { grid-template-columns: 1fr; }
  .stats { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
```

- [ ] **Step 7: Run frontend tests**

Run: `npm run test:web -- ReelScreen`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add web
git commit -m "feat: add reel frontend"
```

---

### Task 8: Saved, History, And Settings Screens

**Files:**
- Modify: `web/src/api/client.ts`
- Modify: `web/src/types.ts`
- Create: `web/src/screens/SavedScreen.tsx`
- Create: `web/src/screens/SavedScreen.test.tsx`
- Create: `web/src/screens/HistoryScreen.tsx`
- Create: `web/src/screens/HistoryScreen.test.tsx`
- Create: `web/src/screens/SettingsScreen.tsx`
- Create: `web/src/screens/SettingsScreen.test.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/styles.css`

- [ ] **Step 1: Add failing screen tests**

Write `web/src/screens/SavedScreen.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SavedScreen from "./SavedScreen";

describe("SavedScreen", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn(async () => Response.json([
      { repository: { id: 1, github_id: 42, owner: "okw0204", name: "git-reel", full_name: "okw0204/git-reel", description: "Discovery", primary_language: "Rust", stars: 25, forks: 2, license: "MIT", updated_at: "2026-05-25T00:00:00Z", topics: ["github"], html_url: "https://github.com/okw0204/git-reel", readme_preview: null }, memo: "週末に試す", tags: ["rust"], saved_at: "2026-05-25T00:00:00Z" }
    ])));
  });
  it("renders saved repositories", async () => {
    render(<SavedScreen />);
    expect(await screen.findByText("okw0204/git-reel")).toBeInTheDocument();
    expect(screen.getByText("週末に試す")).toBeInTheDocument();
  });
});
```

Write `web/src/screens/HistoryScreen.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import HistoryScreen from "./HistoryScreen";

describe("HistoryScreen", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn(async () => Response.json([
      { repository: { id: 1, github_id: 42, owner: "okw0204", name: "git-reel", full_name: "okw0204/git-reel", description: "Discovery", primary_language: "Rust", stars: 25, forks: 2, license: "MIT", updated_at: "2026-05-25T00:00:00Z", topics: [], html_url: "https://github.com/okw0204/git-reel", readme_preview: null }, latest_event: "saved", latest_event_at: "2026-05-25T00:00:00Z" }
    ])));
  });
  it("renders history events", async () => {
    render(<HistoryScreen />);
    expect(await screen.findByText("okw0204/git-reel")).toBeInTheDocument();
    expect(screen.getByText("saved")).toBeInTheDocument();
  });
});
```

Write `web/src/screens/SettingsScreen.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SettingsScreen from "./SettingsScreen";

describe("SettingsScreen", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn(async () => Response.json({
      auth_connected: true,
      username: "local-dev",
      discovery_mix: ["recently_updated", "language_rotation"],
      database: "sqlite"
    })));
  });
  it("renders auth and database state", async () => {
    render(<SettingsScreen />);
    expect(await screen.findByText("local-dev")).toBeInTheDocument();
    expect(screen.getByText("sqlite")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests to verify failure**

Run: `npm run test:web -- SavedScreen HistoryScreen SettingsScreen`

Expected: FAIL because screens and types do not exist.

- [ ] **Step 3: Add frontend types and API methods**

Append to `web/src/types.ts`:

```ts
export type SavedRepository = {
  repository: Repository;
  memo: string | null;
  tags: string[];
  saved_at: string;
};

export type HistoryItem = {
  repository: Repository;
  latest_event: "viewed" | "saved" | "skipped" | "returned" | "detail_opened";
  latest_event_at: string;
};

export type SettingsSummary = {
  auth_connected: boolean;
  username: string | null;
  discovery_mix: string[];
  database: string;
};
```

Add to `api` in `web/src/api/client.ts`:

```ts
saved: (query = "") => request<import("../types").SavedRepository[]>(`/api/saved?query=${encodeURIComponent(query)}`),
history: () => request<import("../types").HistoryItem[]>("/api/history"),
settings: () => request<import("../types").SettingsSummary>("/api/settings")
```

- [ ] **Step 4: Create Saved screen**

Write `web/src/screens/SavedScreen.tsx`:

```tsx
import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { SavedRepository } from "../types";

export default function SavedScreen() {
  const [items, setItems] = useState<SavedRepository[]>([]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void api.saved(query).then(setItems);
  }, [query]);

  return (
    <section>
      <div className="screen-header">
        <h1>保存</h1>
        <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="絞り込み" aria-label="保存を絞り込み" />
      </div>
      {items.length === 0 ? <p>まだ保存したリポジトリはありません</p> : null}
      <div className="list">
        {items.map((item) => (
          <article className="list-item" key={item.repository.id}>
            <h2>{item.repository.full_name}</h2>
            <p>{item.repository.description ?? "説明はありません"}</p>
            {item.memo ? <p>{item.memo}</p> : null}
            <div className="topics">{item.tags.map((tag) => <span key={tag}>{tag}</span>)}</div>
          </article>
        ))}
      </div>
    </section>
  );
}
```

- [ ] **Step 5: Create History and Settings screens**

Write `web/src/screens/HistoryScreen.tsx`:

```tsx
import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { HistoryItem } from "../types";

export default function HistoryScreen() {
  const [items, setItems] = useState<HistoryItem[]>([]);
  useEffect(() => {
    void api.history().then(setItems);
  }, []);
  return (
    <section>
      <h1>履歴</h1>
      {items.length === 0 ? <p>履歴はまだありません</p> : null}
      <div className="list">
        {items.map((item) => (
          <article className="list-item" key={`${item.repository.id}-${item.latest_event_at}`}>
            <h2>{item.repository.full_name}</h2>
            <p>{item.latest_event}</p>
            <p>{item.latest_event_at}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
```

Write `web/src/screens/SettingsScreen.tsx`:

```tsx
import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { SettingsSummary } from "../types";

export default function SettingsScreen() {
  const [settings, setSettings] = useState<SettingsSummary | null>(null);
  useEffect(() => {
    void api.settings().then(setSettings);
  }, []);
  if (!settings) return <p>読み込み中です</p>;
  return (
    <section>
      <h1>設定</h1>
      <dl className="settings-grid">
        <div><dt>GitHub</dt><dd>{settings.auth_connected ? settings.username : "未接続"}</dd></div>
        <div><dt>Database</dt><dd>{settings.database}</dd></div>
        <div><dt>Discovery</dt><dd>{settings.discovery_mix.join(", ")}</dd></div>
      </dl>
    </section>
  );
}
```

Replace screen selection in `web/src/App.tsx`:

```tsx
import { useState } from "react";
import AppShell from "./components/AppShell";
import HistoryScreen from "./screens/HistoryScreen";
import ReelScreen from "./screens/ReelScreen";
import SavedScreen from "./screens/SavedScreen";
import SettingsScreen from "./screens/SettingsScreen";

type Screen = "reel" | "saved" | "history" | "settings";

export default function App() {
  const [screen, setScreen] = useState<Screen>("reel");
  return (
    <AppShell active={screen} onNavigate={setScreen}>
      {screen === "reel" ? <ReelScreen /> : null}
      {screen === "saved" ? <SavedScreen /> : null}
      {screen === "history" ? <HistoryScreen /> : null}
      {screen === "settings" ? <SettingsScreen /> : null}
    </AppShell>
  );
}
```

Append to `web/src/styles.css`:

```css
.screen-header { display: flex; justify-content: space-between; gap: 16px; align-items: center; }
.screen-header input { border: 1px solid #b9c0b5; border-radius: 8px; padding: 10px 12px; min-width: 220px; }
.list { display: grid; gap: 12px; }
.list-item { background: #fff; border: 1px solid #d7ddd2; border-radius: 8px; padding: 18px; }
.list-item h2 { font-size: 20px; }
.settings-grid { display: grid; gap: 12px; max-width: 640px; }
.settings-grid div { background: #fff; border: 1px solid #d7ddd2; border-radius: 8px; padding: 16px; }
.settings-grid dt { color: #557064; }
.settings-grid dd { margin: 6px 0 0; font-weight: 700; }
```

- [ ] **Step 6: Run frontend tests**

Run: `npm run test:web -- SavedScreen HistoryScreen SettingsScreen`

Expected: PASS.

Run: `npm run test:web`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add web
git commit -m "feat: add saved history and settings screens"
```

---

### Task 9: End-To-End Main Flow

**Files:**
- Create: `e2e/playwright.config.ts`
- Create: `e2e/git-reel.spec.ts`
- Modify: `package.json`
- Modify: `README.md`

- [ ] **Step 1: Add failing E2E test**

Write `e2e/playwright.config.ts`:

```ts
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  use: {
    baseURL: "http://127.0.0.1:5173",
    trace: "on-first-retry"
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] }
    }
  ],
  webServer: [
    {
      command: "cargo run --manifest-path server/Cargo.toml",
      url: "http://127.0.0.1:4317/api/health",
      reuseExistingServer: true,
      timeout: 120_000
    },
    {
      command: "npm run dev:web",
      url: "http://127.0.0.1:5173",
      reuseExistingServer: true,
      timeout: 120_000
    }
  ]
});
```

Write `e2e/git-reel.spec.ts`:

```ts
import { expect, test } from "@playwright/test";

test("user can connect, browse, save, skip, and inspect local views", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("GitHubに接続するとリールを開始できます")).toBeVisible();

  await page.getByRole("button", { name: "開発用に接続" }).click();
  await expect(page.getByRole("heading", { name: /.+\/.+/ })).toBeVisible();

  await page.getByRole("button", { name: "保存" }).click();
  await page.getByRole("button", { name: "スキップ" }).click();

  await page.getByRole("button", { name: "保存" }).click();
  await expect(page.getByText(/rust-lang\/rust|tauri-apps\/tauri|sqlite\/sqlite/)).toBeVisible();

  await page.getByRole("button", { name: "履歴" }).click();
  await expect(page.getByText(/saved|skipped|viewed/)).toBeVisible();
});
```

- [ ] **Step 2: Run E2E test to verify failure or app issues**

Run: `npx playwright install chromium`

Expected: Chromium is installed for local Playwright.

Run: `npm run test:e2e`

Expected: FAIL if server/frontend wiring has contract mismatches; otherwise PASS.

- [ ] **Step 3: Fix contract mismatches discovered by E2E**

If `GET /api/settings` returns 404, change the settings nest in `server/src/app.rs` from `nest("/api/settings", routes::settings::router())` to a direct route:

```rust
.route("/api/settings", get(routes::settings::settings))
```

and make the `settings` function public in `server/src/routes/settings.rs`:

```rust
pub async fn settings(State(state): State<AppState>) -> Result<Json<SettingsResponse>, ApiError> {
```

If frontend navigation buttons are not found by accessible name, update `web/src/components/AppShell.tsx` button text to exactly `リール`, `保存`, `履歴`, and `設定`.

If saving before any repository appears races with loading, disable action buttons in `RepoCard` while action state is pending by adding a `disabled` prop to buttons and setting it from `ReelScreen`.

- [ ] **Step 4: Run E2E test until it passes**

Run: `npm run test:e2e`

Expected: PASS.

Run: `npm test`

Expected: PASS for frontend unit tests and server tests.

- [ ] **Step 5: Update README with E2E command**

Append to `README.md`:

```markdown
## End-to-end tests

Install the browser once:

```bash
npx playwright install chromium
```

Run the local flow:

```bash
npm run test:e2e
```
```

- [ ] **Step 6: Commit**

```bash
git add README.md package.json package-lock.json e2e server web
git commit -m "test: add end-to-end git reel flow"
```

---

### Task 10: Final Verification And MVP Polish

**Files:**
- Modify: `README.md`
- Modify: `web/src/styles.css`
- Modify: any file with a failing test discovered in this task

- [ ] **Step 1: Run all automated checks**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: PASS.

Run: `npm run test:web`

Expected: PASS.

Run: `npm run test:e2e`

Expected: PASS.

Run: `npm --workspace web run build`

Expected: PASS and `web/dist/` is created.

- [ ] **Step 2: Manually verify local app**

Run server:

```bash
cargo run --manifest-path server/Cargo.toml
```

Run frontend in a second terminal:

```bash
npm run dev:web
```

Open `http://127.0.0.1:5173` and verify:

- Reel starts with `GitHubに接続するとリールを開始できます`.
- `開発用に接続` loads a repository card.
- `次へ`, `前へ`, `保存`, `スキップ`, and `詳細` are visible.
- Keyboard shortcuts `j`, `k`, `s`, `x`, and `d` trigger their actions.
- Saved view shows saved repositories.
- History view shows viewed, saved, and skipped repositories.
- Settings view shows local auth and SQLite state.

- [ ] **Step 3: Tighten Japanese empty and error states**

Verify these exact messages appear where applicable:

```text
GitHubに接続するとリールを開始できます
まだ保存したリポジトリはありません
履歴はまだありません
候補がありません。条件をゆるめて再試行してください
READMEを取得できませんでした
```

If a message differs, update the relevant screen component string and rerun `npm run test:web`.

- [ ] **Step 4: Check styles at desktop and mobile widths**

Run: `npm run dev:web`

Use browser widths `1280px` and `390px`. Confirm no text overlaps in:

- navigation
- repository title
- action buttons
- stats grid
- saved list
- history list
- settings grid

If an action button wraps poorly on mobile, change `.actions` in `web/src/styles.css` to:

```css
.actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
  gap: 10px;
}
```

- [ ] **Step 5: Commit polish**

```bash
git add README.md web server
git commit -m "chore: verify discovery first mvp"
```

---

## Self-Review

**Spec coverage:** Reel, Saved, History, Settings, local SQLite persistence, append-only events, saved state, notes, tags, discovery batches, queue dedupe, Japanese UI strings, development auth, fixture-backed GitHub conversion tests, frontend tests, and E2E flow are covered. Production OAuth token hardening and Tauri packaging are excluded because the spec marks them outside MVP.

**Placeholder scan:** The plan avoids deferred work markers and includes exact files, concrete code, commands, expected outcomes, and commit messages for each task.

**Type consistency:** Backend `Repository`, `RepoEventKind`, `ReelResponse`, `SavedRepository`, `HistoryItem`, and frontend matching types use the same field names as the JSON contract. Route paths in the API contract match the frontend client and E2E test.
