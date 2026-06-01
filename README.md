# git-reel

Git Reel は、GitHub リポジトリをリール形式で気軽に眺めるための local-first アプリです。検索 UI や GitHub Trending の置き換えではなく、1 件ずつ候補を見て、気になったものをローカルの「保存済み」として残す体験に重点を置いています。

現在の MVP はローカル開発向けです。GitHub への書き込みは行わず、OAuth 接続または開発用接続でリール操作、保存、スキップ、履歴、メモ、タグを確認できます。`GITHUB_TOKEN` を設定すると GitHub Search API から候補を補充し、未設定または取得失敗時はシードされた候補リポジトリを使います。

## 主な機能

- リポジトリ候補を 1 件ずつ表示するリール画面
- 次へ、前へ、保存、スキップ、詳細表示
- 保存済みリポジトリの一覧と検索
- リポジトリごとのメモとタグ
- 閲覧・保存・スキップなどのローカル履歴
- GitHub OAuth 接続、または OAuth 未設定時の開発用接続
- `GITHUB_TOKEN` 設定時の GitHub Search API による候補補充
- Rust API、React/Vite フロントエンド、SQLite による local-first 構成

## ディレクトリ構成

```text
.
├── server/   # Rust / Axum のローカル API と SQLite マイグレーション
├── web/      # React / Vite のフロントエンド
├── e2e/      # Playwright によるローカルフローの E2E テスト
└── docs/     # 仕様・実装計画などの設計資料
```

## 必要なもの

- Node.js
- npm
- Rust toolchain
- SQLite を扱える環境

Playwright の E2E テストを実行する場合は、Chromium ブラウザのインストールも必要です。

## セットアップ

フロントエンド依存関係をインストールします。

```bash
npm install
```

## 開発サーバーの起動

ローカル API と Web アプリをまとめて起動します。

```bash
make dev
```

ブラウザで Vite が表示する URL を開き、リール画面の「開発用に接続」からローカルのシード候補を閲覧できます。

`GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定して起動すると、リール画面の接続ボタンは GitHub OAuth 接続になります。OAuth のコールバック URL は既定で `http://127.0.0.1:4317/api/auth/github/callback` です。

API と Web アプリを個別に確認したい場合は、別々のターミナルで起動します。

```bash
cargo run --manifest-path server/Cargo.toml
```

別のターミナルで次を実行します。

```bash
npm run dev:web
```

## 環境変数

| 変数 | 用途 | 既定値 |
| --- | --- | --- |
| `GIT_REEL_DATABASE_URL` | SQLite の接続先 | `sqlite:git-reel.db` |
| `GITHUB_TOKEN` | GitHub Search API から候補を補充するためのトークン | 未設定 |
| `GITHUB_CLIENT_ID` | GitHub OAuth App の Client ID | 未設定 |
| `GITHUB_CLIENT_SECRET` | GitHub OAuth App の Client Secret | 未設定 |
| `GIT_REEL_PUBLIC_BASE_URL` | API 側の公開 URL。OAuth コールバック URL の生成に使う | `http://127.0.0.1:4317` |
| `GIT_REEL_PUBLIC_APP_URL` | Web アプリ側の公開 URL。OAuth 完了後の戻り先に使う | `http://127.0.0.1:5173` |

OAuth 未設定時は「開発用に接続」からローカル状態だけを接続済みにできます。OAuth 設定時は開発用接続は無効になり、「GitHubに接続」から OAuth フローを開始します。

## テスト

フロントエンドとサーバーのテストをまとめて実行します。

```bash
npm test
```

個別に実行する場合は、次のコマンドを使います。

```bash
npm run test:web
npm run test:server
```

E2E テスト用ブラウザを初回だけインストールします。

```bash
npx playwright install chromium
```

E2E テストを実行します。

```bash
npm run test:e2e
```

Makefile 経由でも実行できます。

```bash
make test
make test-web
make test-server
make test-e2e
make build
```

## ローカルデータ

通常の開発実行では SQLite データベースにローカル状態を保存します。保存済み、履歴、メモ、タグ、接続状態、OAuth のアクセストークンは GitHub 側には送信されません。

テストではインメモリの SQLite を使うため、通常のローカルデータとは分離されています。

## 設計メモ

- UI の表示文言は日本語を基本にしています。
- GitHub 由来のリポジトリ名、説明、README、トピックなどは元の言語を保持します。
- MVP では GitHub Star、Watch、リポジトリ情報の更新など、GitHub への書き込み操作は扱いません。
- `GITHUB_TOKEN` 設定時の候補取得は最近更新されたリポジトリの検索に限定し、失敗時はローカルのシード候補へフォールバックします。
- 将来的な Tauri 化を見据え、フロントエンドとローカル API の境界を明確にしています。
