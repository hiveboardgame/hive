import { expect, test } from "../../support/fixtures";
import { expectPieceAt } from "../../support/game/board";
import { confirmControl } from "../../support/game/controls";
import { reviewHistory } from "../../support/game/panels";
import { startGame } from "../../support/game/setup";
import { boardPositions, playOpening } from "../../support/game/opening";

test.describe("Takebacks", () => {
  test.describe.configure({ timeout: 90_000 });

  test("Reject a takeback without changing the board", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 7);
    await test.step("Reject a takeback without changing the board", async () => {
      await confirmControl(blackPage, "Request Takeback");
      await expect(whitePage.getByText("Opponent wants a takeback")).toBeVisible();
      await whitePage.getByTitle("Reject Takeback", { exact: true }).click();
      await expect(whitePage.getByText("Opponent wants a takeback")).toBeHidden();
      await expect(blackPage.getByTitle("Request Takeback", { exact: true })).toBeEnabled();
      for (const page of [whitePage, blackPage]) {
        await expectPieceAt(page, "White Beetle 1", boardPositions.beetleMove, 1);
      }
      if (!isMobileLayout) await reviewHistory(whitePage, 7);
    }, { box: true });
  });

  test("Accept a takeback and restore the beetle beside the queen", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 7);
    await test.step("Accept a takeback and restore the beetle beside the queen", async () => {
      await confirmControl(blackPage, "Request Takeback");
      await expect(whitePage.getByText("Opponent wants a takeback")).toBeVisible();
      await whitePage.getByTitle("Accept Takeback", { exact: true }).click();
      for (const page of [whitePage, blackPage]) {
        await expectPieceAt(page, "White Beetle 1", boardPositions.whiteBeetle);
        await expectPieceAt(page, "Black Grasshopper 1", boardPositions.blackGrasshopper);
      }
      if (!isMobileLayout) await reviewHistory(whitePage, 6, [[5, "bG1"]]);
    }, { box: true });
  });
});
