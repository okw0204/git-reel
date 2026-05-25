import { X } from "lucide-react";
import type { RepositoryDetail } from "../types";

type DetailDrawerProps = {
  detail: RepositoryDetail | null;
  open: boolean;
  onClose: () => void;
  memo: string;
  tagInput: string;
  onMemoChange: (value: string) => void;
  onTagInputChange: (value: string) => void;
  onSaveMemo: () => void;
  onSaveTags: () => void;
};

export function DetailDrawer({
  detail,
  open,
  onClose,
  memo,
  tagInput,
  onMemoChange,
  onTagInputChange,
  onSaveMemo,
  onSaveTags
}: DetailDrawerProps) {
  if (!open) return null;

  return (
    <aside className="drawer" aria-label="リポジトリ詳細">
      <div className="drawer-header">
        <h2>詳細</h2>
        <button className="icon-button" onClick={onClose} type="button" aria-label="詳細を閉じる">
          <X aria-hidden="true" size={18} />
        </button>
      </div>
      {detail ? (
        <>
          <section>
            <h3>README</h3>
            <pre className="readme-preview">{detail.readme_preview ?? "READMEプレビューはありません"}</pre>
          </section>
          <section className="form-section">
            <label htmlFor="memo">メモ</label>
            <textarea id="memo" value={memo} onChange={(event) => onMemoChange(event.target.value)} rows={5} />
            <button className="secondary-button" onClick={onSaveMemo} type="button">メモを保存</button>
          </section>
          <section className="form-section">
            <label htmlFor="tags">タグ</label>
            <input id="tags" value={tagInput} onChange={(event) => onTagInputChange(event.target.value)} placeholder="rust, cli" />
            <button className="secondary-button" onClick={onSaveTags} type="button">タグを保存</button>
          </section>
        </>
      ) : (
        <p>詳細を読み込み中です</p>
      )}
    </aside>
  );
}
