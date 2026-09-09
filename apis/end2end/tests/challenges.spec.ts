import { expect, test } from "./fixtures";
import {
  acceptChallenge,
  cancelChallenge,
  challengeFrom,
  createDirectChallenge,
  createPublicChallenge,
  declineChallenge,
} from "./test_utils/challenges";
import { confirmControl, showControlsIfMobile } from "./test_utils/game_controls";

test.describe("Challenges", () => {
  test.describe.configure({ timeout: 60_000 });

  test("Decline a direct challenge", async ({ players }) => {
    const { userOne: challenger, userTwo: opponent } = players;
    const challenge = challengeFrom(opponent.page, challenger.username);

    await test.step("Create the direct challenge", async () => {
      await createDirectChallenge(challenger.page, opponent.username, "Random");
      await expect(challenge).toBeVisible();
    }, { box: true });

    await test.step("Decline it and confirm removal", async () => {
      await declineChallenge(opponent.page, challenger.username);
      await expect(challenge).toHaveCount(0);
    }, { box: true });
  });

  test("Only the creator can cancel a public challenge", async ({ players }) => {
    const { userOne: challenger, userTwo: opponent } = players;
    const challenge = challengeFrom(opponent.page, challenger.username);

    await test.step("Create the public challenge", async () => {
      await createPublicChallenge(challenger.page, "1+2");
      await expect(challenge).toBeVisible();
    }, { box: true });

    await test.step("Verify the opponent cannot cancel it", async () => {
      await expect(challenge.getByRole("button", { name: "Cancel Challenge" })).toHaveCount(0);
    }, { box: true });

    await test.step("Cancel it and confirm removal", async () => {
      await cancelChallenge(challenger.page, challenger.username);
      await expect(challenge).toHaveCount(0);
    }, { box: true });
  });

  test("Accept a direct challenge and abort the game", async ({
    players,
    isMobileLayout,
  }) => {
    const { userOne: challenger, userTwo: opponent } = players;

    await test.step("Create the direct challenge", async () => {
      await createDirectChallenge(challenger.page, opponent.username, "Random");
    }, { box: true });

    await test.step("Accept the direct challenge", async () => {
      await acceptChallenge({ challenger, opponent });
    }, { box: true });

    await test.step("Abort the game", async () => {
      await showControlsIfMobile(challenger.page, isMobileLayout);
      await expect(challenger.page.getByTitle("Abort")).toBeVisible();
      await confirmControl(challenger.page, "Abort");
    }, { box: true });

    await test.step("Confirm the abort outcome", async () => {
      for (const player of [challenger, opponent]) {
        await expect(player.page.getByRole("alert")).toContainText(`${challenger.username} aborted the game`);
        await expect(player.page).toHaveURL(/\/$/);
      }
    }, { box: true });
  });
});
