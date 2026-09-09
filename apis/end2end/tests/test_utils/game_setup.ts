import { test } from "playwright/test";
import { acceptChallenge, createDirectChallenge } from "./challenges";
import { showControlsIfMobile } from "./game_controls";
import type { Player } from "./player";

export async function startGame({ white, black, isMobileLayout }: {
  white: Player;
  black: Player;
  isMobileLayout: boolean;
}) {
  await test.step(`Create a white challenge for ${black.username}`, async () => {
    await createDirectChallenge(white.page, black.username, "White");
  }, { box: true });

  await test.step(`Accept ${white.username}'s challenge as ${black.username}`, async () => {
    await acceptChallenge({ challenger: white, opponent: black });
  }, { box: true });

  await test.step("Prepare both players' game controls", async () => {
    await showControlsIfMobile(white.page, isMobileLayout);
    await showControlsIfMobile(black.page, isMobileLayout);
  }, { box: true });
}
