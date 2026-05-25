import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { SettingsSummary } from "../types";

export function SettingsScreen() {
  const [settings, setSettings] = useState<SettingsSummary | null>(null);

  useEffect(() => {
    void api.settings().then(setSettings);
  }, []);

  return (
    <section className="stack">
      <header className="screen-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h1>設定</h1>
        </div>
      </header>
      {settings ? (
        <div className="settings-grid">
          <div><span>接続</span><strong>{settings.auth_connected ? "接続済み" : "未接続"}</strong></div>
          <div><span>ユーザー</span><strong>{settings.username ?? "-"}</strong></div>
          <div><span>DB</span><strong>{settings.database}</strong></div>
          <div><span>探索</span><strong>{settings.discovery_mix.join(", ")}</strong></div>
        </div>
      ) : null}
    </section>
  );
}
