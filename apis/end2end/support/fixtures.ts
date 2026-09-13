import { test as base, expect, type TestInfo } from "@playwright/test";
import { accountWaitTimeout, reserveAccounts } from "./accounts/pool";
import { withUsers, type Players } from "./browser/session";
import { signIn } from "./browser/authentication";
import { attachFailureDiagnostics, logBrowserDiagnostics } from "./browser/diagnostics";
import type { Player } from "./browser/player";

type Fixtures = {
  user: Player;
  players: Players;
  isMobileLayout: boolean;
  publicChallenge: boolean;
};

function signInWithDiagnostics(testInfo: TestInfo): typeof signIn {
  return async (page, username, baseURL) => {
    logBrowserDiagnostics(page, username);
    try {
      await signIn(page, username, baseURL);
    } catch (error) {
      await attachFailureDiagnostics([{ page, username }], testInfo);
      throw error;
    }
  };
}

const timeout = accountWaitTimeout + 90_000;
export const test = base.extend<Fixtures>({
  publicChallenge: [false, { option: true }],
  isMobileLayout: async ({}, use, testInfo) => {
    // Firefox's narrow viewport project is mobile layout coverage, too.
    await use(testInfo.project.name.endsWith("-mobile"));
  },
  user: [async ({ browser, baseURL, isMobileLayout, publicChallenge }, use, testInfo) => {
    await withUsers({ browser, baseURL, isMobileLayout, testInfo, count: 1, authenticate: signInWithDiagnostics(testInfo), reserve: count => reserveAccounts(count, { publicChallenge }) }, async ([user]) => {
      try { await use(user); } finally {
        if (testInfo.status !== testInfo.expectedStatus) await attachFailureDiagnostics([user], testInfo);
      }
    });
  }, { timeout }],
  players: [async ({ browser, baseURL, isMobileLayout, publicChallenge }, use, testInfo) => {
    await withUsers({ browser, baseURL, isMobileLayout, testInfo, count: 2, authenticate: signInWithDiagnostics(testInfo), reserve: count => reserveAccounts(count, { publicChallenge }) }, async ([userOne, userTwo]) => {
      try { await use({ userOne, userTwo }); } finally {
        if (testInfo.status !== testInfo.expectedStatus) await attachFailureDiagnostics([userOne, userTwo], testInfo);
      }
    });
  }, { timeout }],
});

export { expect };
