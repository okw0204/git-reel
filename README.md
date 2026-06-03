# git-reel

Git Reel は、GitHub リポジトリをリール形式で気軽に眺めるための local-first アプリです。検索 UI や GitHub Trending の置き換えではなく、1 件ずつ候補を見て、気になったものをローカルの「保存済み」として残す体験に重点を置いています。

現在の MVP はローカル開発向けです。GitHub への書き込みは行わず、OAuth 接続または開発用接続でリール操作、保存、スキップ、履歴、メモ、タグを確認できます。OAuth 接続済みの場合は保存済み OAuth token で starred repositories を読み取り、language と topic の傾向から GitHub Search API の候補を補充します。未接続、取得失敗、または候補を採用できない場合は `GITHUB_TOKEN`、最後にシードされた候補リポジトリへフォールバックします。

## 主な機能

- リポジトリ候補を 1 件ずつ表示するリール画面
- 次へ、前へ、保存、スキップ、詳細表示
- 保存済みリポジトリの一覧と検索
- リポジトリごとのメモとタグ
- 閲覧・保存・スキップなどのローカル履歴
- GitHub OAuth 接続、または OAuth 未設定時の開発用接続
- OAuth 接続済み token による starred repositories 起点の候補補充、または `GITHUB_TOKEN` 設定時の GitHub Search API fallback
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

OAuth を使う場合は、`.env.example` をコピーして `.env` を作成します。

```bash
cp .env.example .env
```

`.env` には GitHub OAuth App の Client ID と Client Secret など、ローカルだけで使う値を設定します。`.env` は `.gitignore` で除外済みです。`GITHUB_CLIENT_SECRET` や `GITHUB_TOKEN` の実値はコミットしないでください。

## GitHub OAuth App の作成

ローカル開発で GitHub OAuth 接続を使う場合は、GitHub の Developer settings で OAuth App を作成します。

- Homepage URL: `http://127.0.0.1:5173`
- Authorization callback URL: `http://127.0.0.1:4317/api/auth/github/callback`

作成後、OAuth App の Client ID と Client Secret を `.env` に設定します。

```dotenv
GITHUB_CLIENT_ID=your_client_id
GITHUB_CLIENT_SECRET=your_client_secret
```

`GIT_REEL_PUBLIC_BASE_URL` を変更した場合は、GitHub OAuth App 側の Authorization callback URL も同じ API URL に合わせてください。既定値では `http://127.0.0.1:4317/api/auth/github/callback` です。

## 開発サーバーの起動

ローカル API と Web アプリをまとめて起動します。

```bash
make dev
```

ブラウザで Vite が表示する URL を開きます。

OAuth 未設定時は、リール画面の「開発用に接続」からローカルのシード候補を閲覧できます。この接続はローカル状態だけを接続済みにする開発用の導線で、GitHub OAuth token は保存されません。

`GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定して起動すると、開発用接続は無効になり、リール画面の接続ボタンは「GitHubに接続」になります。OAuth 接続後は保存済み OAuth token で starred repositories を読み取り、その language/topic 傾向から候補を取得します。starred 取得や Search が失敗した場合、または候補を採用できない場合は `GITHUB_TOKEN`、最後にローカル seed へフォールバックします。

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
| `GITHUB_TOKEN` | OAuth token がない場合、取得失敗時、候補を採用できない場合の GitHub Search API fallback 用トークン | 未設定 |
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

通常の開発実行では SQLite データベースにローカル状態を保存します。保存済み、履歴、メモ、タグ、接続状態は GitHub に送信しません。OAuth のアクセストークンは starred repositories、Search、README 取得など GitHub API の読み取りに使いますが、GitHub への書き込みは行いません。

テストではインメモリの SQLite を使うため、通常のローカルデータとは分離されています。

## 設計メモ

- UI の表示文言は日本語を基本にしています。
- GitHub 由来のリポジトリ名、説明、README、トピックなどは元の言語を保持します。
- MVP では GitHub Star、Watch、リポジトリ情報の更新など、GitHub への書き込み操作は扱いません。
- OAuth 接続済みの場合は保存済み OAuth token で starred repositories の language/topic 傾向を読み取り、OAuth token がない場合、取得失敗時、候補を採用できない場合は `GITHUB_TOKEN`、最後にローカルのシード候補へフォールバックします。
- 将来的な Tauri 化を見据え、フロントエンドとローカル API の境界を明確にしています。
