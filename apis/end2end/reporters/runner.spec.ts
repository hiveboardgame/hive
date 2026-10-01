import { test, expect } from "@playwright/test";
import { runSynthetic } from "./synthetic-runner";

test.describe("Reporter runner integration", () => {
  // Worker startup and intentional worker restarts retain the existing budget.
  test.describe.configure({ timeout: 90_000 });

  test("groups ordinary statuses and fixture steps across projects", async ({}, testInfo) => {
    const markdown = runSynthetic(testInfo, `
      test('Pass', async ({ ready }) => {
        await test.step('Create challenge', async () => {});
        await test.step('Accept challenge', async () => { expect(true).toBe(true); });
      });
      test.skip('Skipped', async () => {});
      test('Expected failure', async () => { test.fail(); expect(false).toBe(true); });
    `, { expectedExit: 0 });
    expect(markdown).toContain("3 scenarios · 6 browser/layout cases");
    expect(markdown).toContain("2 passed");
    expect(markdown).toContain("2 skipped");
    expect(markdown).toContain("2 expected failure");
    expect(markdown).toContain("| Scenario | chromium desktop | webkit mobile |");
    expect(markdown).toContain("Sign in as user&#95;1");
    expect(markdown).toContain("/blob/tested-commit/scenarios.ts#L");
    expect(markdown).not.toContain("expect.toBe");
  });

  test("retains retry attempts and final failures", async ({}, testInfo) => {
    const markdown = runSynthetic(testInfo, `
      test('Flaky', async ({ ready }, info) => {
        await test.step('Accept after retry', async () => { expect(info.retry).toBe(1); });
      });
      test('Failure', async ({ ready }) => {
        await test.step('Reject draw', async () => { expect('pending').toBe('rejected'); });
      });
    `, { retries: 1 });
    expect(markdown).toContain("2 scenarios · 4 browser/layout cases");
    expect(markdown).toContain("2 failed · 2 flaky");
    expect(markdown).toContain("Attempt 1: ❌ Failed");
    expect(markdown).toContain("Attempt 2: ✅ Passed");
    expect(markdown).toContain("Reject draw");
  });

  test("reports a timed-out named step", async ({}, testInfo) => {
    const markdown = runSynthetic(testInfo, `
      test('Timeout', async () => {
        test.setTimeout(150);
        await test.step('Wait for opponent', async () => { await new Promise(() => {}); });
      });
    `);
    expect(markdown).toContain("1 scenarios · 2 browser/layout cases");
    expect(markdown).toContain("2 timed out");
    expect(markdown).toContain("Wait for opponent");
  });

  test("reports fixture failures and global teardown errors", async ({}, testInfo) => {
    const markdown = runSynthetic(testInfo, `
      test('Setup failure', async ({ ready }) => {});
      test('Teardown failure', async ({ ready }) => {});
    `, { globalTeardown: true });
    expect(markdown).toContain("2 scenarios · 4 browser/layout cases");
    expect(markdown).toContain("4 failed");
    expect(markdown).toContain("Sign in both players › Sign in as user&#95;1");
    expect(markdown).toContain("Synthetic login failure");
    expect(markdown).toContain("Synthetic fixture teardown error");
    expect(markdown).toContain("Synthetic global teardown error");
  });
});
