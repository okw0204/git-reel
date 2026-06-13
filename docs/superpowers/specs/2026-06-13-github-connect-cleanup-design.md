# GitHub 接続整理設計

## 背景

Git Reel は現在、GitHub 接続に複数の経路を持っている。正規の GitHub OAuth 接続に加えて、OAuth 未設定時の `/api/auth/dev-connect`、OAuth token がない場合の `GITHUB_TOKEN`、最後の seed fallback がある。

今回の変更では、通常利用とローカル開発の接続経路を GitHub OAuth に一本化する。疑似的な接続や環境変数 token による候補補充を削除し、seed はテスト用途に限定する。

## 目的

- 通常実行では GitHub OAuth 接続だけを接続手段にする。
- `/api/auth/dev-connect` と Web 側の開発用接続導線を削除する。
- `GITHUB_TOKEN` による GitHub Search API fallback を削除する。
- seed fallback を通常実行から外し、テスト用の候補準備に限定する。
- README、`.env.example`、AGENTS の説明を新しい接続方針に合わせる。

## 非目的

- GitHub OAuth の認可フロー自体は変えない。
- GitHub への書き込み操作は追加しない。
- OAuth scope は現状の `read:user` から広げない。
- 複数ユーザーや本番認証基盤の追加は行わない。

## 採用方針

通常実行の接続と候補補充は保存済み OAuth token だけに依存する。ローカル開発でも `GITHUB_CLIENT_ID` と `GITHUB_CLIENT_SECRET` を設定し、GitHub OAuth で接続してからリールを開始する。

`dev-connect` は削除する。OAuth 未設定時は接続済みになれないため、Web では OAuth 設定が必要であることを表示する。サーバーは `/api/auth/dev-connect` を公開しない。

`GITHUB_TOKEN` は設定項目から削除する。`Config` と `AppState` から環境変数 token 用の `GitHubClient` を取り除き、`DiscoveryService` は保存済み OAuth token がある場合だけ GitHub Search API を呼ぶ。

seed は通常の `ensure_candidates()` の最終 fallback としては使わない。サーバーテストや E2E が固定候補を必要とする場合は、テスト用の構築経路またはテストヘルパーで seed 相当の候補を準備する。

## API とサーバー

`routes/auth.rs` では `/dev-connect` route、`DevConnectRequest`、`dev_connect`、関連テストを削除する。`/state` は引き続き `connected`、`username`、`oauth_configured`、`oauth_start_url` を返す。OAuth 設定済みで access token が保存されている場合だけ `connected = true` とする。

`config.rs` から `github_token` を削除する。`.env` や `.env.example` でも `GITHUB_TOKEN` は扱わない。`app.rs` から `AppState.github_client` を削除し、環境変数 token 由来の `GitHubClient` を作らない。

`discovery.rs` は OAuth token だけを補充元にする。`store.auth_access_token()` が `Some` の場合は OAuth token で `GitHubClient` を作り、候補が採用できた場合に queue を補充する。token がない、GitHub API が失敗する、採用候補が 0 件の場合は通常実行では候補を追加せず、結果として `/api/reel/current` や `/api/reel/next` は `queue_empty` を返す。

## Web

`web/src/api/client.ts` から `devConnect` を削除する。

`ReelScreen` の未接続表示は GitHub OAuth 接続を前提にする。`auth.oauth_configured` が true の場合は「GitHubに接続」ボタンで `auth.oauth_start_url` へ遷移する。false の場合は、OAuth 設定が必要であることを表示し、接続ボタンは出さない。

UI 表示文言は日本語にする。GitHub 由来のリポジトリ名や README preview は元の言語を保つ。

## テスト

サーバー単体テストでは、`dev-connect` 前提のテストを削除または OAuth token 保存済み状態へ置き換える。Discovery のテストは OAuth token client の優先挙動を残し、`GITHUB_TOKEN` fallback と通常 seed fallback の期待値を削除する。

リール操作の API テストは、`auth_state` に `connected = 1` と `access_token` を入れた状態で実行する。候補が必要なテストは `DiscoveryService::enqueue_candidates()` で明示的に候補を入れるか、テスト用 helper で seed 相当を投入する。

Web テストは、OAuth 未設定時に開発用接続を表示しないこと、OAuth 設定済み時に GitHub OAuth 開始 URL へ遷移することを確認する。

E2E は dev-connect をクリックするフローから外す。OAuth の外部連携を E2E で通さない場合は、テスト起動時に DB へ接続状態と候補を準備するか、E2E 対象を未接続表示の確認に縮小する。OAuth 実連携は手動確認に残す。

## ドキュメント

README は、ローカル開発でも GitHub OAuth App の作成と `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET` 設定が必要であることを説明する。`GITHUB_TOKEN`、開発用接続、通常 seed fallback の説明を削除する。

`.env.example` は OAuth 関連と公開 URL だけを残す。AGENTS の実行時注意と構成メモも、候補補充の優先順位を「保存済み OAuth token のみ」に更新する。

## エラーと空状態

OAuth 未設定時は未接続状態として扱い、Web で設定不足を案内する。OAuth token がない場合や GitHub API が失敗した場合は seed に落とさず、候補なしとして扱う。GitHub API の一時失敗はログに残すが、疑似候補で成功したようには見せない。

## 成功条件

- `/api/auth/dev-connect` が存在しない。
- `GITHUB_TOKEN` がコード、設定例、README から削除されている。
- 通常実行の Discovery が seed fallback しない。
- OAuth 接続済み token がある場合だけ GitHub Search API から候補補充する。
- テストは dev-connect や `GITHUB_TOKEN` に依存せず通る。
