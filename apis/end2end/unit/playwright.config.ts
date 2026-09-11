import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: "*.spec.ts",
  outputDir: "../test-results/unit-tests",
  reporter: "list",
  workers: 1,
  retries: 0,
  forbidOnly: !!process.env.CI,
});
