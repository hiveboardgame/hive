import { expect, test } from "../../support/fixtures";
import { boardPiece } from "../../support/game/board";
import { startGame } from "../../support/game/setup";
import { playOpening } from "../../support/game/opening";

test.describe("Turn rules", () => {
  test.describe.configure({ timeout: 90_000 });

  test("Cannot move or place during the opponent's turn", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 5);
    await test.step("Cannot move or place during the opponent's turn", async () => {
      await boardPiece(whitePage, "White Beetle 1").click();
      await expect(whitePage.getByRole("button", { name: /^Move to board position / })).toHaveCount(0);
      await whitePage.getByRole("button", { name: "White Ant 2 in reserve", exact: true }).click();
      await expect(whitePage.getByRole("button", { name: /^Move to board position / })).toHaveCount(0);
    }, { box: true });
  });
});
