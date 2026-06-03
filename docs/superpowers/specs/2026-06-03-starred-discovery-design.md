# Starred repositories 起点の Discovery 設計

## 背景

Git Reel は OAuth 接続済みの場合、保存済み OAuth token を使って GitHub Search API から候補を補充できる。現状の OAuth Discovery は最近更新されたリポジトリを広く検索するため、ユーザー本人の関心を十分に反映しない。

この変更では、OAuth 接続済みユーザーの starred repositories を読み取り、language と topic の傾向から Search query を作る。完全一致の topic だけでなく、小さな手書き近傍マッピングも使い、近いけれど少し広がりのある候補を discovery queue に投入する。

## 目的

- OAuth 接続済みの場合、starred repositories をもとにした候補補充を試みる。
- starred repositories の language と topic を集計して Search query を生成する。
- 手書き近傍マッピングにより、完全一致だけでない探索対象を混ぜる。
- 生成した候補を既存の discovery queue に投入する。
- discovery batch に starred 起点であることが分かる strategy と query を残す。
- starred 取得や Search が失敗した場合でも、`GITHUB_TOKEN` または seed 候補へフォールバックする。
- 通常のテストは live GitHub API に依存させない。
- GitHub への書き込み操作は追加しない。

## 対象外

- 複雑な推薦アルゴリズム。
- README や description のキーワード抽出。
- LLM による分類。
- ユーザーが編集できる関心プロファイル。
- 候補ごとの詳しい「出てきた理由」表示。
- GitHub Star の書き込みや同期。

## 採用方針

OAuth token がある場合は、既存の `recently_updated_oauth_search` を starred 起点 Discovery に置き換える。つまり OAuth 接続済みの候補補充では、starred repositories を取得し、その傾向から生成した Search query で候補を探す。`recently_updated_oauth_search` という OAuth token 用の戦略名と呼び出し経路は削除する。

starred 起点 Discovery が失敗した場合や採用候補が 0 件の場合は、既存通り `GITHUB_TOKEN` 由来の fallback client を試し、最後に seed 候補へ落とす。OAuth token がない場合も現在と同じく `GITHUB_TOKEN`、seed の順に補充する。

この方針により、OAuth 接続の価値を「ユーザー本人の興味から候補が出る」体験に寄せられる。一方で、GitHub API 失敗時のリール体験は既存 fallback により維持できる。

## アーキテクチャ

主な責務分担は次の通り。

- `DiscoveryService`: queue が空のときの補充元の優先順位を制御する。
- `GitHubDiscoveryClient`: recently updated Search と starred 起点 Discovery の API 境界を表す。
- `GitHubClient`: starred repositories の取得、傾向集計、Search query 生成、Search API 実行、README preview 補完を担当する。
- `RepositoryStore`: 既存通り OAuth token 取得、repository upsert、batch 作成、queue 投入を担当する。

`GitHubDiscoveryClient` には starred 起点 Discovery 用のメソッドを追加する。`DiscoveryService::ensure_candidates()` は OAuth token がある場合、このメソッドを `starred_oauth_search` 戦略として呼ぶ。既存の `search_recently_updated_repositories()` は `GITHUB_TOKEN` fallback 用に残すが、OAuth token 用には使わない。

## Starred 起点 Query 生成

`GitHubClient` は OAuth token で `/user/starred` を取得する。初回スコープでは取得件数を小さく保ち、たとえば `per_page=50` の 1 ページだけを見る。

取得した starred repositories から次を集計する。

- `language`: `language:<name>` の候補として使う。
- `topics`: `topic:<name>` の候補として使う。

集計後、出現回数の多い language と topic を上位から少数選ぶ。選ばれた topic や language に対して、手書き近傍マッピングを適用する。

例:

- `react`: `vite`, `frontend`, `typescript`
- `rust`: `cli`, `wasm`, `systems-programming`
- `typescript`: `frontend`, `nodejs`, `web`
- `python`: `machine-learning`, `data-science`, `automation`
- `cli`: `terminal`, `developer-tools`, `rust`

生成する Search query は、既存の recently updated query と同じ安全条件を引き継ぐ。

```text
stars:10..5000 fork:false archived:false pushed:>YYYY-MM-DD (language:Rust OR topic:rust OR topic:cli OR topic:wasm) sort:updated-desc
```

GitHub Search API の query 長や構文の扱いを安定させるため、初回は OR 条件の数を少数に制限する。starred repositories 自体を候補として返すのではなく、starred の傾向から隣接リポジトリを探索する。

## データフロー

OAuth 接続済みのリール補充は次の流れになる。

1. ユーザーが GitHub OAuth で接続する。
2. OAuth callback が `auth_state.access_token` を保存する。
3. ユーザーがリールを開く、または「次へ」を押す。
4. `DiscoveryService::ensure_candidates()` が queue の空きを確認する。
5. queue が空なら `RepositoryStore::auth_access_token()` で OAuth token を読む。
6. token があれば、OAuth token 由来の `GitHubClient` で starred 起点 Discovery を実行する。
7. `GitHubClient` が `/user/starred` を取得し、language/topic と近傍マッピングから Search query を作る。
8. Search API で候補を取得し、既存の README preview 補完を行う。
9. `DiscoveryService::enqueue_candidates()` が `starred_oauth_search` と生成 query を discovery batch に保存し、採用候補を queue に入れる。
10. 採用候補が 0 件または API 失敗の場合は `GITHUB_TOKEN` fallback、最後に seed を試す。

補充元の優先順位は次の通り。

1. `auth_state.access_token` による starred 起点 Discovery
2. `GITHUB_TOKEN` 由来の recently updated Search
3. ローカル seed

`recently_updated_oauth_search` はこの優先順位から外し、実装からも削除する。recently updated Search の実装自体は `recently_updated_live_search` の fallback 用として維持する。

## エラー処理

リール体験を止めない方針にする。

- OAuth token がない場合: `GITHUB_TOKEN` client を試し、なければ seed にフォールバックする。
- starred repositories の取得に失敗した場合: warning を出し、`GITHUB_TOKEN` client または seed にフォールバックする。
- starred repositories が空の場合: 採用候補 0 件として扱い、fallback する。
- starred 由来 Search query が作れない場合: 採用候補 0 件として扱い、fallback する。
- starred 由来 Search API が失敗した場合: warning を出し、fallback する。
- README GraphQL が失敗した場合: 既存通りその候補だけ `readme_preview = None` として続行する。
- DB エラーの場合: 既存通り API エラーとして返す。

OAuth token が失効している可能性がある場合でも、この変更では接続状態の自動解除は行わない。再接続導線や rate limit 表示は後続の改善対象とする。

## フロントエンドと README

フロントエンドの API 契約は変更しない。候補カードにも「なぜ出てきたか」は表示しない。

README は、OAuth 接続後の候補補充が単なる最近更新 Search ではなく、starred repositories の language/topic 傾向を使うことを説明する。GitHub への書き込みを行わないこと、取得失敗時は `GITHUB_TOKEN` または seed にフォールバックすることも既存説明と矛盾しないように更新する。

## テスト方針

通常のテストは live GitHub API に依存させない。

サーバーテストでは次を確認する。

- starred repositories の fixture から language/topic を集計できる。
- 手書き近傍マッピングにより、完全一致以外の topic が Search query に含まれる。
- OAuth token がある場合、Discovery は starred 起点メソッドを使う。
- OAuth token がある場合、既存の recently updated OAuth Search は呼ばれない。
- starred 起点 Discovery が失敗した場合、`GITHUB_TOKEN` client または seed にフォールバックする。
- discovery batch に `starred_oauth_search` と生成 query が保存される。
- Search API fixture から候補を作れる。

フロントエンドテストは API 契約を変えないため原則追加しない。README 変更は文言が実挙動と矛盾しないことを確認する。

## 実装上の注意

最小変更を優先する。関心プロファイルを DB に保存したり、ユーザー設定 UI を増やしたりしない。starred 起点の傾向計算は `GitHubClient` または近い純粋関数に閉じ込め、`DiscoveryService` は補充元の優先順位だけを見る。

`GitHubDiscoveryClient` の fake 実装で starred 起点メソッドの成功・失敗を制御できるようにし、テストが GitHub API に依存しない状態を維持する。
