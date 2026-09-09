import { expect, test } from "../../support/fixtures";
import { confirmControl } from "../../support/game/controls";
import { startGame } from "../../support/game/setup";
import { playOpening } from "../../support/game/opening";

test.describe("Game endings", () => {
  test.describe.configure({ timeout: 90_000 });

  test("Abort an unstarted game", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await test.step("Abort an unstarted game", async () => {
      await confirmControl(whitePage, "Abort");
      for (const page of [whitePage, blackPage]) {
        await expect(page.getByRole("alert")).toContainText(`${players.userOne.username} aborted the game`);
        await expect(page).toHaveURL(/\/$/);
      }
    }, { box: true });
  });

  test("Resign after the opening", async ({ players, isMobileLayout }) => {
    const { page: whitePage } = players.userOne;
    const { page: blackPage } = players.userTwo;
    await startGame({ white: players.userOne, black: players.userTwo, isMobileLayout });
    await playOpening(players, 2);
    await test.step("Resign after the opening", async () => {
      await confirmControl(whitePage, "Resign");
      for (const page of [whitePage, blackPage]) {
        await expect(page.getByText(`${players.userTwo.username} won by resignation`, { exact: true })).toBeVisible();
      }
    }, { box: true });
  });
});
