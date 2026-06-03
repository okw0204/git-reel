import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  use: {
    baseURL: "http://127.0.0.1:5173",
    trace: "on-first-retry"
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] }
    }
  ],
  webServer: [
    {
      command: "env GITHUB_CLIENT_ID= GITHUB_CLIENT_SECRET= GIT_REEL_DATABASE_URL=sqlite::memory: cargo run --manifest-path server/Cargo.toml",
      cwd: "..",
      url: "http://127.0.0.1:4317/api/health",
      reuseExistingServer: true,
      timeout: 120_000
    },
    {
      command: "npm run dev:web",
      cwd: "..",
      url: "http://127.0.0.1:5173",
      reuseExistingServer: true,
      timeout: 120_000
    }
  ]
});
