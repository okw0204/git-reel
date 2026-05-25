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
      if (path === "/api/auth/state") return Response.json({ connected: false, username: null });
      if (path === "/api/auth/dev-connect") return Response.json({ connected: true, username: "local-dev" });
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

  test("開発用接続からリールを表示できる", async () => {
    render(<App />);

    await screen.findByText("GitHubに接続するとリールを開始できます");
    await userEvent.click(screen.getByRole("button", { name: "開発用に接続" }));

    expect(await screen.findByRole("heading", { name: "okw0204/git-reel" })).toBeInTheDocument();
  });

  test("履歴画面を表示できる", async () => {
    render(<App />);

    await waitFor(() => expect(screen.getByRole("button", { name: "履歴" })).toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "履歴" }));

    expect(await screen.findByText("saved")).toBeInTheDocument();
  });
});
