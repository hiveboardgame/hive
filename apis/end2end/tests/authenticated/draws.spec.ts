import { expect, test } from "../../support/fixtures";
import { confirmControl } from "../../support/game/controls";
import { showTab } from "../../support/game/panels";
import { startGame } from "../../support/game/setup";
import { playOpening } from "../../support/game/opening";

test.describe("Draws", () => {
  test.describe.configure({ timeout: 90_000 });

  test("Reject a draw and allow another offer", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 2);
    await test.step("Reject a draw and allow another offer", async () => {
      await confirmControl(whitePage, "Offer Draw");
      await expect(blackPage.getByText("Opponent offers a draw")).toBeVisible();
      await blackPage.getByTitle("Reject Draw", { exact: true }).click();
      await expect(blackPage.getByText("Opponent offers a draw")).toBeHidden();
      await expect(whitePage.getByTitle("Offer Draw", { exact: true })).toBeEnabled();
    }, { box: true });
  });

  test("Accept a draw and show the result to both players", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 2);
    await test.step("Accept a draw and show the result to both players", async () => {
      await confirmControl(whitePage, "Offer Draw");
      await expect(blackPage.getByText("Opponent offers a draw")).toBeVisible();
      await blackPage.getByTitle("Accept Draw", { exact: true }).click();
      for (const page of [whitePage, blackPage]) {
        if (!isMobileLayout) await showTab(page, "History", isMobileLayout);
        await expect(page.getByText(isMobileLayout ? "½-½ Draw agreed" : "Draw agreed", { exact: true })).toBeVisible();
      }
    }, { box: true });
  });
});
