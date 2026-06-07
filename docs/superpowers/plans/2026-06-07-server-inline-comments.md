# Server Inline Comments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** サーバー側のコードリーディングを助けるため、状態遷移・外部境界・フォールバック理由に限定してインラインコメントを少し厚くする。

**Architecture:** 既存の実装や公開 API は変更しない。コメントは「何をしているか」の逐語説明ではなく、「なぜこの順序・境界・分岐にしているか」を短く補足する。対象は `server/src/repositories.rs`, `server/src/routes/auth.rs`, `server/src/routes/reel.rs`, `server/src/discovery.rs`, `server/src/github.rs` に限定する。

**Tech Stack:** Rust, axum, sqlx, reqwest, tokio。動作変更なしのコメント追加のため、検証は `cargo test --manifest-path server/Cargo.toml` で既存テストの非回帰を確認する。

---

## File Structure

- Modify: `server/src/repositories.rs`
  - DB キュー、履歴イベント、保存・メモ・タグの状態遷移にコメントを足す。
- Modify: `server/src/routes/auth.rs`
  - OAuth state/cookie 検証、token 保存、接続状態判定の意図にコメントを足す。
- Modify: `server/src/routes/reel.rs`
  - route 層が決めるユーザー操作の意味と store 層へ寄せる責務を明確にする。
- Modify: `server/src/discovery.rs`
  - 既存コメントを活かし、補充元の優先順位と seed fallback の意図だけ必要なら補う。
- Modify: `server/src/github.rs`
  - Search API と README preview の境界、外部 API レスポンスを内部型へ寄せる理由を必要なら補う。

### Task 1: Repository Store コメント追加

**Files:**
- Modify: `server/src/repositories.rs`

- [ ] **Step 1: 既存コメント密度を確認する**

Read: `server/src/repositories.rs`

確認ポイント:
- `auth_access_token`, `upsert_repository`, `history`, `previous_reel_repository`, `create_discovery_batch`, `enqueue_repository`, `claim_next_queued_repository`, `save_repository`, `set_note`, `replace_tags`, `saved` に既存コメントがある。
- 追加コメントは、既存コメントを言い換えず、読解時に状態遷移の意味が残りにくい箇所だけに足す。

- [ ] **Step 2: 状態遷移のコメントを追加する**

Edit: `server/src/repositories.rs`

追加方針:
```rust
// current/next の表示制御と履歴除外で同じ判定を使うため、イベント有無だけを返す。
```

候補箇所:
- `has_prior_interaction`
- `consume_repository`
- `find_by_normalized_name`

避けるコメント:
```rust
// repository_id で検索する。
// SQL を実行する。
```

- [ ] **Step 3: 差分を確認する**

Run: `git diff -- server/src/repositories.rs`

Expected:
- Rust コードの動作差分がない。
- 追加行は `//` コメントのみ。
- コメントは 1 箇所 1〜2 行以内。

### Task 2: OAuth Route コメント追加

**Files:**
- Modify: `server/src/routes/auth.rs`

- [ ] **Step 1: OAuth の読解ポイントを確認する**

Read: `server/src/routes/auth.rs`

確認ポイント:
- `github_start` は state を DB と cookie に保存する。
- `github_callback` は GitHub から返った state、cookie、DB レコードを照合する。
- `auth_state` は OAuth 設定済みの場合、`connected = 1` だけでなく token の存在も要求する。
- `dev_connect` は OAuth 未設定の開発用経路に限定する。

- [ ] **Step 2: セキュリティ境界と接続判定のコメントを追加する**

Edit: `server/src/routes/auth.rs`

追加方針:
```rust
// cookie と DB の両方を消費して、別ブラウザや再利用された state を callback として受け付けない。
```

候補箇所:
- `github_callback` の state 検証直前または `delete_oauth_state` 後
- `auth_state` の `connected` 算出直前
- `redirect_to_public_app` の cookie clear 直前

避けるコメント:
```rust
// GitHub へ POST する。
// Header を設定する。
```

- [ ] **Step 3: 差分を確認する**

Run: `git diff -- server/src/routes/auth.rs`

Expected:
- OAuth 処理順序、SQL、HTTP request の変更がない。
- 追加行は `//` コメントのみ。

### Task 3: Reel Route コメント追加

**Files:**
- Modify: `server/src/routes/reel.rs`

- [ ] **Step 1: route 層の責務を確認する**

Read: `server/src/routes/reel.rs`

確認ポイント:
- `current` は先頭候補のプレビューで、キューを消費しない。
- `next` はユーザーが見た操作で、キュー消費と viewed 記録を行う。
- `previous` は戻った履歴を returned としてイベント化する。
- `detail` は詳細閲覧を関心シグナルとして残す。

- [ ] **Step 2: route と store の境界コメントを追加する**

Edit: `server/src/routes/reel.rs`

追加方針:
```rust
// 認証前は discovery を走らせず、フロントが接続導線を出せる empty_reason だけを返す。
```

候補箇所:
- `current` と `next` の認証チェック直前
- `skip` の `record_event` と `consume_repository` の間

避けるコメント:
```rust
// JSON を返す。
// save_repository を呼ぶ。
```

- [ ] **Step 3: 差分を確認する**

Run: `git diff -- server/src/routes/reel.rs`

Expected:
- route 定義、handler の戻り値、イベント種別の変更がない。
- 追加行は `//` コメントのみ。

### Task 4: Discovery/GitHub コメントの最小補強

**Files:**
- Modify: `server/src/discovery.rs`
- Modify: `server/src/github.rs`

- [ ] **Step 1: 既存コメントと重複しない箇所を確認する**

Read:
- `server/src/discovery.rs`
- `server/src/github.rs`

確認ポイント:
- どちらも既にコメントが比較的多い。
- 追加は関数間の責務境界が見えにくい箇所に限定する。

- [ ] **Step 2: 必要最小限のコメントだけ追加する**

Edit: `server/src/discovery.rs`, `server/src/github.rs`

追加方針:
```rust
// accepted = 0 は API 成功でも全候補が既読・保存済みだった状態なので、次の補充元を試す。
```

候補箇所:
- `DiscoveryService::ensure_candidates` の `accepted > 0` 判定付近
- `GitHubClient::search_recently_updated_repositories` の README preview 失敗処理付近

避けるコメント:
```rust
// Vec に collect する。
// body を parse する。
```

- [ ] **Step 3: 差分を確認する**

Run: `git diff -- server/src/discovery.rs server/src/github.rs`

Expected:
- 実装行の変更がない。
- コメントが既存コメントと重複していない。

### Task 5: 検証

**Files:**
- Verify only

- [ ] **Step 1: サーバーテストを実行する**

Run: `cargo test --manifest-path server/Cargo.toml`

Expected:
- PASS
- コメント追加のみなので、失敗した場合は今回の編集以外の環境要因か既存失敗かを切り分ける。

- [ ] **Step 2: 最終差分を確認する**

Run: `git diff -- server/src/repositories.rs server/src/routes/auth.rs server/src/routes/reel.rs server/src/discovery.rs server/src/github.rs`

Expected:
- 追加・変更はコメントのみ。
- コメントは日本語。
- 1 箇所あたり 1〜2 行以内。
- 逐語説明ではなく、状態・境界・理由を説明している。

## Self-Review

- Spec coverage: サーバー中心、状態遷移・外部境界・フォールバック理由に限定する方針を各 task に反映した。
- Placeholder scan: 未定義の TBD/TODO/後回し項目は含めていない。
- Type consistency: 動作変更なしのコメント追加なので、型・関数シグネチャ変更は計画に含めていない。
