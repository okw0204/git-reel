# OAuth Dev Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ローカル開発で GitHub OAuth の `.env` 設定手順を迷わず使えるようにする。

**Architecture:** サーバー起動時に `.env` を読み込んでから既存の `Config::from_env` を使う。ドキュメントは README に手順を集約し、`.env.example` は実 secret を含まないテンプレートとして追加する。

**Tech Stack:** Rust 2021, Axum, `dotenvy`, npm workspace, Markdown

---

## File Structure

- Modify: `server/Cargo.toml`  
  `dotenvy` を通常依存に追加する。
- Modify: `server/Cargo.lock`  
  `cargo test` または `cargo check` で依存解決結果を更新する。
- Modify: `server/src/main.rs`  
  `build_app()` の前に `.env` を読み込む。`.env` が存在しない場合は無視する。
- Create: `.env.example`  
  ローカル開発向け環境変数のテンプレートを追加する。secret の実値は入れない。
- Modify: `README.md`  
  GitHub OAuth App の作成手順、callback URL、`.env` の扱い、開発用接続との違いを説明する。

## Task 1: `.env` 読み込みを追加する

**Files:**
- Modify: `server/Cargo.toml:14-29`
- Modify: `server/Cargo.lock`
- Modify: `server/src/main.rs:1-16`

- [ ] **Step 1: 依存追加前のサーバーテストを実行する**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: 現在の状態でサーバーテストが通る。失敗した場合は、このタスクの変更を入れる前に失敗内容を記録してユーザーに確認する。

- [ ] **Step 2: `dotenvy` 依存を追加する**

Edit `server/Cargo.toml` dependencies so the section contains `dotenvy = "0.15"`:

```toml
[dependencies]
anyhow = "1"
async-trait = "0.1"
axum = { version = "0.7", features = ["macros"] }
chrono = { version = "0.4", features = ["serde"] }
dotenvy = "0.15"
futures-util = "0.3"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid"] }
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["serde", "v4"] }
```

- [ ] **Step 3: 起動時に `.env` を読み込む**

Replace `server/src/main.rs` with:

```rust
use git_reel_server::build_app;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

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

- [ ] **Step 4: サーバーテストで依存解決と既存挙動を確認する**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: テストが通り、`server/Cargo.lock` に `dotenvy` が追加される。

- [ ] **Step 5: Task 1 をコミットする**

```bash
git add server/Cargo.toml server/Cargo.lock server/src/main.rs
git commit -m "feat: load env file for server"
```

## Task 2: `.env.example` を追加する

**Files:**
- Create: `.env.example`

- [ ] **Step 1: `.env.example` を作成する**

Create `.env.example` with:

```dotenv
# SQLite の接続先。未設定時は sqlite:git-reel.db が使われます。
GIT_REEL_DATABASE_URL=sqlite:git-reel.db

# OAuth token がない場合に GitHub Search API の fallback として使う Personal Access Token。
# GitHub OAuth だけを使う場合は空のままで構いません。
GITHUB_TOKEN=

# GitHub OAuth App の Client ID / Client Secret。
# 両方を設定すると、開発用接続ではなく GitHub OAuth 接続が有効になります。
GITHUB_CLIENT_ID=
GITHUB_CLIENT_SECRET=

# ローカル開発の既定 URL。
# GitHub OAuth App の callback URL は http://127.0.0.1:4317/api/auth/github/callback にします。
GIT_REEL_PUBLIC_BASE_URL=http://127.0.0.1:4317
GIT_REEL_PUBLIC_APP_URL=http://127.0.0.1:5173
```

- [ ] **Step 2: `.env.example` だけが追跡対象になっていることを確認する**

Run: `git status --short`

Expected: `.env.example` は `?? .env.example` として表示される。実 secret を含む `.env` は表示されない。

- [ ] **Step 3: Task 2 をコミットする**

```bash
git add .env.example
git commit -m "docs: add env example"
```

## Task 3: README に OAuth セットアップ手順を追記する

**Files:**
- Modify: `README.md:37-80`

- [ ] **Step 1: README のセットアップ節を更新する**

Replace `README.md` lines 37-80 with:

````markdown
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

`GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定して起動すると、開発用接続は無効になり、リール画面の接続ボタンは「GitHubに接続」になります。OAuth 接続後は保存済み OAuth token を優先して GitHub Search API から候補を取得します。

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
| `GITHUB_TOKEN` | OAuth token がない場合の GitHub Search API fallback 用トークン | 未設定 |
| `GITHUB_CLIENT_ID` | GitHub OAuth App の Client ID | 未設定 |
| `GITHUB_CLIENT_SECRET` | GitHub OAuth App の Client Secret | 未設定 |
| `GIT_REEL_PUBLIC_BASE_URL` | API 側の公開 URL。OAuth コールバック URL の生成に使う | `http://127.0.0.1:4317` |
| `GIT_REEL_PUBLIC_APP_URL` | Web アプリ側の公開 URL。OAuth 完了後の戻り先に使う | `http://127.0.0.1:5173` |

OAuth 未設定時は「開発用に接続」からローカル状態だけを接続済みにできます。OAuth 設定時は開発用接続は無効になり、「GitHubに接続」から OAuth フローを開始します。
````

- [ ] **Step 2: README に secret コミット禁止と callback URL が含まれることを確認する**

Run: `rg "GITHUB_CLIENT_SECRET|Authorization callback URL|コミットしない|開発用に接続|GitHubに接続" README.md`

Expected: それぞれの語句が README 内に表示される。

- [ ] **Step 3: Task 3 をコミットする**

```bash
git add README.md
git commit -m "docs: describe oauth local setup"
```

## Task 4: 全体検証を行う

**Files:**
- Verify: `server/Cargo.toml`
- Verify: `server/src/main.rs`
- Verify: `.env.example`
- Verify: `README.md`

- [ ] **Step 1: サーバーテストを実行する**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected: exit code 0。

- [ ] **Step 2: Web とサーバーの通常テストを実行する**

Run: `npm test`

Expected: exit code 0。

- [ ] **Step 3: secret が追跡対象に入っていないことを確認する**

Run: `git status --short`

Expected: 作業ツリーに未コミット変更がない。`.env` が表示されない。

- [ ] **Step 4: 最終状態を確認する**

Run: `git log --oneline -5`

Expected: 最新 5 件の中に次のコミットが含まれる。

```text
feat: load env file for server
docs: add env example
docs: describe oauth local setup
```

## Self-Review

- Spec coverage: `.env.example`、README の OAuth 手順、secret をコミットしない注意、OAuth 未設定時と設定時の違い、サーバー `.env` 読み込みを各タスクに割り当てた。
- Placeholder scan: 未確定や後回しを示す表現は含めていない。
- Type consistency: Rust 側は既存の `build_app()` と `Config::from_env` をそのまま使い、追加する外部 API は `dotenvy::dotenv()` のみ。
