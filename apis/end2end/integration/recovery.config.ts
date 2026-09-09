import { defineConfig } from "@playwright/test";
import config from "../playwright.config";

// Run separately: these checks deliberately leave state for fixture recovery.
export default defineConfig({
  ...config,
  testDir: ".",
  testMatch: "account_recovery.spec.ts",
  outputDir: "../test-results/recovery-tests",
  reporter: "list",
  timeout: 120_000,
  workers: 2,
  projects: config.projects!.filter(project => ["chromium-desktop", "webkit-mobile"].includes(project.name!)),
});
