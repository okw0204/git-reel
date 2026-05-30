import { expect, test } from "@playwright/test";

// 開発用 API で接続状態を作り、保存・スキップ・履歴確認まで MVP の最短フローを通す。
test("user can connect, browse, save, skip, and inspect local views", async ({ page, request }) => {
  await page.goto("/");
  await expect(page.getByText("GitHubに接続するとリールを開始できます")).toBeVisible();

  const response = await request.post("/api/auth/dev-connect", { data: { username: "e2e-user" } });
  expect(response.ok()).toBeTruthy();
  await page.reload();
  await expect(page.getByRole("heading", { name: /.+\/.+/ })).toBeVisible();

  await page.getByRole("button", { name: "保存", exact: true }).click();
  await page.getByRole("button", { name: "スキップ" }).click();

  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("heading", { name: /rust-lang\/rust|tauri-apps\/tauri|sqlite\/sqlite/ })).toBeVisible();

  await page.getByRole("button", { name: "履歴" }).click();
  await expect(page.getByText(/saved|skipped|viewed/).first()).toBeVisible();
});
