import { useEffect } from "react";

type ShortcutHandlers = {
  onNext: () => void;
  onPrevious: () => void;
  onSave: () => void;
  onSkip: () => void;
  onDetail: () => void;
  enabled?: boolean;
};

export function useKeyboardShortcuts({
  onNext,
  onPrevious,
  onSave,
  onSkip,
  onDetail,
  enabled = true
}: ShortcutHandlers) {
  useEffect(() => {
    if (!enabled) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      // メモやタグ入力中は文字入力を優先し、リール操作のショートカットを発火させない。
      if (target?.closest("input, textarea, [contenteditable=true]")) return;

      if (event.key === "j" || event.key === "ArrowRight") onNext();
      if (event.key === "k" || event.key === "ArrowLeft") onPrevious();
      if (event.key === "s") onSave();
      if (event.key === "x") onSkip();
      if (event.key === "d") onDetail();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [enabled, onDetail, onNext, onPrevious, onSave, onSkip]);
}
