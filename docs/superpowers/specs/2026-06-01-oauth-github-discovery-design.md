# OAuth トークンによる GitHub Discovery 設計

## 背景

Git Reel はすでに GitHub OAuth 接続で `auth_state.access_token` と `username` を保存できる。現在の実 GitHub Discovery は `GITHUB_TOKEN` が設定されている場合だけ Search API を呼び、OAuth 接続後の保存済みトークンは候補取得に使っていない。

この変更では、OAuth 接続済みユーザーのアクセストークンを使って実リポジトリ候補を取得し、可能な範囲で README preview も補完する。既存の local-first な保存済み、履歴、メモ、タグは引き続きローカル DB に閉じる。GitHub への書き込み操作は追加しない。

## 目的

- OAuth 接続後、`auth_state.access_token` を使って GitHub Search API から実候補を取得する。
- Search API の候補に対して、GraphQL API で README preview を補完する。
- 既存の discovery queue、batch 記録、重複排除、操作済みリポジトリ除外を再利用する。
- OAuth token が使えない場合も、リール体験を止めずに `GITHUB_TOKEN` または seed にフォールバックする。
- 通常のテストは live GitHub API に依存させない。
- フロントエンドの API 契約は基本的に変更しない。

## 対象外

- GitHub への Star、Watch、Issue 作成などの書き込み操作。
- starred repositories、user repositories、following など認証ユーザー文脈を使った新しい候補戦略。
- 手動同期 API やバックグラウンド同期ジョブ。
- rate limit の詳細な UI 表示。
- 複数ユーザーや本格的なセッション管理。

## 採用方針

採用する方針は、Discovery 実行時に DB から OAuth token を読む方式とする。

`/api/reel/current` や `/api/reel/next` は今まで通り `DiscoveryService::ensure_candidates()` を呼ぶ。queue が空の場合、`DiscoveryService` は `RepositoryStore` 経由で `auth_state.access_token` を取得する。token が存在すれば、その token で `GitHubClient` を作り、Search API と README preview 補完を実行する。

この方式は、既存の `auth_state` と `DiscoveryService` の責務に沿っている。`AppState` を可変共有状態にする必要がなく、OAuth callback 後にメモリ上の GitHub client を差し替える同期問題も避けられる。

## アーキテクチャ

主な責務分担は次の通り。

- `routes/reel.rs`: 既存通り `DiscoveryService::ensure_candidates()` を呼ぶ。
- `DiscoveryService`: queue が空のときの token 優先順位、GitHub discovery、fallback を制御する。
- `GitHubClient`: Search API、GraphQL API、JSON 変換を担当する。
- `RepositoryStore`: DB から保存済み OAuth token を返す。

`RepositoryStore` には `auth_access_token()` のような小さな取得メソッドを追加する。このメソッドは `auth_state` の id 1 から `connected = 1` かつ `access_token IS NOT NULL` の token だけを返す。

`DiscoveryService` は OAuth token 由来の一時的な `GitHubClient` を最優先で使う。既存の `AppState.github_client` は `GITHUB_TOKEN` 由来の fallback client として残す。

## データフロー

OAuth 接続後のリール表示は次の流れになる。

1. ユーザーが GitHub OAuth で接続する。
2. OAuth callback が `auth_state.access_token` と `username` を保存する。
3. ユーザーがリールを開く、または「次へ」を押す。
4. `routes/reel.rs` が `DiscoveryService::ensure_candidates()` を呼ぶ。
5. queue が空なら `DiscoveryService` が DB から OAuth token を読む。
6. OAuth token があれば `GitHubClient` で Search API を呼ぶ。
7. 取得した候補に対して README preview を GraphQL API で補完する。
8. 既存の `enqueue_candidates()` で DB に upsert し、queue に入れる。
9. queue から 1 件 claim して画面に返す。

token の優先順位は次の通り。

1. `auth_state.access_token`
2. `GITHUB_TOKEN` 由来の既存 `AppState.github_client`
3. ローカル seed

## README preview 補完

Search API の結果を `NewRepository` に変換した後、各候補について GraphQL API で README を取得する。既存の `parse_graphql_readme_preview()` を活用し、GitHub HTTP 境界だけを `GitHubClient` に追加する。

README preview は候補品質を上げる補助情報として扱う。README が存在しない、GraphQL API が失敗する、個別候補の parse に失敗する、といった場合でも discovery 全体は失敗させない。その候補の `readme_preview` を `None` のままにして続行する。

初期実装では候補数が最大 30 件のため、README 補完も同期的に行う。ただし、初回表示が重くなりすぎる場合に備え、失敗時に即 fallback するのではなく、README なし候補として返せる構造にする。

## エラー処理

リール体験を止めない方針にする。

- OAuth token がない場合: `GITHUB_TOKEN` client を試し、なければ seed にフォールバックする。
- OAuth token で Search API が失敗した場合: warning を出し、`GITHUB_TOKEN` client または seed にフォールバックする。
- `GITHUB_TOKEN` client も失敗した場合: warning を出し、seed にフォールバックする。
- README GraphQL が失敗した場合: その候補だけ `readme_preview = None` として続行する。
- Search API の取得候補が 0 件の場合: fallback する。
- enqueue 後の採用候補が 0 件の場合: fallback する。
- DB エラーの場合: 既存通り API エラーとして返す。

OAuth token が失効している可能性がある場合でも、この変更では接続状態の自動解除は行わない。接続状態の明示的な失効表示や再接続導線は後続の改善対象とする。

## フロントエンドと文言

フロントエンドの API 契約は変更しない。既存のリール表示、保存、スキップ、履歴、詳細表示はそのまま使う。

ただし、未接続画面の説明文に「OAuth 接続後もシード候補を使います」という趣旨の古い文言があるため、実データ取得に合わせて更新する。GitHub 由来のリポジトリ名、説明、README、topic は元の言語を保つ。

README の環境変数説明も更新する。`GITHUB_TOKEN` は引き続き使えるが、OAuth 接続済みの場合は保存済み OAuth token が優先されることを明記する。

## テスト方針

通常のテストは live GitHub API に依存させない。

サーバーテストでは次を確認する。

- `RepositoryStore` が接続済み `auth_state.access_token` を取得できる。
- 未接続または token なしの場合は OAuth token を返さない。
- OAuth token がある場合、Discovery は OAuth token 由来の GitHub client を優先する。
- OAuth token がない場合、既存の `GITHUB_TOKEN` client を使う。
- OAuth token client が失敗した場合、`GITHUB_TOKEN` client または seed にフォールバックする。
- Search API fixture から候補を作れる。
- GraphQL README fixture から preview を補完できる。
- README 補完失敗は候補取得全体を失敗させない。

フロントエンドテストでは、未接続画面の説明文が実データ取得後の挙動と矛盾しないことを確認する。

## 実装上の注意

最小変更を優先する。`AppState` を可変にしたり、同期専用 API を増やしたりしない。OAuth token の読み取りは `RepositoryStore` に閉じ込め、GitHub HTTP 呼び出しは `GitHubClient` に閉じ込める。

README preview 補完は候補取得の補助であり、失敗しても候補取得全体を失敗させない。Search API と seed fallback の信頼性を優先する。
