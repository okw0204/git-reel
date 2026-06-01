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
  const [auth, setAuth] = useState<AuthState>({
    connected: false,
    username: null,
    oauth_configured: false,
    oauth_start_url: null
  });

  useEffect(() => {
    // 初期表示時にローカルの接続状態を復元し、リール画面の空状態を決める。
    void api.authState().then(setAuth);
  }, []);

  return (
    <AppShell view={view} username={auth.username} onNavigate={setView}>
      {view === "reel" ? <ReelScreen auth={auth} onAuthChange={setAuth} /> : null}
      {view === "saved" ? <SavedScreen /> : null}
      {view === "history" ? <HistoryScreen /> : null}
      {view === "settings" ? <SettingsScreen /> : null}
    </AppShell>
  );
}
