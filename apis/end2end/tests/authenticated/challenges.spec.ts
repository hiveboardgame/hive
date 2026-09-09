import { expect, test } from "../../support/fixtures";
import {
  acceptChallenge,
  cancelChallenge,
  challengeFrom,
  createDirectChallenge,
  createPublicChallenge,
  declineChallenge,
  openChallengeHome,
} from "../../support/game/challenges";

test.describe("Challenges", () => {
  test.describe.configure({ timeout: 60_000 });

  test("Decline a direct challenge", async ({ players }) => {
    const { userOne: challenger, userTwo: opponent } = players;
    const challenge = challengeFrom(opponent.page, challenger.username);

    await test.step("Create the direct challenge", async () => {
      await createDirectChallenge(challenger, opponent, "Random");
      await expect(challenge).toBeVisible();
    }, { box: true });

    await test.step("Decline it and confirm removal", async () => {
      await declineChallenge(opponent, challenger);
      await expect(challenge).toHaveCount(0);
    }, { box: true });
  });

  test.describe("Public challenges", () => {
    test.use({ publicChallenge: true });
    test("Only the creator can cancel a public challenge", async ({ players }) => {
      const { userOne: challenger, userTwo: opponent } = players;
      const challenge = challengeFrom(opponent.page, challenger.username);

      await test.step("Create the public challenge", async () => {
        await openChallengeHome(opponent);
        await createPublicChallenge(challenger);
        await expect(challenge).toBeVisible();
      }, { box: true });

      await test.step("Verify the opponent cannot cancel it", async () => {
        await expect(challenge.getByRole("button", { name: "Cancel Challenge" })).toHaveCount(0);
      }, { box: true });

      await test.step("Cancel it and confirm removal", async () => {
        await cancelChallenge(challenger);
        await expect(challenge).toHaveCount(0);
      }, { box: true });
    });

  });

  test("Accept a direct challenge", async ({ players }) => {
    const { userOne: challenger, userTwo: opponent } = players;

    await test.step("Create the direct challenge", async () => {
      await createDirectChallenge(challenger, opponent, "Random");
    }, { box: true });

    await test.step("Accept the direct challenge", async () => {
      await acceptChallenge({ challenger, opponent });
    }, { box: true });
  });
});
