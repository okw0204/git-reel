use crate::{
    error::ApiError,
    models::{HistoryItem, NewRepository, RepoEventKind, Repository, SavedRepository},
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
            ON CONFLICT(github_id) DO UPDATE SET
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

    pub async fn saved(&self, query: &str) -> Result<Vec<SavedRepository>, ApiError> {
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
            items.push(SavedRepository {
                repository,
                memo: row.get("memo"),
                tags,
                saved_at: row.get("saved_at"),
            });
        }
        Ok(items)
    }

    pub async fn tags_for(&self, repository_id: i64) -> Result<Vec<String>, ApiError> {
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

    pub async fn note_for(&self, repository_id: i64) -> Result<Option<String>, ApiError> {
        let note = sqlx::query_scalar("SELECT body FROM repo_notes WHERE repository_id = ?")
            .bind(repository_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(note)
    }

    pub async fn find_repository(&self, repository_id: i64) -> Result<Repository, ApiError> {
        let row = sqlx::query("SELECT * FROM repositories WHERE id = ?")
            .bind(repository_id)
            .fetch_one(&self.pool)
            .await?;
        repository_from_row(&row)
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
