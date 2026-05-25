import { Bookmark, Clock3, Github, Settings, Sparkles } from "lucide-react";

type View = "reel" | "saved" | "history" | "settings";

type AppShellProps = {
  view: View;
  username: string | null;
  onNavigate: (view: View) => void;
  children: React.ReactNode;
};

const navItems = [
  { view: "reel", label: "リール", icon: Sparkles },
  { view: "saved", label: "保存済み", icon: Bookmark },
  { view: "history", label: "履歴", icon: Clock3 },
  { view: "settings", label: "設定", icon: Settings }
] as const;

export function AppShell({ view, username, onNavigate, children }: AppShellProps) {
  return (
    <div className="shell">
      <aside className="sidebar" aria-label="メインナビゲーション">
        <div className="brand">
          <Github aria-hidden="true" />
          <span>Git Reel</span>
        </div>
        <nav className="nav-list">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.view}
                className={view === item.view ? "nav-item active" : "nav-item"}
                onClick={() => onNavigate(item.view)}
                type="button"
              >
                <Icon aria-hidden="true" size={18} />
                {item.label}
              </button>
            );
          })}
        </nav>
        <div className="account">
          <span className="status-dot" aria-hidden="true" />
          <span>{username ?? "未接続"}</span>
        </div>
      </aside>
      <main className="content">{children}</main>
    </div>
  );
}
