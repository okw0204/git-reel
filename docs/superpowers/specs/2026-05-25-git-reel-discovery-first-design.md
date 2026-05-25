# Git Reel Discovery-First MVP Design

## Overview

Git Reel is a local-first app for casually discovering GitHub repositories in a short-form, reel-like flow. It is not a search UI or a Trending clone. The main experience is opening the app, moving through one repository at a time, and saving repositories that feel interesting but are not necessarily worth starring on GitHub.

The MVP focuses on validating the discovery experience. It fetches repositories from GitHub, presents them as a single-card reel, records local interactions, and lets the user maintain a personal "curious" collection with custom tags and notes.

The UI language is Japanese by default. Repository names, README content, descriptions, topics, and other GitHub-originated content remain in their original language.

## Goals

- Show GitHub repositories one at a time in a casual browsing flow.
- Let the user move to the next repository and return to previously viewed repositories.
- Let the user save repositories separately from GitHub Star.
- Record skip or not-interested actions.
- Keep viewing history locally.
- Support user-defined tags and notes per repository.
- Provide saved and history views.
- Avoid re-showing repositories the user has already seen where practical.
- Surface not only famous repositories, but also lower-star repositories that look promising.
- Keep the app local-first and lightweight.

## Non-Goals For MVP

- Do not write to GitHub, including Star, Watch, or repository metadata changes.
- Do not build a full GitHub search replacement.
- Do not build a production-grade recommender in the first version.
- Do not require Tauri for the MVP, though the architecture should keep a future Tauri version realistic.

## Architecture

The MVP uses a local web architecture:

- React/Vite frontend
- Rust local API server
- SQLite local database
- GitHub OAuth
- GitHub Search API for candidate discovery
- GitHub GraphQL API for supplemental repository details

The React frontend owns the user interface, client-side interaction state, keyboard handling, and navigation between Reel, Saved, History, and Settings.

The Rust local API owns GitHub OAuth handling, GitHub API calls, discovery queue generation, database reads and writes, and repository action endpoints. This keeps local data and GitHub integration out of the frontend and preserves a clean boundary for a future Tauri shell.

SQLite stores repositories, local user actions, saved state, notes, tags, discovery batches, and auth state. The implementation should keep token storage conservative and revisit safer OS-backed storage when moving toward a packaged desktop app.

## Screens

### Reel

The Reel screen is the primary experience. It shows one repository card at a time with a detail drawer.

The lightweight card shows:

- Repository owner/name
- Description
- Primary language
- Star count
- Fork count
- License when available
- Last updated date
- GitHub topics
- Link to open the repository on GitHub

The detail drawer shows:

- README summary or README preview when available
- User-defined tags
- User memo
- Additional metadata fetched through GraphQL when available

The main actions are:

- Next repository
- Previous repository
- Save as "curious"
- Skip / not interested
- Open detail drawer
- Open on GitHub

The UI exposes visible buttons and keyboard shortcuts. The exact shortcut mapping can be finalized during implementation, but the intended actions are next, previous, save, skip, and detail toggle.

### Saved

Saved shows repositories the user marked as personally interesting. It is separate from GitHub Star. Users can filter saved repositories and view or edit their tags and notes.

### History

History shows repositories that appeared in the Reel, including viewed-only, saved, and skipped repositories. It lets the user recover something they saw earlier and supports the app's duplicate-avoidance behavior.

### Settings

Settings contains:

- GitHub OAuth connection state
- Minimal discovery mix controls
- Local database or export-related controls

Advanced discovery tuning should stay out of the main Reel flow.

## Data Model

The SQLite schema should include these conceptual tables:

- `repositories`: GitHub repository identity and metadata. Use `github_id` where available and keep `owner/name` as a stable display and fallback identity.
- `repo_events`: append-only local events such as viewed, saved, skipped, returned, and detail-opened.
- `saved_repositories`: local saved state for the user's "curious" list.
- `repo_notes`: per-repository memo text.
- `tags`: user-defined tag records.
- `repo_tags`: many-to-many repository/tag links.
- `discovery_batches`: records of query strategy, source API, and candidate batch metadata.
- `auth_state`: OAuth token and expiration metadata, subject to safer storage decisions during implementation.

Repositories should be deduplicated by `github_id` when available, otherwise by normalized `owner/name`. Repositories already displayed in the Reel should not be reintroduced into the normal queue. Reopening from Saved or History is allowed.

Skip and not-interested actions are stored as events. They should exclude the repository from ordinary rediscovery and provide data for future recommendation improvements.

## Discovery Logic

The MVP uses GitHub Search API to gather candidate repositories and GraphQL to enrich details needed by cards and the detail drawer.

Discovery should combine two priorities:

- Find repositories that appear to be starting to gain interest.
- Maintain diversity across languages, topics, star bands, and update recency.

Initial query strategies can include:

- Recently updated repositories with low-to-medium stars.
- Recently created repositories with some minimum activity.
- Language-diverse repository searches.
- Topic and language combinations chosen from a rotating pool.
- Non-fork and non-archived repositories where possible.

Before adding candidates to the queue, the app checks SQLite for prior viewed, saved, or skipped repositories and excludes those from the normal feed.

This area is expected to need adjustment after real use. The first implementation should make the query strategy observable through `discovery_batches` and keep the code easy to tune.

## Error Handling And Empty States

OAuth not connected:

Show a Japanese prompt in Reel and Settings explaining that GitHub connection is needed to start the reel.

GitHub rate limit:

Show the reset time when available. Saved and History remain usable.

Candidate queue empty:

Try to fetch a new discovery batch. If that fails, show a Japanese message suggesting retry or relaxed discovery conditions.

Network error:

Show a retry action and keep local views available.

Repository detail fetch failure:

Show the lightweight card if available and display a detail fetch error in the drawer.

Local database error:

Do not mark the action as successful. Show a retryable error.

Empty states should be short, concrete, and Japanese. Examples:

- `まだ保存したリポジトリはありません`
- `GitHubに接続するとリールを開始できます`

## Testing Strategy

Rust API tests should cover:

- Repository persistence
- Save action
- Skip action
- History event recording
- Repository deduplication
- Discovery queue behavior
- SQLite migration creation and basic CRUD

GitHub client tests should use fixtures for Search API and GraphQL response conversion. Tests should avoid depending on live GitHub API calls by default.

Frontend tests should cover:

- Reel next and previous actions
- Save and skip actions
- Detail drawer behavior
- Saved list rendering
- History list rendering
- Settings OAuth state rendering

End-to-end tests should cover the main local flow with mocked or development auth:

1. Load Reel.
2. Show a repository candidate.
3. Save one repository.
4. Skip another repository.
5. Confirm Saved and History reflect the actions.

Discovery quality has no fixed correct answer in MVP. Tests should instead verify that already viewed repositories are not reintroduced, candidate batches can refill the queue, and query strategy output does not collapse into one narrow category.

## Open Implementation Decisions

- Exact Rust web framework.
- Exact React routing and state management libraries.
- OAuth token storage mechanism for the local development version.
- Exact keyboard shortcut mapping.
- Initial list of language/topic/star-band query strategies.

These are implementation-level choices and should be resolved in the implementation plan without changing the product direction above.
