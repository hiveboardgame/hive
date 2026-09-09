import { expect, test } from "../../support/fixtures";
import { boardPiece, expectPieceAt } from "../../support/game/board";
import { historyControl, reviewHistory, showTab } from "../../support/game/panels";
import { startGame } from "../../support/game/setup";
import { boardPositions, playOpening } from "../../support/game/opening";

test.describe("Board history", () => {
  test.describe.configure({ timeout: 90_000 });

  test("Review the two-move opening", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 2);
    await test.step("Review the two-move opening", async () => {
      const piecesOnBoard = whitePage.getByRole("button", { name: / on board$/ });

      if (isMobileLayout) {
        await expectPieceAt(whitePage, "Black Ant 1", boardPositions.blackAnt);
        await historyControl(whitePage, "Previous").click();
        await expect(piecesOnBoard).toHaveCount(1);
        await expectPieceAt(whitePage, "White Ant 1", boardPositions.openingAnt);
        await expect(boardPiece(whitePage, "Black Ant 1")).toHaveCount(0);
        await expect(historyControl(whitePage, "Previous")).toBeDisabled();
        await historyControl(whitePage, "Next").click();
        await expectPieceAt(whitePage, "Black Ant 1", boardPositions.blackAnt);
        await expect(piecesOnBoard).toHaveCount(2);
        await expect(historyControl(whitePage, "Next")).toBeDisabled();
      } else {
        await reviewHistory(whitePage, 2);
      }
      await showTab(whitePage, "Game", isMobileLayout);
    }, { box: true });
  });

  test("Review seven moves and restore the stacked beetle", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 7);
    await test.step("Review seven moves and restore the stacked beetle", async () => {
      const piecesOnBoard = whitePage.getByRole("button", { name: / on board$/ });

      if (isMobileLayout) {
        await test.step("Step back and see the beetle return beside the queen", async () => {
          await expectPieceAt(whitePage, "White Beetle 1", boardPositions.beetleMove, 1);
          await historyControl(whitePage, "Previous").click();
          await expectPieceAt(whitePage, "White Beetle 1", boardPositions.whiteBeetle);
          await expectPieceAt(whitePage, "White Queen", boardPositions.whiteQueen);
        }, { box: true });

        await test.step("Jump to the first move and advance the board one move", async () => {
          await historyControl(whitePage, "First").click();
          await expect(piecesOnBoard).toHaveCount(1);
          await expectPieceAt(whitePage, "White Ant 1", boardPositions.openingAnt);
          await expect(historyControl(whitePage, "First")).toBeDisabled();
          await historyControl(whitePage, "Next").click();
          await expect(piecesOnBoard).toHaveCount(2);
          await expectPieceAt(whitePage, "Black Ant 1", boardPositions.blackAnt);
        }, { box: true });

        await test.step("Return to the latest board with the beetle on the queen", async () => {
          await historyControl(whitePage, "Last").click();
          await expectPieceAt(whitePage, "White Beetle 1", boardPositions.beetleMove, 1);
          await expectPieceAt(whitePage, "Black Grasshopper 1", boardPositions.blackGrasshopper);
          await expect(historyControl(whitePage, "Last")).toBeDisabled();
        }, { box: true });
      } else {
        await reviewHistory(whitePage, 7, [
          [3, "bQ"],
          [6, "wB1"],
        ]);
      }
      await showTab(whitePage, "Game", isMobileLayout);
    }, { box: true });
  });
});
