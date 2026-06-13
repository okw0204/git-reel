# 基本方針

- UI 表示文言は日本語を基本にする。
- GitHub 由来のリポジトリ名・説明・README・topic は元の言語を保つ。

# 開発時の判断基準

- Web から API を呼ぶ処理は `web/src/api/client.ts` に集約する。画面から直接 `fetch` を増やさない。
- サーバーのルーター構築は `server/src/app.rs` に集約する。本番用とテスト用の差分は `Config` に閉じ込める。
- リール候補の補充は保存済み OAuth token を使う GitHub Search API を基本にし、通常実行で seed 候補へ fallback させない。seed 候補はテスト用の候補準備に限定する。
- E2E は外部 GitHub OAuth に依存しない範囲を検証する。

# 作業運用

- テストや起動方法は `Makefile`、`package.json`、`web/package.json`、`server/Cargo.toml` を確認して選ぶ。
