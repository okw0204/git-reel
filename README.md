# git-reel

Git Reel は、GitHub リポジトリをリール形式で気軽に眺めるための local-first アプリです。検索 UI や GitHub Trending の置き換えではなく、1 件ずつ候補を見て、気になったものをローカルの「保存済み」として残す体験に重点を置いています。

現在の MVP はローカル開発向けです。GitHub への書き込みは行わず、開発用接続ではシードされた候補リポジトリを使ってリール操作、保存、スキップ、履歴、メモ、タグを確認できます。

## 主な機能

- リポジトリ候補を 1 件ずつ表示するリール画面
- 次へ、前へ、保存、スキップ、詳細表示
- 保存済みリポジトリの一覧と検索
- リポジトリごとのメモとタグ
- 閲覧・保存・スキップなどのローカル履歴
- 開発用の疑似 GitHub 接続
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

ローカル API を起動します。

```bash
cargo run --manifest-path server/Cargo.toml
```

別のターミナルで Web アプリを起動します。

```bash
npm run dev:web
```

ブラウザで Vite が表示する URL を開き、リール画面の「開発用に接続」からローカルのシード候補を閲覧できます。

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

## ローカルデータ

通常の開発実行では SQLite データベースにローカル状態を保存します。保存済み、履歴、メモ、タグ、開発用接続状態は GitHub 側には送信されません。

テストではインメモリの SQLite を使うため、通常のローカルデータとは分離されています。

## 設計メモ

- UI の表示文言は日本語を基本にしています。
- GitHub 由来のリポジトリ名、説明、README、トピックなどは元の言語を保持します。
- MVP では GitHub Star、Watch、リポジトリ情報の更新など、GitHub への書き込み操作は扱いません。
- 将来的な Tauri 化を見据え、フロントエンドとローカル API の境界を明確にしています。
