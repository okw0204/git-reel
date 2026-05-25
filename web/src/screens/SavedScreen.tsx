import { Search } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { SavedRepository } from "../types";

export function SavedScreen() {
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<SavedRepository[]>([]);

  useEffect(() => {
    // 入力のたびに即検索せず、短く待ってから API に問い合わせる。
    const timer = window.setTimeout(() => {
      void api.saved(query).then(setItems);
    }, 120);
    return () => window.clearTimeout(timer);
  }, [query]);

  return (
    <section className="stack">
      <header className="screen-header">
        <div>
          <p className="eyebrow">Saved</p>
          <h1>保存済み</h1>
        </div>
        <label className="search-box">
          <Search aria-hidden="true" size={18} />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="検索" />
        </label>
      </header>
      <div className="list-grid">
        {items.map((item) => (
          <article className="list-card" key={item.repository.id}>
            <h2>{item.repository.full_name}</h2>
            <p>{item.repository.description ?? "説明はまだありません"}</p>
            <div className="topic-list">
              {item.tags.map((tag) => <span key={tag}>{tag}</span>)}
            </div>
            {item.memo ? <p className="memo">{item.memo}</p> : null}
          </article>
        ))}
      </div>
    </section>
  );
}
