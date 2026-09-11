import { test as base, expect } from "@playwright/test";
import { accountWaitTimeout } from "./accounts/pool";
import { publicTimeControl } from "./accounts/catalog";
import { withUsers, type Players } from "./browser/session";
import type { Player } from "./browser/player";

type Fixtures = {
  user: Player;
  players: Players;
  isMobileLayout: boolean;
};

const timeout = accountWaitTimeout + 90_000;
export const test = base.extend<Fixtures>({
  isMobileLayout: async ({}, use, testInfo) => {
    // Firefox's narrow viewport project is mobile layout coverage, too.
    await use(testInfo.project.name.endsWith("-mobile"));
  },
  user: [async ({ browser, baseURL, isMobileLayout }, use, testInfo) => {
    await withUsers({ browser, baseURL, isMobileLayout, testInfo, count: 1 }, async ([user]) => use(user));
  }, { timeout }],
  players: [async ({ browser, baseURL, isMobileLayout }, use, testInfo) => {
    await withUsers({ browser, baseURL, isMobileLayout, testInfo, count: 2 }, async ([userOne, userTwo]) => {
      await use({ userOne, userTwo, publicTimeControl: publicTimeControl(userOne) });
    });
  }, { timeout }],
});

export { expect };
