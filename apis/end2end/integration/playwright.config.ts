import { defineConfig } from "@playwright/test";

// These checks create and drop their own database. Never use application data.
export default defineConfig({
  testDir: ".",
  testMatch: ["account_pool.spec.ts", "seed.spec.ts", "ownership.spec.ts", "connection.spec.ts", "tournament_fixtures.spec.ts"],
  outputDir: "../test-results/pool-tests",
  reporter: "list",
  workers: 1,
  retries: 0,
  timeout: 30_000,
  forbidOnly: !!process.env.CI,
});
