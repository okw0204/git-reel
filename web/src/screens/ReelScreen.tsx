import { Bookmark, ChevronLeft, ChevronRight, Info, SkipForward, UserCheck } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { api } from "../api/client";
import { DetailDrawer } from "../components/DetailDrawer";
import { RepoCard } from "../components/RepoCard";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import type { AuthState, ReelResponse, Repository, RepositoryDetail } from "../types";

type ReelScreenProps = {
  auth: AuthState;
};

export function ReelScreen({ auth }: ReelScreenProps) {
  const [repository, setRepository] = useState<Repository | null>(null);
  const [emptyReason, setEmptyReason] = useState<string | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detail, setDetail] = useState<RepositoryDetail | null>(null);
  const [memo, setMemo] = useState("");
  const [tagInput, setTagInput] = useState("");
  const [message, setMessage] = useState("");

  // API の空状態と候補を同時に反映し、認証待ち・候補なしの表示分岐を一箇所に寄せる。
  const applyReel = (payload: ReelResponse) => {
    setRepository(payload.repository);
    setEmptyReason(payload.empty_reason ?? null);
  };

  const loadCurrent = useCallback(async () => {
    applyReel(await api.current());
  }, []);

  useEffect(() => {
    void loadCurrent();
  }, [loadCurrent]);

  const connect = async () => {
    if (!auth.oauth_configured) return;
    window.location.href = auth.oauth_start_url ?? "/api/auth/github/start";
  };

  const next = useCallback(async () => {
    if (!auth.connected) return;
    applyReel(await api.next());
    setDetailOpen(false);
  }, [auth.connected]);

  const previous = useCallback(async () => {
    if (!auth.connected) return;
    applyReel(await api.previous());
    setDetailOpen(false);
  }, [auth.connected]);

  const save = useCallback(async () => {
    if (!repository) return;
    await api.save(repository.id);
    setMessage(`${repository.full_name} を保存しました`);
  }, [repository]);

  const skip = useCallback(async () => {
    if (!repository) return;
    await api.skip(repository.id);
    applyReel(await api.next());
    setDetailOpen(false);
  }, [repository]);

  const toggleDetail = useCallback(async () => {
    // ドロワーを開く瞬間だけ詳細を取得し、閉じるだけの操作では余計な API 呼び出しをしない。
    if (!repository) return;
    if (!detailOpen) {
      const payload = await api.detail(repository.id);
      setDetail(payload);
      setMemo(payload.memo);
      setTagInput(payload.tags.join(", "));
    }
    setDetailOpen((value) => !value);
  }, [detailOpen, repository]);

  const saveMemo = async () => {
    if (!repository) return;
    await api.note(repository.id, memo);
    setMessage("メモを保存しました");
  };

  const saveTags = async () => {
    // 入力はカンマ区切りに限定し、永続化前に空要素を取り除く。
    if (!repository) return;
    const tags = tagInput.split(",").map((tag) => tag.trim()).filter(Boolean);
    await api.tags(repository.id, tags);
    setMessage("タグを保存しました");
  };

  useKeyboardShortcuts({
    onNext: next,
    onPrevious: previous,
    onSave: save,
    onSkip: skip,
    onDetail: toggleDetail,
    enabled: auth.connected
  });

  if (!auth.connected || emptyReason === "auth_required") {
    return (
      <section className="center-panel">
        <p className="eyebrow">Local-first discovery</p>
        <h1>GitHubに接続するとリールを開始できます</h1>
        <p>
          {auth.oauth_configured
            ? "GitHub OAuth で接続すると、保存済み OAuth token を使って実リポジトリ候補を取得します。"
            : "リールを開始するには GitHub OAuth の設定が必要です。GITHUB_CLIENT_ID と GITHUB_CLIENT_SECRET を設定してサーバーを起動してください。"}
        </p>
        {auth.oauth_configured ? (
          <button className="primary-button" onClick={connect} type="button">
            <UserCheck aria-hidden="true" size={18} />
            GitHubに接続
          </button>
        ) : null}
      </section>
    );
  }

  return (
    <section className="reel-layout">
      <div className="reel-main">
        {repository ? <RepoCard repository={repository} /> : <div className="empty-state">候補が空です</div>}
        <div className="action-bar" aria-label="リール操作">
          <button onClick={previous} type="button"><ChevronLeft aria-hidden="true" size={18} />前へ</button>
          <button onClick={save} type="button"><Bookmark aria-hidden="true" size={18} />保存</button>
          <button onClick={skip} type="button"><SkipForward aria-hidden="true" size={18} />スキップ</button>
          <button onClick={toggleDetail} type="button"><Info aria-hidden="true" size={18} />詳細</button>
          <button onClick={next} type="button"><ChevronRight aria-hidden="true" size={18} />次へ</button>
        </div>
        {message ? <p className="toast" role="status">{message}</p> : null}
      </div>
      <DetailDrawer
        detail={detail}
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        memo={memo}
        tagInput={tagInput}
        onMemoChange={setMemo}
        onTagInputChange={setTagInput}
        onSaveMemo={saveMemo}
        onSaveTags={saveTags}
      />
    </section>
  );
}
