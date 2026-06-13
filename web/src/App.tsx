import { useEffect, useState } from "react";
import { api } from "./api/client";
import { AppShell } from "./components/AppShell";
import { HistoryScreen } from "./screens/HistoryScreen";
import { ReelScreen } from "./screens/ReelScreen";
import { SavedScreen } from "./screens/SavedScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import type { AuthState } from "./types";

type View = "reel" | "saved" | "history" | "settings";

export default function App() {
  const [view, setView] = useState<View>("reel");
  const [authLoaded, setAuthLoaded] = useState(false);
  const [auth, setAuth] = useState<AuthState>({
    connected: false,
    username: null,
    oauth_configured: false,
    oauth_start_url: null
  });

  useEffect(() => {
    // 初期表示時にローカルの接続状態を復元し、リール画面の空状態を決める。
    void api.authState().then(setAuth).finally(() => setAuthLoaded(true));
  }, []);

  return (
    <AppShell view={view} username={auth.username} onNavigate={setView}>
      {view === "reel" && !authLoaded ? (
        <section className="center-panel">
          <p className="eyebrow">Local-first discovery</p>
          <h1>接続状態を確認しています</h1>
          <p>ローカルに保存された GitHub 接続情報を読み込んでいます。</p>
        </section>
      ) : null}
      {view === "reel" && authLoaded ? <ReelScreen auth={auth} /> : null}
      {view === "saved" ? <SavedScreen /> : null}
      {view === "history" ? <HistoryScreen /> : null}
      {view === "settings" ? <SettingsScreen /> : null}
    </AppShell>
  );
}
