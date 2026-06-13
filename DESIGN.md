# Git Reel 実装設計

## 目的

Git Reel は、GitHub リポジトリをリール形式で気軽に眺めるための local-first アプリである。検索 UI や GitHub Trending の置き換えではなく、1 件ずつ候補を見て、気になったものをローカルの「保存済み」として残す体験を中心にする。

現在の MVP はローカル開発向けで、GitHub への書き込みは行わない。GitHub OAuth 接続後、保存済み OAuth token を使って GitHub Search API から候補を補充し、閲覧、保存、スキップ、履歴、メモ、タグを SQLite に保存する。

## 非目的

- GitHub Star、Watch、リポジトリ metadata 更新などの GitHub への書き込み操作は扱わない。
- GitHub の完全な検索 UI や Trending clone は作らない。
- 複数ユーザー向けの認証基盤や hosted service 化は扱わない。
- 通常実行で seed 候補に fallback しない。
- E2E で外部 GitHub OAuth の実フローに依存しない。

## 全体構成

```text
web/                 React / Vite のフロントエンド
server/              Rust / Axum のローカル API
server/migrations/   SQLite schema
e2e/                 Playwright E2E
docs/superpowers/    個別の設計 spec と実装 plan
```

フロントエンドは UI、画面遷移、ユーザー操作の起点を担当する。GitHub API 呼び出し、OAuth callback、SQLite 永続化、Discovery queue の管理はサーバーが担当する。

SQLite は local-first な状態の保存先である。保存済み、履歴、メモ、タグ、接続状態、OAuth access token はローカル DB に保存し、GitHub 側へ送信しない。

## アプリケーション境界

- Web から API を呼ぶ処理は `web/src/api/client.ts` に集約する。画面から直接 `fetch` を増やさない。
- サーバーの router 構築は `server/src/app.rs` に集約する。本番用とテスト用の差分は `Config` に閉じ込める。
- route 層は HTTP 入出力とユーザー操作の意味付けを担当し、DB 操作の詳細は `RepositoryStore` に寄せる。
- GitHub API の response は `server/src/github.rs` でアプリ内の `NewRepository` に変換し、DB 層を外部 API の形に依存させない。
- Discovery の補充判断は `DiscoveryService` に寄せる。route から直接 GitHub Search API を呼ばない。

## 主要ユーザーフロー

### 初期表示と接続状態

Web は起動時に `/api/auth/state` を呼び、ローカルに保存された GitHub 接続状態を確認する。

OAuth が未設定、または access token が保存されていない場合、リール画面は未接続状態を表示する。OAuth 設定済みの場合は GitHub OAuth 開始 URL へ遷移できる。OAuth 未設定の場合は設定が必要であることを表示し、疑似的な接続手段は出さない。

### GitHub OAuth

OAuth 開始時、サーバーは state を生成し、`oauth_states` table と HttpOnly cookie の両方に保存する。callback では GitHub から返った state、cookie の state、DB の state を照合し、再利用や別ブラウザからの callback を受け付けない。

callback が成功すると、サーバーは GitHub token endpoint で access token を取得し、GitHub user endpoint から username を取得する。取得した token と username は `auth_state` に保存する。OAuth scope は `read:user` を基本にし、GitHub への書き込み権限は要求しない。

### リール表示

`/api/reel/current` は現在の先頭候補を返す preview 用 endpoint で、queue を消費しない。未接続時は `empty_reason = "auth_required"` を返す。

`/api/reel/next` はユーザーが候補を見た操作として扱い、queue の先頭を消費し、`viewed` event を記録する。候補がなければ `DiscoveryService::ensure_candidates` が補充を試す。

`/api/reel/previous` は直近の viewed/returned event を現在地として、一つ前の viewed repository を返す。戻った操作も `returned` event として記録する。

### 保存、スキップ、詳細

保存は `saved_repositories` に冪等に記録し、ユーザーが保存を選んだ事実は `saved` event として残す。

スキップは `skipped` event を記録し、未消費の queue 行を `consumed_at` で閉じる。これにより、スキップ済み候補が次回の先頭候補として残らない。

詳細表示は `detail_opened` event を記録し、メモ、タグ、README preview を返す。README preview は補助情報であり、取得失敗しても repository card 全体の表示失敗にはしない。

### 保存済み、履歴、設定

保存済み画面は `saved_repositories` を起点に、repository、memo、tags を表示する。検索は保存済み一覧に対して行う。

履歴画面は `repo_events` の最新 event を repository ごとに集約し、重複表示を避ける。

設定画面は GitHub 接続状態、現在の discovery mix、DB 接続先など、ローカル状態の概要を表示する。

## API 設計

Web が使う API は `web/src/api/client.ts` に集約する。

主な endpoint は次の通り。

- `GET /api/health`: ローカル API の生存確認。
- `GET /api/auth/state`: 接続状態、username、OAuth 設定有無、OAuth 開始 URL を返す。
- `GET /api/auth/github/start`: GitHub OAuth 認可画面へ redirect する。
- `GET /api/auth/github/callback`: GitHub OAuth callback を処理する。
- `GET /api/reel/current`: queue の先頭候補を消費せず返す。
- `POST /api/reel/next`: 次候補を消費して `viewed` event を記録する。
- `POST /api/reel/previous`: 前に見た候補へ戻り `returned` event を記録する。
- `POST /api/reel/:id/save`: repository を保存し `saved` event を記録する。
- `POST /api/reel/:id/skip`: repository をスキップし `skipped` event を記録する。
- `GET /api/reel/:id/detail`: 詳細表示に必要なローカル情報を返し `detail_opened` event を記録する。
- `GET /api/saved`: 保存済み repository を返す。
- `PATCH /api/saved/:id/note`: repository memo を更新する。
- `PUT /api/saved/:id/tags`: repository tags を置き換える。
- `GET /api/history`: repository ごとの最新 event を履歴として返す。
- `GET /api/settings`: 接続状態や DB などの概要を返す。

エラー時は HTTP status を使い、Web の API client は non-2xx を例外として扱う。画面側は操作単位で retry や空状態を表示する。

## データモデル

SQLite の主要 table は次の役割を持つ。

- `repositories`: GitHub repository の identity と表示用 metadata。`github_id` と `normalized_full_name` で重複を抑える。
- `repo_events`: ユーザー操作を append-only に記録する。履歴、前へ戻る操作、再投入防止の根拠にする。
- `saved_repositories`: ローカルの「保存済み」状態。GitHub Star とは別物として扱う。
- `repo_notes`: repository ごとの memo。
- `tags`: ユーザー定義 tag。
- `repo_tags`: repository と tag の many-to-many 関係。
- `discovery_batches`: 候補補充の strategy、query、候補数、採用数の監査ログ。
- `discovery_queue`: リール表示待ち候補の queue。`position` で表示順を管理し、`consumed_at` で消費済みを表す。
- `auth_state`: GitHub OAuth 接続状態、username、access token。
- `oauth_states`: OAuth callback 検証用の一時 state。

repository は GitHub 由来の情報を元の言語のまま保持する。UI の固定文言は日本語を基本にする。

## Discovery 設計

Discovery は `DiscoveryService::ensure_candidates` が担当する。

候補が queue に残っている場合は補充しない。queue が空の場合だけ、`auth_state` に保存された OAuth access token を使って GitHub Search API から候補を取得する。

通常実行では、OAuth token がない場合、GitHub API が失敗した場合、採用候補が 0 件の場合でも seed 候補へ fallback しない。その場合は queue を空のままにし、reel API は `empty_reason = "queue_empty"` を返す。

GitHub Search API では、現在は最近更新された repository を対象にする。Search API の結果は `NewRepository` に変換し、README preview は GraphQL で補助的に取得する。README preview は timeout を短くし、候補補充全体を詰まらせない。

候補採用時は、DB へ upsert した後の repository id で重複を判定する。既に viewed、saved、skipped、returned、detail_opened などの prior interaction がある repository は通常の queue に戻さない。

seed 候補はテスト用の候補準備に限定する。

## エラーと空状態

- OAuth 未接続: `auth_required` を返し、Web は GitHub 接続が必要であることを表示する。
- OAuth 未設定: Web はローカル OAuth App 設定が必要であることを表示する。
- queue 空: `queue_empty` を返し、Web は候補がない状態として表示する。
- GitHub API 失敗: サーバーは warn log を残し、通常実行では seed fallback しない。
- README preview 取得失敗: preview なしの候補として扱い、候補全体は失敗にしない。
- DB 更新失敗: 操作成功として扱わず、HTTP error として返す。

空状態や固定 UI 文言は日本語で短く具体的に書く。GitHub 由来の repository 名、説明、README、topic は元の言語を保つ。

## テスト方針

テストや起動方法は `Makefile`、`package.json`、`web/package.json`、`server/Cargo.toml` を確認して選ぶ。

サーバーテストでは、route、store、Discovery、GitHub response parsing、OAuth state 検証を対象にする。実 GitHub API には依存せず、fixture や差し替え可能な client factory を使う。

Web テストでは、主要画面、API client 経由の操作、未接続や OAuth 未設定の表示を確認する。

E2E は外部 GitHub OAuth に依存しない範囲を検証する。候補が必要な場合は、テスト用 DB や helper で接続状態と候補を準備する。

代表的な確認コマンドは次の通り。

```bash
npm test
npm run test:web
npm run test:server
npm run test:e2e
make test
make build
```

## 変更時の判断基準

- 画面から直接 `fetch` を追加せず、`web/src/api/client.ts` に API 呼び出しを追加する。
- route 追加や router 構築変更は `server/src/app.rs` の集約方針を崩さない。
- 本番用とテスト用の差分は `Config` に閉じ込める。
- 通常実行の Discovery は保存済み OAuth token を使う。`GITHUB_TOKEN` や seed fallback を通常経路に戻さない。
- GitHub への書き込み操作を追加する場合は、OAuth scope、UI 表示、失敗時の扱いを全体設計として見直す。
- DB schema を変える場合は migration、store、API response、テストデータを同じ変更範囲で見直す。
- ユーザー操作として意味があるものは、必要に応じて `repo_events` に残し、履歴や再投入防止との関係を確認する。
- UI 固定文言は日本語、GitHub 由来の内容は元の言語を保つ。
