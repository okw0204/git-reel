# 基本原則

UI 表示文言は日本語を基本とし、GitHub 由来のリポジトリ名・説明・README・topic は元の言語を保つ。

# 開発運用

- 機能変更を伴わない軽微な修正（ドキュメント変更のみなど）を除き、開発では superpowers を利用する。
- ドキュメントを `docs/` に作成する段階から、作業用ブランチを作成する。

# 開発コマンド

- 依存関係はルートで `npm install`。root の npm workspace は `web` だけで、Rust サーバーは Cargo workspace ではない。
- 通常のローカル起動は `make dev`。内部で `cargo run --manifest-path server/Cargo.toml` と `npm run dev:web` を同時起動する。
- API は `127.0.0.1:4317`、Vite は `127.0.0.1:5173`。Web 側の `/api` は Vite proxy で API に流れる。
- `npm test` は `npm run test:web && npm run test:server` だけで、E2E は含まない。Makefile 経由なら `make test`。
- Web の focused test は `npm --workspace web run test -- --run web/src/App.test.tsx` のように Vitest にファイルを渡す。
- Web の型チェック込みビルドは `npm --workspace web run build`。Makefile 経由なら `make build`。専用の lint/format script は現状ない。
- サーバーテストは `cargo test --manifest-path server/Cargo.toml`。Makefile 経由なら `make test-server`。サーバーだけ起動する時も manifest path を明示する。
- E2E は初回に `npx playwright install chromium` が必要。実行は `npm run test:e2e` または `make test-e2e` で、Playwright が API と Vite を自動起動する。

# 実行時の注意

- 通常実行の SQLite は `GIT_REEL_DATABASE_URL` 未指定だと `sqlite:git-reel.db`。ルートから起動するとルート直下に DB ができる。
- 一時 DB でサーバーを起動したい場合は `GIT_REEL_DATABASE_URL=sqlite::memory: cargo run --manifest-path server/Cargo.toml` を使う。インメモリ SQLite は接続数 1 に制限されている。
- DB マイグレーションは `server/src/db.rs` の `sqlx::migrate!("./migrations")` で起動時に自動適用される。
- 候補が空のときは、OAuth 接続済みの保存済み `auth_state.access_token` を使って GitHub Search API から最近更新されたリポジトリを補充する。OAuth token がない場合や取得失敗時、通常実行では seed へフォールバックしない。
- ローカル開発でもリールを開始するには `GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定し、GitHub OAuth で接続する。OAuth の URL 生成には `GIT_REEL_PUBLIC_BASE_URL` と `GIT_REEL_PUBLIC_APP_URL` を使い、未指定時はそれぞれ `http://127.0.0.1:4317` と `http://127.0.0.1:5173`。
- GitHub OAuth は接続状態とアクセストークンをローカル DB に保存する。現 MVP は GitHub への書き込みはしない。

# 構成メモ

- フロントエンドの API 境界は `web/src/api/client.ts` に集約されている。画面から直接 `fetch` を増やさない。
- サーバーのルーター構築は `server/src/app.rs`。本番用とテスト用は同じ経路で、差分は `Config` に閉じ込めている。
- 空のリール候補は `DiscoveryService::ensure_candidates` が保存済み OAuth token だけで補充を試す。候補追加や重複排除の挙動変更は `server/src/discovery.rs`、`server/src/github.rs`、repository store を確認する。
- seed 候補は通常実行の fallback ではなく、テスト用の候補準備に限定する。
- E2E は OAuth 未設定時の設定案内など、外部 GitHub OAuth に依存しない範囲を検証する。通常の単体テストと別に必要な変更だけ実行する。
