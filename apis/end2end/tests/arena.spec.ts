import { expect, test } from "../support/fixtures";
import { reloadHydrated } from "../support/browser/hydration";
import { expectPieceAt, placePiece } from "../support/game/board";
import { confirmControl, showControlsIfMobile } from "../support/game/controls";
import { boardPositions } from "../support/game/opening";

// Two independent clients and deliberate reloads include fresh browser hydration.
test.describe.configure({ timeout: 90_000 });

test("arena intent survives reload and a live assignment can be played and revisited @journey", async ({ data, player, isMobileLayout: mobile }) => {
  const { users, tournamentId } = await data.arena();
  const first = await player(users[0]);
  const second = await player(users[1]);
  const tournamentPath = `/tournament/${tournamentId}`;
  await first.page.locator(`a[href="${tournamentPath}"]`).click();
  await first.page.getByRole("button", { name: "Join", exact: true }).click();
  await first.page.getByRole("dialog", { name: "Join tournament", exact: true })
    .getByRole("button", { name: "Join tournament", exact: true }).click();
  await first.page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(first.page.getByRole("button", { name: "Resume", exact: true })).toBeVisible();
  await reloadHydrated(first.page);
  await first.page.getByRole("button", { name: "Resume", exact: true }).click();
  await expect(first.page.getByRole("button", { name: "Pause", exact: true })).toBeVisible();

  await second.page.locator(`a[href="${tournamentPath}"]`).click();
  await second.page.getByRole("button", { name: "Join", exact: true }).click();
  await second.page.getByRole("dialog", { name: "Join tournament", exact: true })
    .getByRole("button", { name: "Join tournament", exact: true }).click();
  await expect(first.page).toHaveURL(/\/game\/[^/?]+$/);
  await expect(second.page).toHaveURL(first.page.url());
  const gamePath = new URL(first.page.url()).pathname;

  const players = [first, second];
  const whiteIdentity = first.page.locator('[data-game-player-color="w"]:visible')
    .getByRole("link");
  await expect(whiteIdentity).toHaveCount(1);
  const whiteUsername = (await whiteIdentity.innerText()).trim();
  const white = players.find(participant => participant.username === whiteUsername)!;
  const black = players.find(participant => participant !== white)!;
  expect(white, "The assigned white player is one of this arena's participants").toBeDefined();
  await placePiece(white.page, "White Ant 1", boardPositions.openingAnt);
  await expectPieceAt(black.page, "White Ant 1", boardPositions.openingAnt);
  await placePiece(black.page, "Black Ant 1", boardPositions.blackAnt);
  await expectPieceAt(white.page, "Black Ant 1", boardPositions.blackAnt);

  await showControlsIfMobile(first.page, mobile);
  await confirmControl(first.page, "Resign");
  for (const participant of players) {
    await expect(participant.page.getByRole("button", { name: "Back to arena", exact: true })).toBeVisible();
  }
  await first.page.getByRole("button", { name: "Back to arena", exact: true }).click();
  await expect(first.page).toHaveURL(new RegExp(`${tournamentPath}$`));
  await first.page.locator(`[data-tournament-player-id="${first.id}"]`).click();
  await expect(first.page.locator(`a[href="${gamePath}"]`)).toBeVisible();
});
