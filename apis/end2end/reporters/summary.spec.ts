import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { expect, test } from "playwright/test";
import { renderSummary, type Attempt, type Scenario, type Summary } from "./summary";

const passingAttempt: Attempt = {
  number: 1, status: "passed", duration: 1200, errors: [],
  steps: [{ title: "Sign in", status: "passed", duration: 1000, children: [
    { title: "Sign in as user_1", status: "passed", duration: 600, children: [] },
  ] }],
};
function scenario(overrides: Partial<Scenario> = {}): Scenario {
  return {
    key: "challenge", feature: "Challenges", title: "Accept a challenge",
    project: "chromium-desktop", location: { file: "tests/challenges.spec.ts", line: 10 },
    status: "passed", attempts: [passingAttempt], ...overrides,
  };
}
function summary(scenarios: Scenario[], overrides: Partial<Summary> = {}): Summary {
  return {
    status: "passed", duration: 2000, projects: [...new Set(scenarios.map(test => test.project))],
    scenarios, errors: [], sourceBaseURL: "https://github.com/example/hive/blob/tested-commit", ...overrides,
  };
}

test("groups browsers by scenario and reports nested durations without summing them", () => {
  const report = renderSummary(summary([scenario(), scenario({ project: "webkit-mobile" })]));
  expect(report).toContain("1 scenarios · 2 browser/layout cases");
  expect(report).toContain("| Scenario | chromium desktop | webkit mobile |");
  expect(report).toContain("| Accept a challenge | ✅ Passed · 1.2 s | ✅ Passed · 1.2 s |");
  expect(report).toContain("  - ✅ Passed — Sign in as user&#95;1 · 600 ms");
  expect(report).toContain("/blob/tested-commit/tests/challenges.spec.ts#L10");
});

test("retains failed attempts for flaky cases and surfaces the deepest failed step", () => {
  const failed: Attempt = {
    number: 1, status: "failed", duration: 800, errors: ["Expected same game URL"],
    steps: [{ title: "Start game", status: "failed", duration: 800, children: [
      { title: "Accept challenge", status: "failed", duration: 300, children: [] },
    ] }],
  };
  const report = renderSummary(summary([scenario({ status: "flaky", attempts: [failed, { ...passingAttempt, number: 2 }] })]));
  expect(report).toContain("1 flaky");
  expect(report).toContain("⚠️ Flaky · 2.0 s");
  expect(report).toContain("Step: Start game › Accept challenge");
  expect(report).toContain("Attempt 1: ❌ Failed");
  expect(report).toContain("Attempt 2: ✅ Passed");
  expect(report).toContain("<details>\n<summary>⚠️ Flaky — Accept a challenge · chromium desktop</summary>");
  expect(report).not.toContain("### ⚠️ Flaky");
  expect(report.indexOf("## Needs attention")).toBeLessThan(report.indexOf("## Scenario matrix"));
});

test("keeps final failures expanded while flaky diagnostics are collapsible", () => {
  const report = renderSummary(summary([scenario({ status: "failed", attempts: [
    { ...passingAttempt, status: "failed", errors: ["Still failing"] },
  ] })]));
  expect(report).toContain("### ❌ Failed — Accept a challenge · chromium desktop");
  expect(report).toContain("> Still failing");
  expect(report).not.toContain("<summary>❌ Failed — Accept a challenge");
});

test("distinguishes skipped, interrupted, timed out, expected failure and unexecuted cases", () => {
  const statuses = ["skipped", "interrupted", "timed out", "expected failure", "not run"] as const;
  const report = renderSummary(summary(statuses.map(status => scenario({
    key: status, title: status, status, attempts: status === "not run" ? [] : [{ ...passingAttempt, status }],
  }))));
  for (const status of statuses) expect(report).toContain(`1 ${status}`);
  expect(report).toContain("This test was not executed.");
  expect(report).toContain("0 passed");
});

test("does not merge equal titles from different source identities", () => {
  const report = renderSummary(summary([scenario(), scenario({ key: "other-file" })]));
  expect(report).toContain("2 scenarios · 2 browser/layout cases");
  expect(report.match(/\| Accept a challenge \|/g)).toHaveLength(2);
});

test("escapes Markdown and HTML, strips terminal codes and bounds error messages", () => {
  const report = renderSummary(summary([scenario({
    title: '<script>|[link](https://bad.test)\n**title**', status: "failed",
    attempts: [{ ...passingAttempt, status: "failed", errors: ['\x1b[31m<script>bad</script>\x1b[0m' + 'x'.repeat(20_000)] }],
  })], { errors: ["Global worker error"] }));
  expect(report).not.toContain("<script>");
  expect(report).not.toContain("\x1b[");
  expect(report).toContain("&lt;script&gt;&#124;&#91;link&#93;");
  expect(report).toContain("## Run errors");
  expect(report).toContain("Setup, teardown, or test body outside a named step");
  expect(report.length).toBeLessThan(10_000);
});

test("bounds large reports, preserves failure detail first and closes details blocks", () => {
  const failed = scenario({ key: "failed", status: "failed", title: "Important failure", attempts: [
    { ...passingAttempt, status: "failed", errors: ["Do not lose this failure"] },
  ] });
  const cases = [...Array.from({ length: 200 }, (_, index) => scenario({ key: String(index), title: `Passing ${index}` })), failed];
  const report = renderSummary(summary(cases), 6000);
  expect(Buffer.byteLength(report)).toBeLessThanOrEqual(6000);
  expect(report).toContain("Do not lose this failure");
  expect(report).toContain("Summary shortened to fit GitHub");
  expect(report.match(/<details>/g)?.length).toBe(report.match(/<\/details>/g)?.length);
});

test("handles discovery errors without claiming tests passed", () => {
  const report = renderSummary(summary([], { status: "failed", errors: ["Cannot load test module"] }));
  expect(report).toContain("Playwright results: failed");
  expect(report).toContain("0 scenarios · 0 browser/layout cases");
  expect(report).toContain("Cannot load test module");
});

test("publishes the summary and artifact link, and fails explicitly for missing output", async ({}, testInfo) => {
  const directory = testInfo.outputPath("publish");
  mkdirSync(path.join(directory, "test-results"), { recursive: true });
  const destination = path.join(directory, "job-summary.md");
  const publisher = path.resolve(__dirname, "publish-summary.mjs");
  const env = { ...process.env, GITHUB_STEP_SUMMARY: destination,
    PLAYWRIGHT_REPORT_URL: "https://github.com/example/hive/actions/runs/123/artifacts/456" };
  const missing = spawnSync(process.execPath, [publisher], { cwd: directory, env, encoding: "utf8" });
  expect(missing.status).toBe(1);
  expect(readFileSync(destination, "utf8")).toContain("Playwright results unavailable");
  writeFileSync(path.join(directory, "test-results/summary.md"), "# Results: failed\n");
  writeFileSync(destination, "");
  const published = spawnSync(process.execPath, [publisher], { cwd: directory, env, encoding: "utf8" });
  expect(published.status).toBe(0);
  expect(readFileSync(destination, "utf8")).toContain("# Results: failed");
  expect(readFileSync(destination, "utf8")).toContain(env.PLAYWRIGHT_REPORT_URL);
});

test("integrates with real Playwright fixtures, retries, failures, skips and run errors", async ({}, testInfo) => {
  // Intentional failures restart workers; allow startup time for both projects.
  test.setTimeout(90_000);
  const directory = testInfo.outputPath("integration");
  mkdirSync(directory, { recursive: true });
  const runner = require.resolve("playwright/test");
  const reporter = path.resolve(__dirname, "github-summary.ts");
  const report = path.join(directory, "summary.md");
  const config = path.join(directory, "playwright.config.ts");
  writeFileSync(config, `export default {
    testDir: '.', testMatch: 'scenarios.ts', workers: 1, retries: 1, timeout: 3000,
    globalTeardown: './teardown.ts',
    outputDir: ${JSON.stringify(path.join(directory, 'results'))},
    reporter: [[${JSON.stringify(reporter)}, { outputFile: ${JSON.stringify(report)} }]],
    projects: [{ name: 'chromium-desktop' }, { name: 'webkit-mobile' }],
  };`);
  writeFileSync(path.join(directory, "teardown.ts"), 'export default () => { throw new Error("Synthetic global teardown error"); };');
  writeFileSync(path.join(directory, "scenarios.ts"), `
    import { test as base, expect } from ${JSON.stringify(runner)};
    const test = base.extend<{ ready: void }>({
      ready: async ({}, use, info) => {
        await test.step('Sign in both players', async () => {
          await test.step('Sign in as user_1', async () => {
            if (info.title === 'Setup failure') throw new Error('Synthetic login failure');
          });
        });
        await use();
      },
    });
    test.describe('Challenges', () => {
      test('Pass', async ({ ready }) => {
        await test.step('Create challenge', async () => {});
        await test.step('Accept challenge', async () => { expect(true).toBe(true); });
      });
      test('Flaky', async ({ ready }, info) => {
        await test.step('Accept after retry', async () => { expect(info.retry).toBe(1); });
      });
      test('Failure', async ({ ready }) => {
        await test.step('Reject draw', async () => { expect('pending').toBe('rejected'); });
      });
      test.skip('Skipped', async () => {});
      test('Expected failure', async () => { test.fail(); expect(false).toBe(true); });
      test('Timeout', async () => {
        test.setTimeout(150);
        await test.step('Wait for opponent', async () => { await new Promise(() => {}); });
      });
      test('Setup failure', async ({ ready }) => {});
    });
  `);
  const execution = spawnSync(process.execPath, [path.join(path.dirname(require.resolve("playwright/package.json")), "cli.js"), "test", "--config", config], {
    cwd: directory, encoding: "utf8", timeout: 75_000,
    env: { ...process.env, GITHUB_ACTIONS: "", GITHUB_WORKSPACE: directory,
      GITHUB_SERVER_URL: "https://github.com", GITHUB_REPOSITORY: "example/hive", GITHUB_SHA: "tested-commit" },
  });
  expect(execution.error, execution.stdout + execution.stderr).toBeUndefined();
  expect(execution.status, execution.stdout + execution.stderr).toBe(1);
  const markdown = readFileSync(report, "utf8");
  expect(markdown).toContain("7 scenarios · 14 browser/layout cases");
  expect(markdown).toContain("2 passed · 4 failed · 2 flaky · 2 skipped · 2 timed out");
  expect(markdown).toContain("2 expected failure");
  expect(markdown).toContain("Sign in as user&#95;1");
  expect(markdown).toContain("Sign in both players › Sign in as user&#95;1");
  expect(markdown).toContain("Attempt 2: ✅ Passed");
  expect(markdown).toContain("Synthetic global teardown error");
  expect(markdown).toContain("/blob/tested-commit/scenarios.ts#L");
  expect(markdown).not.toContain("expect.toBe");
});
