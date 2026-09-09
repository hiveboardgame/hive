import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { expect, test } from "@playwright/test";

test.describe("Report publishing", () => {
  for (const available of [false, true]) {
    test(available ? "publishes the summary and artifact link" : "reports missing output and fails explicitly", async ({}, testInfo) => {
      const directory = testInfo.outputPath("publish");
      mkdirSync(path.join(directory, "test-results"), { recursive: true });
      const destination = path.join(directory, "job-summary.md");
      const publisher = path.resolve(__dirname, "publish-summary.mjs");
      const env = { ...process.env, GITHUB_STEP_SUMMARY: destination,
        PLAYWRIGHT_REPORT_URL: "https://github.com/example/hive/actions/runs/123/artifacts/456" };
      if (available) writeFileSync(path.join(directory, "test-results/summary.md"), "# Results: failed\n");
      const execution = spawnSync(process.execPath, [publisher], { cwd: directory, env, encoding: "utf8", timeout: 10_000 });
      expect(execution.error, execution.stdout + execution.stderr).toBeUndefined();
      expect(execution.status).toBe(available ? 0 : 1);
      const markdown = readFileSync(destination, "utf8");
      expect(markdown).toContain(available ? "# Results: failed" : "Playwright results unavailable");
      if (available) expect(markdown).toContain(env.PLAYWRIGHT_REPORT_URL);
    });
  }
});
