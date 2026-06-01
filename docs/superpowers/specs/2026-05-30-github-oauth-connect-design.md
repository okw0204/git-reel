# GitHub OAuth 接続設計

## 背景

Git Reel は現在、開発用の疑似接続で `auth_state` を接続済みにしている。リール画面や API にはすでに「接続済みでなければ候補を返さない」境界があるため、今回の PR ではこの接続状態を GitHub OAuth に置き換える。

この PR では GitHub API から実際のリポジトリ候補を取得するところまでは扱わない。OAuth で GitHub ユーザーを確認し、後続 PR で使えるアクセストークンを保存するところまでをスコープにする。

## 目的

- GitHub OAuth で Git Reel に接続できるようにする。
- GitHub のユーザー名を取得し、画面上の接続状態に反映する。
- GitHub API 呼び出しに使えるアクセストークンを `auth_state` に保存する。
- 既存のローカル優先なリール体験とシード候補の挙動は維持する。

## 対象外

- GitHub Search API や GraphQL API から実データのリポジトリ候補を取得する。
- Star、Watch、Issue 作成など GitHub への書き込み操作を追加する。
- 複数ユーザー、セッション管理、Cookie 認証を本格導入する。
- リフレッシュトークンやトークン期限更新の仕組みを作る。
- Tauri の deep link 対応を入れる。

## 方針

推奨方針は「OAuth 接続のみを 1 つの PR で実装する」。既存の `auth_state` テーブルには `username` と `access_token` があるため、DB スキーマ変更は最小限にできる。リール候補は現行どおりローカルシードを使い、OAuth の成功可否と接続状態の置き換えに集中する。

代替案として OAuth と GitHub 実データ取得を同時に入れる案もあるが、発見戦略、API 利用制限、空結果、API 失敗、重複排除まで同時に設計する必要があり、1 つの PR として大きくなりやすい。そのため今回のスコープから外す。

## サーバー設計

`Config` に GitHub OAuth 用の環境変数を追加する。

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GIT_REEL_PUBLIC_BASE_URL`

`GIT_REEL_PUBLIC_BASE_URL` はコールバック URL 構築に使う。ローカル開発では `http://127.0.0.1:4317` を想定する。

`/api/auth` に GitHub OAuth 用ルートを追加する。

- `GET /api/auth/github/start`
- `GET /api/auth/github/callback`

`start` は GitHub の認可 URL にリダイレクトする。権限範囲は最小限にし、公開情報のユーザー確認だけで足りるため、`read:user` のみに留める。

`callback` は GitHub から返された `code` を受け取り、GitHub のトークンエンドポイントに送ってアクセストークンに交換する。その後 GitHub `/user` API を呼び、`login` を取得する。取得できたら `auth_state` の id 1 を upsert し、`connected = 1`, `username`, `access_token`, `updated_at` を更新する。

OAuth 設定が不足している場合は、サーバー起動自体は失敗させず、OAuth 開始時に設定不足としてエラーを返す。これにより、既存のローカル開発やテストが OAuth 設定なしでも動く。

## フロントエンド設計

リール画面の未接続状態にあるボタンを、開発用接続から GitHub OAuth 開始へ差し替える。

- 表示文言は「GitHubに接続」にする。
- クリック時は `window.location.href = "/api/auth/github/start"` でサーバーの OAuth 開始ルートへ遷移する。
- コールバック後はリール画面に戻り、既存の `authState()` 呼び出しで接続状態を反映する。

`dev-connect` はテストやローカル補助として当面残す。UI からは基本的に使わないが、既存テストを一度に壊さないため、削除や整理は別 PR に分ける。

## データの流れ

1. ユーザーが「GitHubに接続」を押す。
2. Web は `/api/auth/github/start` に遷移する。
3. サーバーは GitHub 認可 URL にリダイレクトする。
4. ユーザーが GitHub で承認する。
5. GitHub が `/api/auth/github/callback?code=...` に戻す。
6. サーバーは `code` をアクセストークンに交換する。
7. サーバーはアクセストークンで GitHub `/user` を呼び `login` を取得する。
8. サーバーは `auth_state` を接続済みに更新する。
9. サーバーは Web のリール画面へリダイレクトする。
10. Web は起動時の `authState()` でユーザー名を表示し、リールを開始できる。

## エラー処理

- OAuth 設定不足の場合は `start` で 500 系の API エラーにする。
- GitHub が `error` を返した場合は接続状態を更新せず、リール画面へリダイレクトする。
- トークン交換に失敗した場合は接続状態を更新しない。
- GitHub `/user` 取得に失敗した場合も接続状態を更新しない。

初回実装ではエラー専用画面は作らない。設定不足や GitHub API 失敗は API エラーとして扱い、GitHub からユーザー操作でキャンセルされた場合だけリール画面へ戻す。ユーザー向けの詳細なエラー表示は後続 PR で改善できる。

## テスト方針

- サーバーテストで `auth_state` の未接続状態を維持する。
- OAuth コールバックの成功経路は、GitHub API 呼び出し部分を小さく切り出してテスト可能にする。
- OAuth 設定不足時に `start` が失敗することをテストする。
- Web テストは未接続時のボタン文言が「GitHubに接続」になることを確認する。
- E2E は既存の開発用接続フローに依存しているため、この PR では必要に応じて `dev-connect` を使うテスト補助を残す。

## 1 つの PR の境界

この PR の完了条件は、GitHub OAuth で接続し、`auth_state` にユーザー名とアクセストークンを保存できること。保存したトークンを使った実データ取得は次の PR で扱う。
