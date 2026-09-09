import { defineConfig } from "playwright/test";

// Reporter checks use synthetic results and never launch browsers or touch the
// shared game accounts. Keep them separate from the six-project E2E matrix.
export default defineConfig({
  testDir: ".",
  testMatch: "*.spec.ts",
  outputDir: "../test-results/reporter-tests",
  reporter: "list",
  workers: 1,
  retries: 0,
  timeout: 30_000,
  forbidOnly: !!process.env.CI,
});
