# 基本原則

UI 表示文言は日本語を基本とし、GitHub 由来のリポジトリ名・説明・README・topic は元の言語を保つ。

# 開発コマンド

- 依存関係はルートで `npm install`。root の npm workspace は `web` だけで、Rust サーバーは Cargo workspace ではない。
- 通常のローカル起動は `make dev`。内部で `cargo run --manifest-path server/Cargo.toml` と `npm run dev:web` を同時起動する。
- API は `127.0.0.1:4317`、Vite は `127.0.0.1:5173`。Web 側の `/api` は Vite proxy で API に流れる。
- `npm test` は `npm run test:web && npm run test:server` だけで、E2E は含まない。
- Web の focused test は `npm --workspace web run test -- --run web/src/App.test.tsx` のように Vitest にファイルを渡す。
- Web の型チェック込みビルドは `npm --workspace web run build`。専用の lint/format script は現状ない。
- サーバーテストは `cargo test --manifest-path server/Cargo.toml`。サーバーだけ起動する時も manifest path を明示する。
- E2E は初回に `npx playwright install chromium` が必要。実行は `npm run test:e2e` で、Playwright が API と Vite を自動起動する。

# 実行時の注意

- 通常実行の SQLite は `GIT_REEL_DATABASE_URL` 未指定だと `sqlite:git-reel.db`。ルートから起動するとルート直下に DB ができる。
- 一時 DB でサーバーを起動したい場合は `GIT_REEL_DATABASE_URL=sqlite::memory: cargo run --manifest-path server/Cargo.toml` を使う。インメモリ SQLite は接続数 1 に制限されている。
- DB マイグレーションは `server/src/db.rs` の `sqlx::migrate!("./migrations")` で起動時に自動適用される。
- `GITHUB_TOKEN` は config に読まれるが、現 MVP の通常フローは開発用シード候補で動き、GitHub への書き込みはしない。

# 構成メモ

- フロントエンドの API 境界は `web/src/api/client.ts` に集約されている。画面から直接 `fetch` を増やさない。
- サーバーのルーター構築は `server/src/app.rs`。本番用とテスト用は同じ経路で、差分は `Config` に閉じ込めている。
- 空のリール候補は `DiscoveryService::seed_if_empty` が固定の開発用候補を投入する。候補追加や重複排除の挙動変更は `server/src/discovery.rs` と repository store の両方を確認する。
- E2E は「開発用に接続」から保存・スキップ・履歴確認までの MVP 最短フローを検証している。通常の単体テストと別に必要な変更だけ実行する。
