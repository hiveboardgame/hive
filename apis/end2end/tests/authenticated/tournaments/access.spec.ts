import { test, expect } from "../../../support/tournaments/fixtures";
import { button, navigate, openTournament } from "../../../support/tournaments/ui";

test.describe("Tournament access and chat", () => {
  test.describe.configure({ timeout: 60_000 });

  test("Show management controls according to the viewer's role", async ({ page, tournamentAdmin: admin, tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [member] });
    await test.step("Keep management controls hidden from visitors and members", async () => {
      for (const viewer of [page, member.page]) {
        await openTournament(viewer, target);
        for (const label of ["Edit Description", "Delete", "Start"]) await expect(button(viewer, label)).toBeHidden();
        await expect(viewer.getByPlaceholder("Invite player", { exact: true })).toBeHidden();
      }
      await expect(button(page, "Join")).toBeDisabled();
      await expect(button(member.page, "Leave")).toBeVisible();
    });
    await test.step("Expose organizer and admin management, then delete through the UI", async () => {
      for (const viewer of [organizer.page, admin.page]) {
        await openTournament(viewer, target);
        for (const label of ["Edit Description", "Delete", "Start"]) await expect(button(viewer, label)).toBeVisible();
        await expect(viewer.getByPlaceholder("Invite player", { exact: true })).toBeVisible();
      }
      await button(admin.page, "Delete").click();
      await expect(admin.page).toHaveURL(/\/tournaments$/);
      // The admin navigates optimistically. The participant redirects only after
      // the server's Deleted update; wait for that UI signal before fetching anew.
      await expect(member.page).toHaveURL(/\/tournaments\/?$/);
      // The list uses OnceResource and can retain a pre-delete snapshot (see TOURNAMENTS.md).
      await navigate(admin.page, "/tournaments/future");
      await expect(admin.page.getByRole("heading", { name: "Tournaments", exact: true })).toBeVisible();
      await expect(admin.page.getByText("Loading tournaments...", { exact: true })).toBeHidden();
      await expect(admin.page.getByText("Error loading tournaments", { exact: true })).toBeHidden();
      await expect(admin.page.getByRole("article").filter({ hasText: target.name })).toHaveCount(0);
    });
    // UI visibility does not establish server-side authorization.
  });

  test("Exchange tournament chat messages and retain them after reload", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [member] });
    await openTournament(organizer.page, target);
    await openTournament(member.page, target);
    await test.step("Exchange messages between organizer and participant", async () => {
      for (const sender of [organizer, member]) {
        const input = sender.page.getByLabel("Chat message", { exact: true });
        await input.fill(`Tournament hello from ${sender.username}`);
        await input.press("Enter");
        for (const viewer of [organizer, member]) await expect(viewer.page.getByRole("log", { name: "Chat messages" }))
          .toContainText(`Tournament hello from ${sender.username}`);
      }
    });
    await test.step("Reload and retain the conversation", async () => {
      await openTournament(member.page, target);
      for (const sender of [organizer, member]) await expect(member.page.getByRole("log", { name: "Chat messages" }))
        .toContainText(`Tournament hello from ${sender.username}`);
    });
  });

  test("Restrict chat for outsiders and after leaving a tournament", async ({ page, tournamentOutsider: outsider, tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { players: [member] });
    await test.step("Show a restriction notice to anonymous and signed-in outsiders", async () => {
      for (const viewer of [page, outsider.page]) {
        await openTournament(viewer, target);
        await expect(viewer.getByLabel("Chat message", { exact: true })).toBeHidden();
        await expect(viewer.getByText(/Only.*(participants|players|members|joined)/i)).toBeVisible();
      }
    });
    await test.step("Remove chat access when the participant leaves", async () => {
      await openTournament(member.page, target);
      await expect(member.page.getByLabel("Chat message", { exact: true })).toBeVisible();
      await button(member.page, "Leave").click();
      await expect(button(member.page, "Join")).toBeVisible();
      await expect(member.page.getByLabel("Chat message", { exact: true })).toBeHidden();
      await openTournament(member.page, target);
      await expect(member.page.getByLabel("Chat message", { exact: true })).toBeHidden();
    });
  });
});
