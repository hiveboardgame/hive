import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { expect, type TestInfo } from "@playwright/test";

// This helper only writes and launches a synthetic suite when its caller runs.
export function runSynthetic(testInfo: TestInfo, scenarios: string, {
  retries = 0, globalTeardown = false, expectedExit = 1,
}: { retries?: number; globalTeardown?: boolean; expectedExit?: number } = {}) {
  const directory = testInfo.outputPath("integration");
  mkdirSync(directory, { recursive: true });
  const report = path.join(directory, "summary.md");
  const config = path.join(directory, "playwright.config.ts");
  writeFileSync(config, `export default {
    testDir: '.', testMatch: 'scenarios.ts', workers: 1, retries: ${retries}, timeout: 3000,
    ${globalTeardown ? "globalTeardown: './teardown.ts'," : ""}
    outputDir: ${JSON.stringify(path.join(directory, "results"))},
    reporter: [[${JSON.stringify(path.resolve(__dirname, "github-summary.ts"))}, { outputFile: ${JSON.stringify(report)} }]],
    projects: [{ name: 'chromium-desktop' }, { name: 'webkit-mobile' }],
  };`);
  if (globalTeardown) {
    writeFileSync(path.join(directory, "teardown.ts"), 'export default () => { throw new Error("Synthetic global teardown error"); };');
  }
  writeFileSync(path.join(directory, "scenarios.ts"), `
    import { test as base, expect } from ${JSON.stringify(require.resolve("@playwright/test"))};
    const test = base.extend<{ ready: void }>({
      ready: async ({}, use, info) => {
        await test.step('Sign in both players', async () => {
          await test.step('Sign in as user_1', async () => {
            if (info.title === 'Setup failure') throw new Error('Synthetic login failure');
          });
        });
        await use();
        if (info.title === 'Teardown failure') throw new Error('Synthetic fixture teardown error');
      },
    });
    test.describe('Challenges', () => { ${scenarios} });
  `);
  const execution = spawnSync(process.execPath, [path.join(path.dirname(require.resolve("playwright/package.json")), "cli.js"), "test", "--config", config], {
    cwd: directory, encoding: "utf8", timeout: 75_000,
    env: { ...process.env, GITHUB_ACTIONS: "", GITHUB_WORKSPACE: directory,
      GITHUB_SERVER_URL: "https://github.com", GITHUB_REPOSITORY: "example/hive", GITHUB_SHA: "tested-commit" },
  });
  expect(execution.error, execution.stdout + execution.stderr).toBeUndefined();
  expect(execution.status, execution.stdout + execution.stderr).toBe(expectedExit);
  return readFileSync(report, "utf8");
}
