# 実 GitHub Discovery 設計

## 概要

この変更では、Git Reel に最小限の実 GitHub Discovery 経路を追加する。既存のローカルファーストなリール体験と開発用 seed の挙動は維持しつつ、`GITHUB_TOKEN` が設定されていて discovery queue が空の場合に、サーバーが GitHub Search API からリポジトリ候補を取得する。

目的は、GitHub OAuth、バックグラウンドジョブ、GraphQL による詳細補完をまだ導入せず、実候補の品質だけを検証できる状態にすること。

## 目的

- `GITHUB_TOKEN` がある場合に、GitHub Search API から実リポジトリ候補を取得する。
- トークンなしのローカル開発や GitHub API 失敗時は、既存の開発用 seed へのフォールバックを維持する。
- 既存の discovery queue、batch 記録、重複排除、既存操作済みリポジトリの除外を再利用する。
- 通常のテストは live GitHub API に依存させない。
- 最初の実 Discovery では、フロントエンドとの API 契約を変更しない。

## 非目的

- GitHub OAuth は実装しない。
- GitHub への書き込みは行わない。
- バックグラウンド discovery job や retry scheduling は追加しない。
- 候補取得時に GraphQL で README preview を取得しない。
- この段階では詳細な rate limit UI を追加しない。

## アーキテクチャ

`Config.github_token` をアプリケーション状態から参照できるようにする。既存の開発用接続ゲートは変更しない。ユーザーは引き続き現在の dev-connect flow でリールを開始し、OAuth は導入しない。そのゲートを通過した後、reel route は `current` または `next` を返す前に、`DiscoveryService` へ候補確保を依頼する。

`github.rs` に小さな `GitHubClient` を追加し、実際の Search API アクセスを担当させる。`GitHubClient` は検索リクエストを組み立て、設定された token で送信し、既存の `parse_search_response` 境界を通して `NewRepository` に変換する。

`DiscoveryService` は repository store と任意の GitHub client を受け取る。queue が空の場合、GitHub client があればまず実 GitHub discovery を試す。成功した候補は既存の `enqueue_candidates` に流す。token がない、API 呼び出しに失敗する、parse に失敗する、または採用候補が 0 件の場合は、既存の開発用 seed 候補へフォールバックする。

これにより、GitHub 固有の HTTP 処理を route から切り離し、queue の挙動は `DiscoveryService` に集約したままにする。

## Discovery クエリ

最初の実クエリは、意図的に単純な 1 戦略だけにする。

```text
stars:10..5000 fork:false archived:false pushed:>YYYY-MM-DD sort:updated-desc
```

`YYYY-MM-DD` はリクエスト時点からおおよそ 90 日前として計算する。これにより、空の試作プロジェクトと有名すぎるリポジトリの両方を避けつつ、最近メンテナンスされているリポジトリを優先する。クエリ文字列は `discovery_batches.query` に記録し、strategy は `recently_updated_live_search` として記録する。

## エラー処理

- `GITHUB_TOKEN` がない場合: 実 discovery をスキップし、開発用 seed 候補を使う。
- GitHub HTTP エラー: 失敗をログに残し、開発用 seed 候補を使う。
- Rate limit response: この段階では GitHub HTTP エラーと同じ扱いにする。Saved と History は引き続き利用できる。
- JSON parse error: GitHub discovery failure として扱い、開発用 seed 候補を使う。
- 取得件数または採用件数が 0 件: 開発用 seed 候補を使う。

reel API は既存の `auth_required` と `queue_empty` のレスポンス形状を維持する。この段階ではフロントエンド API 契約の変更は不要。

## テスト

通常のテストは実 GitHub に依存させない。

Rust テストでは次を確認する。

- 既存の Search API fixture 変換が引き続き動作する。
- クエリ構築が意図した実 Search API 用の形を生成する。
- token がない場合は開発用 seed 候補にフォールバックする。
- テスト用 GitHub client が返した実候補が `enqueue_candidates` 経由で queue に入る。
- GitHub discovery failure 時は開発用 seed 候補にフォールバックする。

バックエンドのレスポンス形状を変更しない設計なので、フロントエンドテストは基本的に変更不要。

## 今後の作業

この段階が動作した後に、複数クエリ戦略、rate limit の可視化、任意の詳細補完で discovery quality を改善する。GitHub OAuth は、実候補 discovery が十分に有用だと確認できてから導入する。
