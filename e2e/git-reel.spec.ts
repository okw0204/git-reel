import { expect, test } from "@playwright/test";

test("OAuth 未設定時は設定案内を表示する", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("GitHubに接続するとリールを開始できます")).toBeVisible();
  await expect(page.getByText("リールを開始するには GitHub OAuth の設定が必要です。GITHUB_CLIENT_ID と GITHUB_CLIENT_SECRET を設定してサーバーを起動してください。")).toBeVisible();
  await expect(page.getByRole("button", { name: "開発用に" + "接続" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "GitHubに接続" })).toHaveCount(0);
});
