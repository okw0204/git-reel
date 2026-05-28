# Real GitHub Discovery Design

## Overview

This change adds the smallest useful live GitHub discovery path to Git Reel. The app keeps the current local-first reel flow and development seed behavior, but when `GITHUB_TOKEN` is set and the discovery queue is empty, the server tries to fetch repository candidates from the GitHub Search API.

The goal is to validate real candidate quality without taking on GitHub OAuth, background jobs, or GraphQL enrichment yet.

## Goals

- Fetch real repository candidates from GitHub Search API when `GITHUB_TOKEN` is available.
- Preserve the existing local seed fallback for tokenless local development and API failures.
- Reuse the existing discovery queue, batch logging, deduplication, and prior-interaction filtering.
- Keep default tests independent from live GitHub API calls.
- Avoid changing the frontend contract for this first live discovery step.

## Non-Goals

- Do not implement GitHub OAuth.
- Do not write to GitHub.
- Do not add background discovery jobs or retry scheduling.
- Do not fetch README previews through GraphQL during candidate discovery.
- Do not add detailed rate-limit UI in this step.

## Architecture

`Config.github_token` becomes available to the application state. The existing development connection gate remains unchanged: the user still starts the reel through the current dev-connect flow, and OAuth is not introduced. After that gate passes, the reel routes continue to ask `DiscoveryService` to ensure candidates exist before returning `current` or `next`.

`github.rs` gains a small `GitHubClient` that owns live Search API access. It builds the search request, sends it with the configured token, and converts the response through the existing `parse_search_response` boundary into `NewRepository` values.

`DiscoveryService` receives the repository store and an optional GitHub client. When the queue is empty, it first tries live GitHub discovery if a client exists. Successful live candidates flow into the existing `enqueue_candidates` method. If there is no token, the API call fails, parsing fails, or no candidates are accepted, the service falls back to the existing local seed candidates.

This keeps GitHub-specific HTTP code out of routes and keeps queue behavior centralized in `DiscoveryService`.

## Discovery Query

The first live query is intentionally one simple strategy:

```text
stars:10..5000 fork:false archived:false pushed:>YYYY-MM-DD sort:updated-desc
```

`YYYY-MM-DD` is computed as roughly 90 days before the request. This favors recently maintained repositories while avoiding both empty toy projects and extremely famous repositories. The query string is recorded in `discovery_batches.query`, and the strategy is recorded as `recently_updated_live_search`.

## Error Handling

- Missing `GITHUB_TOKEN`: skip live discovery and use local seed candidates.
- GitHub HTTP error: log the failure and use local seed candidates.
- Rate limit response: treat as a GitHub HTTP error for now; saved and history views remain unaffected.
- JSON parse error: treat as a GitHub discovery failure and use local seed candidates.
- Zero fetched or zero accepted candidates: use local seed candidates.

The reel API should continue returning the existing `auth_required` and `queue_empty` shapes. No frontend API contract change is required for this step.

## Testing

Default tests must not depend on live GitHub.

Rust tests should cover:

- Existing Search API fixture conversion still works.
- Query construction produces the intended live search shape.
- No-token discovery falls back to local seed candidates.
- Live candidates returned by a test GitHub client are enqueued through `enqueue_candidates`.
- GitHub discovery failure falls back to local seed candidates.

Frontend tests do not need changes unless backend response shapes change, which this design avoids.

## Future Work

After this step is working, improve discovery quality with multiple query strategies, observable rate-limit behavior, and optional detail enrichment. GitHub OAuth should wait until real candidate discovery feels useful enough to justify the extra auth complexity.
