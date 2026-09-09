import { test, expect } from "../../../support/tournaments/fixtures";
import { bulkForfeit, button, gameRow, navigate, notification, notifications, openTournament, start, unplayed, unplayedGameLink } from "../../../support/tournaments/ui";

test.describe("Tournament game controls", () => {
  test.describe.configure({ timeout: 60_000 });

  test("Start and finish a tournament with status feedback and notifications", async ({ tournament: { data, organizer, member } }) => {
    test.setTimeout(90_000);
    const target = await data.seed(organizer, { players: [organizer] });
    await openTournament(organizer.page, target);
    await openTournament(member.page, target);
    await test.step("Enable Start once the minimum roster is met", async () => {
      await expect(button(organizer.page, "Start")).toBeDisabled();
      await button(member.page, "Join").click();
      await expect(button(organizer.page, "Start")).toBeEnabled();
      await start(organizer.page);
      await expect(button(organizer.page, "Finish")).toBeDisabled();
      await expect(button(member.page, "Leave")).toBeHidden();
      await notifications(member.page);
      await expect(notification(member.page, target.name)).toContainText("Started");
      await notification(member.page, target.name).getByRole("button", { name: "Dismiss" }).click();
    });
    await test.step("Resolve the pending games through the UI and finish", async () => {
      // Results only unlock Finish; points and standings order are intentionally not asserted.
      await bulkForfeit(organizer.page);
      await expect(button(organizer.page, "Finish")).toBeEnabled();
      await button(organizer.page, "Finish").click();
      await expect(button(organizer.page, "Finish")).toBeHidden();
      await expect(notification(member.page, target.name)).toContainText("Finished");
      await openTournament(organizer.page, target);
      await expect(organizer.page.locator("main")).toContainText("Finished");
      await navigate(organizer.page, "/tournaments/finished");
      await expect(organizer.page.locator("article").filter({ hasText: target.name })).toBeVisible();
    });
  });

  test("Propose, accept, and cancel an agreed game date", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [organizer, member] });
    await openTournament(organizer.page, target);
    await start(organizer.page);
    const [game] = await data.games(target);
    await openTournament(member.page, target);
    await test.step("Propose a date and accept it on the other client", async () => {
      const own = await gameRow(organizer.page, game, true);
      // Each game repeats id=start-time, so scope the date control to this game's card.
      const future = new Date(Date.now() + 86400_000);
      const local = `${future.getFullYear()}-${String(future.getMonth() + 1).padStart(2, "0")}-${String(future.getDate()).padStart(2, "0")}T12:00`;
      await own.locator('input[type="datetime-local"]').fill(local);
      await button(own, "Propose Date").click();
      const other = await gameRow(member.page, game, true);
      await expect(other).toContainText("Proposed");
      await button(other, "Accept").click();
      await expect(own).toContainText("To play");
      await expect(other).toContainText("To play");
      await expect(await gameRow(organizer.page, game)).toContainText("Scheduled at");
    });
    await test.step("Reload the agreement and cancel it", async () => {
      await openTournament(member.page, target);
      const other = await gameRow(member.page, game, true);
      await expect(other).toContainText("To play");
      await button(other, "Cancel").click();
      await expect(other).not.toContainText("To play");
      await expect(await gameRow(organizer.page, game, true)).not.toContainText("To play");
      await expect(await gameRow(organizer.page, game)).toContainText("Not yet scheduled");
    });
  });

  test("Reject a proposed game date on the other client", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [organizer, member] });
    await openTournament(organizer.page, target);
    await start(organizer.page);
    const [game] = await data.games(target);
    await openTournament(member.page, target);
    await test.step("Propose and reject a date", async () => {
      const own = await gameRow(organizer.page, game, true);
      await button(own, "Propose Date").click();
      const other = await gameRow(member.page, game, true);
      await expect(other).toContainText("Proposed");
      await expect(button(own, "Accept")).toHaveCount(0);
      await button(other, "Reject").click();
      await expect(own).not.toContainText("Proposed");
      await expect(other).not.toContainText("Proposed");
      await expect(button(own, "Propose Date")).toBeEnabled();
    });
  });

  test("Dismiss a readiness popup, then accept another game and enter the board", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [organizer, member] });
    await openTournament(organizer.page, target);
    await start(organizer.page);
    const games = await data.games(target);
    await openTournament(member.page, target);
    await test.step("Follow a game link and dismiss its readiness popup", async () => {
      await (await gameRow(organizer.page, games[0])).getByRole("link", { name: "Join Game", exact: true }).click();
      await expect(button(organizer.page, "Ready")).toBeVisible();
      await button(organizer.page, "Ready").click();
      await expect(member.page.getByRole("heading", { name: "Tournament Game Ready!" })).toBeVisible();
      await button(member.page, "Close").click();
      await expect(member.page.getByRole("heading", { name: "Tournament Game Ready!" })).toBeHidden();
      await expect(button(organizer.page, "Ready")).toBeVisible();
    });
    await test.step("Accept another readiness request and show a playable board", async () => {
      await openTournament(organizer.page, target);
      await (await gameRow(organizer.page, games[1])).getByRole("link", { name: "Join Game", exact: true }).click();
      await button(organizer.page, "Ready").click();
      await button(member.page, "Accept Game").click();
      for (const page of [organizer.page, member.page]) {
        await expect(page).toHaveURL(new RegExp(`/game/${games[1].nanoid}$`));
        await expect(button(page, "Ready")).toBeHidden();
        await expect(page.getByRole("button", { name: /in reserve$/ }).first()).toBeVisible();
      }
    });
  });

  test("Cancel, apply, and reset an individual adjudication", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [organizer, member] });
    await openTournament(organizer.page, target);
    await start(organizer.page);
    const [game] = await data.games(target);
    await openTournament(member.page, target);
    await test.step("Inspect the result menu and cancel without changing the game", async () => {
      const row = await gameRow(organizer.page, game);
      await button(row, "Adjudicate").click();
      // Opening the menu removes the game's link. Locate the visible menu by its controls.
      const menu = (await unplayed(organizer.page)).locator(".ui-card-row").filter({ has: button(organizer.page, "White won") });
      for (const label of ["White won", "Black won", "Double forfeit", "Draw"]) await expect(button(menu, label)).toBeVisible();
      await button(menu, "Cancel").click();
      await expect(button(row, "Adjudicate")).toBeVisible();
      await expect(row.getByRole("link", { name: "Join Game", exact: true })).toBeVisible();
    });
    await test.step("Apply a representative result and reset it", async () => {
      let row = await gameRow(organizer.page, game);
      await button(row, "Adjudicate").click();
      await button(await unplayed(organizer.page), "White won").click();
      // A tournament update replaces its details panels. Wait for the new link text
      // in the collapsed panel before opening it, rather than racing its replacement.
      for (const page of [organizer.page, member.page]) await expect(unplayedGameLink(page, game)).toHaveText("View Game");
      row = await gameRow(organizer.page, game);
      await expect(row.getByRole("link", { name: "View Game", exact: true })).toBeVisible();
      await expect((await gameRow(member.page, game)).getByRole("link", { name: "View Game", exact: true })).toBeVisible();
      await openTournament(organizer.page, target);
      row = await gameRow(organizer.page, game);
      await button(row, "Adjudicate").click();
      await button(await unplayed(organizer.page), "Delete").click();
      for (const page of [organizer.page, member.page]) await expect(unplayedGameLink(page, game)).toHaveText("Join Game");
      await expect((await gameRow(organizer.page, game)).getByRole("link", { name: "Join Game", exact: true })).toBeVisible();
      await expect((await gameRow(member.page, game)).getByRole("link", { name: "Join Game", exact: true })).toBeVisible();
    });
  });

  test("Confirm, cancel, and undo bulk adjudications", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [organizer, member] });
    const page = organizer.page;
    await openTournament(page, target);
    await start(page);
    await test.step("Cancel the bulk confirmation, then apply it", async () => {
      await button(page, "Double forfeit unstarted games").click();
      await expect(page.getByText("This will adjudicate every unstarted tournament game as a double forfeit.", { exact: true })).toBeVisible();
      await button(page, "Cancel").click();
      await expect(button(page, "Finish")).toBeDisabled();
      await bulkForfeit(page);
      await expect(button(page, "Finish")).toBeEnabled();
    });
    await test.step("Cancel undo, then confirm it and restore pending games", async () => {
      await button(page, "Undo adjudications").click();
      await button(page, "Cancel").click();
      await expect(button(page, "Finish")).toBeEnabled();
      await button(page, "Undo adjudications").click();
      await button(page, "Confirm undo").click();
      await expect(button(page, "Finish")).toBeDisabled();
      await expect(button(page, "Double forfeit unstarted games")).toBeVisible();
      await expect(button(page, "Undo adjudications")).toBeHidden();
      await openTournament(member.page, target);
      await expect((await unplayed(member.page)).getByRole("link", { name: "Join Game", exact: true }).first()).toBeVisible();
    });
  });

  test("Expose Swiss creation to admins and gate next-round controls", async ({ tournamentAdmin: admin, tournament: { data, organizer, member } }) => {
    await data.requireBye();
    await test.step("Show the Swiss format only to the privileged account", async () => {
      await navigate(organizer.page, "/tournaments/create");
      await expect(organizer.page.locator('option[value="DoubleSwiss"]')).toHaveCount(0);
      await navigate(admin.page, "/tournaments/create");
      await expect(admin.page.locator('option[value="DoubleSwiss"]')).toHaveCount(1);
      await admin.page.getByRole("combobox", { name: "Mode", exact: true }).selectOption("DoubleSwiss");
    });
    const target = await data.seed(admin, { mode: "DoubleSwiss", players: [organizer, member] });
    await openTournament(admin.page, target);
    await start(admin.page);
    await test.step("Enable the next round after resolving pending games", async () => {
      const next = button(admin.page, "Progress to next round");
      await expect(next).toBeDisabled();
      await bulkForfeit(admin.page);
      await expect(next).toBeEnabled();
      await next.click();
      await expect(next).toBeDisabled();
      await expect(button(admin.page, "Double forfeit unstarted games")).toBeVisible();
      // Deliberately no game-count, opponent, bye-score, or ranking assertions.
    });
  });
});
