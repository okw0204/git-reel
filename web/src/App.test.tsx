import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import App from "./App";

const repo = {
  id: 1,
  github_id: 10,
  owner: "okw0204",
  name: "git-reel",
  full_name: "okw0204/git-reel",
  description: "GitHub discovery",
  primary_language: "Rust",
  stars: 120,
  forks: 8,
  license: "MIT",
  updated_at: "2026-05-25T00:00:00Z",
  topics: ["github", "discovery"],
  html_url: "https://github.com/okw0204/git-reel",
  readme_preview: "# Git Reel"
};

describe("App", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/auth/state") return Response.json({ connected: false, username: null, oauth_configured: false });
      if (path === "/api/auth/dev-connect") return Response.json({ connected: true, username: "local-dev", oauth_configured: false });
      if (path === "/api/reel/current") return Response.json({ repository: null, empty_reason: "auth_required" });
      if (path === "/api/reel/next") return Response.json({ repository: repo, empty_reason: null });
      if (path === "/api/reel/1/save") return Response.json({ ok: true });
      if (path === "/api/history") return Response.json([{ repository: repo, latest_event: "saved", latest_event_at: "2026-05-25T00:00:00Z" }]);
      return Response.json({ ok: true });
    }));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  test("未接続時に GitHub OAuth 接続へ遷移できる", async () => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/auth/state") {
        return Response.json({
          connected: false,
          username: null,
          oauth_configured: true,
          oauth_start_url: "http://127.0.0.1:4317/api/auth/github/start"
        });
      }
      if (path === "/api/reel/current") return Response.json({ repository: null, empty_reason: "auth_required" });
      return Response.json({ ok: true });
    }));
    const location = { href: "http://127.0.0.1:5173/" };
    vi.stubGlobal("location", location);

    render(<App />);

    await screen.findByText("GitHubに接続するとリールを開始できます");
    expect(screen.getByText("GitHub OAuth で接続すると、保存済み OAuth token を使って実リポジトリ候補を取得します。")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "GitHubに接続" }));

    expect(window.location.href).toBe("http://127.0.0.1:4317/api/auth/github/start");
  });

  test("認証状態の読み込み中は接続ボタンを表示しない", () => {
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/auth/state") return new Promise<Response>(() => undefined);
      if (path === "/api/reel/current") return Promise.resolve(Response.json({ repository: null, empty_reason: "auth_required" }));
      return Promise.resolve(Response.json({ ok: true }));
    }));

    render(<App />);

    expect(screen.queryByRole("button", { name: "開発用に接続" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "GitHubに接続" })).not.toBeInTheDocument();
  });

  test("OAuth 未設定のローカル環境では開発用接続でリールを表示できる", async () => {
    render(<App />);

    await screen.findByText("GitHubに接続するとリールを開始できます");
    expect(screen.getByText("OAuth 未設定のローカル環境では開発用接続でシード候補を使います。")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "開発用に接続" }));

    expect(await screen.findByRole("heading", { name: "okw0204/git-reel" })).toBeInTheDocument();
  });

  test("履歴画面を表示できる", async () => {
    render(<App />);

    await waitFor(() => expect(screen.getByRole("button", { name: "履歴" })).toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "履歴" }));

    expect(await screen.findByText("saved")).toBeInTheDocument();
  });

  test("接続済みのリール表示では候補を進めず現在の候補を表示する", async () => {
    const fetch = vi.fn(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/auth/state") return Response.json({ connected: true, username: "local-dev", oauth_configured: false });
      if (path === "/api/reel/current") return Response.json({ repository: repo, empty_reason: null });
      if (path === "/api/reel/next") {
        return Response.json({ repository: { ...repo, id: 2, full_name: "next/repo" }, empty_reason: null });
      }
      return Response.json({ ok: true });
    });
    vi.stubGlobal("fetch", fetch);

    render(<App />);

    expect(await screen.findByRole("heading", { name: "okw0204/git-reel" })).toBeInTheDocument();
    expect(fetch).not.toHaveBeenCalledWith("/api/reel/next", expect.anything());
  });
});
