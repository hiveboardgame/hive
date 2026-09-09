import { test as base, expect } from "playwright/test";
import { signIn } from "./test_utils/authentication";
import type { Player } from "./test_utils/player";

type Fixtures = {
  players: { userOne: Player; userTwo: Player };
  isMobileLayout: boolean;
};

export const test = base.extend<Fixtures>({
  isMobileLayout: async ({}, use, testInfo) => {
    // Firefox's narrow viewport project is mobile layout coverage, too.
    await use(testInfo.project.name.endsWith("-mobile"));
  },
  players: async ({ browser, page }, use) => {
    // Playwright applies the project's context options to newContext(), including
    // device emulation, baseURL, HTTPS handling, and service-worker policy.
    const opponentContext = await browser.newContext();
    try {
      const userOne: Player = { username: "user_1", page };
      const userTwo: Player = {
        username: "user_2",
        page: await opponentContext.newPage(),
      };
      await test.step("Sign in both players", async () => {
        for (const player of [userOne, userTwo]) {
          await test.step(`Sign in as ${player.username}`, async () => {
            await signIn(player.page, player.username);
          }, { box: true });
        }
      }, { box: true });
      await use({ userOne, userTwo });
    } finally {
      // Also runs if creating the second page or signing in fails.
      await opponentContext.close();
    }
  },
});

export { expect };
