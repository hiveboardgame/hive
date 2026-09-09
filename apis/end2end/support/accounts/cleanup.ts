import { expect } from "@playwright/test";
import type { AccountReservation, AccountState } from "./reservation";
import { confirmControl, showControlsIfMobile } from "../game/controls";
import type { Player } from "../browser/player";

export async function cleanUpAccounts(
  reservation: AccountReservation,
  players: readonly Player[],
  isMobileLayout: boolean,
) {
  const ids = new Set(reservation.pool.map(account => account.id));
  const read = async () => {
    const state = await reservation.state();
    if (state.challenges.some(row => !ids.has(row.challenger_id)
        || (row.opponent_id !== null && !ids.has(row.opponent_id)))
      || state.games.some(row => !ids.has(row.white_id) || !ids.has(row.black_id)
        || row.tournament_id !== null)) {
      throw new Error("Reserved accounts have state involving non-test users or a tournament; cleanup will not modify it.");
    }
    return state;
  };
  const remaining = (state: AccountState) => state.challenges.length + state.games.length;
  let state = await read();
  while (remaining(state)) {
    const challenge = state.challenges[0];
    const game = state.games[0];
    const participants = challenge
      ? [challenge.challenger_id, challenge.opponent_id] : [game.white_id, game.black_id];
    const player = players.find(player => participants.includes(player.id));
    if (!player) throw new Error("No reserved player can clean up this account state.");
    const before = remaining(state);
    const stillPresent = async () => {
      state = await read();
      return challenge
        ? state.challenges.some(row => row.nanoid === challenge.nanoid)
        : state.games.some(row => row.nanoid === game.nanoid);
    };
    reservation.assertHeld();
    try {
      // Documents for direct challenges can render unauthenticated SSR pages.
      await player.page.goto(challenge ? "/" : `/game/${game.nanoid}`);
      if (!await stillPresent()) continue;
      reservation.assertHeld();
      if (challenge) {
        const name = challenge.challenger_id === player.id ? "Cancel Challenge" : "Decline Challenge";
        // Rows can group several challenges. Only this user's own actions are visible.
        await player.page.getByRole("row").getByRole("button", { name, exact: true })
          .first().click({ timeout: 5_000 });
      } else {
        await showControlsIfMobile(player.page, isMobileLayout);
        if (!await stillPresent()) continue;
        reservation.assertHeld();
        const current = state.games.find(row => row.nanoid === game.nanoid)!;
        await confirmControl(player.page, current.turn < 2 ? "Abort" : "Resign");
      }
    } catch (error) {
      // Another reserved participant may have resolved the state during a click.
      if (await stillPresent()) throw error;
    }
    await expect.poll(async () => {
      state = await read();
      return remaining(state);
    }, { message: "The reserved user's outstanding state was removed" }).toBeLessThan(before);
  }
}
