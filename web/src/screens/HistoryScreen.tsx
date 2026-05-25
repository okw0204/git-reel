import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { HistoryItem } from "../types";

export function HistoryScreen() {
  const [items, setItems] = useState<HistoryItem[]>([]);

  useEffect(() => {
    void api.history().then(setItems);
  }, []);

  return (
    <section className="stack">
      <header className="screen-header">
        <div>
          <p className="eyebrow">History</p>
          <h1>履歴</h1>
        </div>
      </header>
      <div className="timeline">
        {items.map((item) => (
          <article className="history-row" key={`${item.repository.id}-${item.latest_event_at}`}>
            <span>{item.latest_event}</span>
            <strong>{item.repository.full_name}</strong>
            <time>{new Date(item.latest_event_at).toLocaleString("ja-JP")}</time>
          </article>
        ))}
      </div>
    </section>
  );
}
