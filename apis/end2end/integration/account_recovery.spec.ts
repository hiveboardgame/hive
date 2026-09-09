import { test, expect } from "@playwright/test";
import { reserveAccounts } from "../support/accounts/pool";
import { publicTimeControl } from "../support/accounts/catalog";
import { withUsers, type Players } from "../support/browser/session";
import { createDirectChallenge, createPublicChallenge } from "../support/game/challenges";
import { startGame } from "../support/game/setup";
import { placePiece } from "../support/game/board";

const scenarios: { name: string; kind: "challenges" | "games"; create: (players: Players, mobile: boolean) => Promise<void> }[] = [
  {
    name: "a direct challenge", kind: "challenges",
    create: async ({ userOne, userTwo }) => { await createDirectChallenge(userOne, userTwo, "White"); },
  },
  {
    name: "a public challenge", kind: "challenges",
    create: async ({ userOne }) => { await createPublicChallenge(userOne); },
  },
  {
    name: "an unstarted game", kind: "games",
    create: async ({ userOne, userTwo }, mobile) => {
      await startGame({ white: userOne, black: userTwo, isMobileLayout: mobile });
    },
  },
  {
    name: "a game after two turns", kind: "games",
    create: async ({ userOne, userTwo }, mobile) => {
      await startGame({ white: userOne, black: userTwo, isMobileLayout: mobile });
      await placePiece(userOne.page, "White Ant 1", "16, 16");
      await placePiece(userTwo.page, "Black Ant 1", "15, 16");
    },
  },
];

for (const scenario of scenarios) {
  test(`recovers ${scenario.name} left by an interrupted attempt`, async ({ browser, baseURL }, testInfo) => {
    const reservation = await reserveAccounts(2);
    // Keep ownership between sessions to simulate an interrupted browser attempt.
    const release = reservation.release.bind(reservation);
    reservation.release = async () => {};
    const options = { browser, baseURL, count: 2 as const, reserve: async () => reservation, testInfo, isMobileLayout: testInfo.project.name.endsWith("-mobile") };
    try {
      await withUsers(options, async () => {});
      // Simulate a process dying after its browser actions: close the contexts
      // but deliberately omit teardown cleanup, while retaining ownership here.
      await withUsers({ ...options, cleanup: async () => {} }, async ([userOne, userTwo]) => {
        await scenario.create({ userOne, userTwo, publicTimeControl: publicTimeControl(userOne) }, options.isMobileLayout);
        await expect.poll(async () => (await reservation.state())[scenario.kind].length).toBe(1);
      });
      expect((await reservation.state())[scenario.kind]).toHaveLength(1);
      // Fresh contexts must recover before the next scenario can receive players.
      await withUsers(options, async () => {
        expect(await reservation.state()).toEqual({ challenges: [], games: [] });
      });
      expect(browser.contexts()).toHaveLength(0);
    } finally {
      await release();
    }
  });
}

test("a failed test cleans its challenge before releasing the pair", async ({ browser, baseURL }, testInfo) => {
  const reservation = await reserveAccounts(2);
  const release = reservation.release.bind(reservation);
  reservation.release = async () => {};
  try {
    await expect(withUsers({
      browser, baseURL, count: 2, reserve: async () => reservation, testInfo, isMobileLayout: testInfo.project.name.endsWith("-mobile"),
    }, async ([userOne, userTwo]) => {
      await createDirectChallenge(userOne, userTwo, "White");
      await expect.poll(async () => (await reservation.state()).challenges.length).toBe(1);
      throw new Error("Intentional scenario failure");
    })).rejects.toThrow("Intentional scenario failure");
    expect(await reservation.state()).toEqual({ challenges: [], games: [] });
    expect(browser.contexts()).toHaveLength(0);
  } finally {
    await release();
  }
});
