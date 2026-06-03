# ローカル開発の OAuth 設定導線

## 背景

Git Reel は GitHub OAuth を設定すると、保存済み OAuth token を使って GitHub Search API からリポジトリ候補を補充できる。一方で、現状の README は設定項目と既定 URL の説明が最小限で、初回セットアップ時に GitHub OAuth App の callback URL や `.env` の扱いを迷いやすい。

Issue #7 では、ローカル開発で OAuth 接続に必要な設定を確認しやすくし、secret を git に入れない導線を明確にする。

## 目的

- ローカル開発で GitHub OAuth 接続を有効にする手順を README だけで追えるようにする。
- `.env.example` を用意し、必要な環境変数と既定値を確認しやすくする。
- `.env` に書いた値をサーバー起動時に読み込めるようにし、`make dev` でも同じ導線を使えるようにする。
- OAuth 未設定時の開発用接続と、OAuth 設定時の GitHub OAuth 接続の違いを明記する。
- `GITHUB_CLIENT_SECRET` や `GITHUB_TOKEN` をコミットしない前提を明記する。

## 対象範囲

今回扱う変更は、ローカル開発向けの設定導線に限定する。

- README の OAuth セットアップ手順を拡充する。
- `.env.example` を追加する。
- サーバー起動時に `.env` を読み込む。

次の変更は対象外とする。

- 本番環境向けの OAuth 設定手順。
- UI 上の設定ガイド表示。
- GitHub OAuth の権限スコープや認可フロー自体の変更。
- GitHub への書き込み操作の追加。

## アプローチ

推奨案は、`README + .env.example + サーバー側の .env 読み込み` の組み合わせとする。

README だけを更新する案は最小だが、実際に `.env` を置いて `make dev` したときに値が読み込まれないままだと手順として不完全になりやすい。UI にヒントを追加する案は親切だが、Issue の受け入れ条件に対して実装範囲が広がる。今回はローカル開発者の初回セットアップを主対象にして、ドキュメントと設定読み込みに絞る。

## 設計

### `.env.example`

リポジトリ直下に `.env.example` を追加する。実際の secret は含めず、空値またはコメント付きの例だけを置く。

含める項目は次の通り。

- `GIT_REEL_DATABASE_URL`
- `GITHUB_TOKEN`
- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GIT_REEL_PUBLIC_BASE_URL`
- `GIT_REEL_PUBLIC_APP_URL`

`GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` は GitHub OAuth 接続を有効にするために両方必要であることを README 側で説明する。`.env.example` には実値を入れない。

### README

README のセットアップと環境変数の説明を整理する。

GitHub OAuth App の作成手順では、ローカル開発の既定値として次を明記する。

- Homepage URL: `http://127.0.0.1:5173`
- Authorization callback URL: `http://127.0.0.1:4317/api/auth/github/callback`

`.env` の作り方として、`.env.example` をコピーして必要な値を埋める流れを示す。`.env` は `.gitignore` で除外済みであり、secret をコミットしないことを注意書きとして明記する。

OAuth 未設定時はリール画面の「開発用に接続」でローカル状態だけを接続済みにできる。OAuth 設定時は開発用接続が無効になり、「GitHubに接続」から OAuth フローを開始する。この違いを README の開発サーバー起動または OAuth セットアップ節で説明する。

### サーバー設定

サーバー起動時にリポジトリ直下の `.env` を読み込む。既存の `Config::from_env` は `std::env` から値を読む構造なので、`.env` 読み込みは設定生成の前に 1 回行う。

Rust 側では `dotenvy` を使う。`.env` が存在しない場合はエラーにせず、従来通り OS 環境変数と既定値で起動できるようにする。これにより、CI や既存のローカル実行は壊さない。

## データフロー

1. 開発者が `.env.example` を `.env` にコピーする。
2. 開発者が GitHub OAuth App の Client ID と Client Secret を `.env` に設定する。
3. `make dev` で Rust API と Vite を起動する。
4. Rust API が起動時に `.env` を読み込む。
5. `Config::from_env` が `GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を取得する。
6. OAuth 設定ありとして `/api/auth` 系の GitHub OAuth フローが有効になる。
7. Web 側では「GitHubに接続」から OAuth フローを開始する。

## エラーハンドリング

`.env` が存在しない場合は正常系として扱う。OAuth 用の環境変数が未設定の場合は、既存通り開発用接続を使える状態にする。

OAuth App 側の callback URL が間違っている場合は GitHub 側の認可または callback で失敗するため、README に既定 callback URL を明記して原因を追いやすくする。

## テスト

主な変更はドキュメントと設定読み込みなので、次の確認を行う。

- `cargo test --manifest-path server/Cargo.toml`
- `npm test`

`.env` 読み込みの追加で既存テストが壊れないことを確認する。必要なら、設定読み込みの単体テストで `.env` の存在に依存しないことも確認する。

## 受け入れ条件との対応

- ローカル開発で OAuth 接続に必要な設定が分かる: README に GitHub OAuth App 作成手順、callback URL、環境変数を記載する。
- secret をコミットしない前提が明記されている: README と `.env.example` で `.env` に実値を書くこと、`.env` をコミットしないことを明記する。
- OAuth 未設定時の開発用接続との違いが分かる: README に OAuth 未設定時と設定時の接続ボタン・挙動の違いを記載する。
