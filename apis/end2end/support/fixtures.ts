import { test as base, expect, type BrowserContext } from "@playwright/test";
import { TestData, type Account } from "./data";
import type { Player } from "./browser/player";
import { signIn } from "./browser/authentication";
import { dismissOnboardingBanners } from "./browser/onboarding";
import { attachFailureDiagnostics, logBrowserDiagnostics } from "./browser/diagnostics";

export type { Account } from "./data";
type Fixtures = {
  data: TestData;
  player: (account: Account, options?: { authenticate?: boolean; path?: string }) => Promise<Player>;
  isMobileLayout: boolean;
};

export const test = base.extend<Fixtures>({
  data: async ({}, use) => { await use(new TestData()); },
  isMobileLayout: async ({ viewport }, use) => { await use((viewport?.width ?? 1280) < 768); },
  page: async ({ page }, use, testInfo) => {
    await dismissOnboardingBanners(page);
    logBrowserDiagnostics(page, "visitor");
    try { await use(page); } finally {
      if (testInfo.status !== testInfo.expectedStatus) {
        await attachFailureDiagnostics([{ page, username: "visitor" }], testInfo);
      }
    }
  },
  player: [async ({ browser, baseURL }, use, testInfo) => {
    if (!baseURL) throw new Error("Configure a browser-test application URL.");
    const players: Player[] = [];
    const contexts: BrowserContext[] = [];
    let primaryError: unknown;
    try {
      await use(async (account, options = {}) => {
        const context = await browser.newContext();
        contexts.push(context);
        const page = await context.newPage();
        const player = { ...account, page, context };
        players.push(player);
        await dismissOnboardingBanners(page);
        logBrowserDiagnostics(page, account.username);
        if (options.authenticate !== false) await signIn(page, account, options.path);
        return player;
      });
    } catch (error) {
      primaryError = error;
      throw error;
    } finally {
      if (testInfo.status !== testInfo.expectedStatus) await attachFailureDiagnostics(players, testInfo);
      const results = await Promise.allSettled(contexts.map(context => context.close()));
      const failure = results.find(result => result.status === "rejected");
      if (failure?.status === "rejected") {
        if (primaryError || testInfo.status !== testInfo.expectedStatus) {
          await testInfo.attach("Browser context cleanup failure", { body: String(failure.reason), contentType: "text/plain" });
        } else { throw failure.reason; }
      }
    }
  }, { timeout: 90_000 }],
});

export { expect };
