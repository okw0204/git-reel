import { expect, test } from "@playwright/test";

test("user can connect, browse, save, skip, and inspect local views", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("GitHubに接続するとリールを開始できます")).toBeVisible();

  await page.getByRole("button", { name: "開発用に接続" }).click();
  await expect(page.getByRole("heading", { name: /.+\/.+/ })).toBeVisible();

  await page.getByRole("button", { name: "保存", exact: true }).click();
  await page.getByRole("button", { name: "スキップ" }).click();

  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("heading", { name: /rust-lang\/rust|tauri-apps\/tauri|sqlite\/sqlite/ })).toBeVisible();

  await page.getByRole("button", { name: "履歴" }).click();
  await expect(page.getByText(/saved|skipped|viewed/).first()).toBeVisible();
});
