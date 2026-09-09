import { test, expect } from "../../../support/tournaments/fixtures";
import { description } from "../../../support/tournaments/data";
import { button, invite, navigate, notification, notifications, openTournament, panel, roster, slider, toggle } from "../../../support/tournaments/ui";

// UI journeys only. Seeded states exercise controls; they are not result-correctness checks.
test.describe("Tournament registration", () => {
  test.describe.configure({ timeout: 60_000 });

  test("Browse, search, and open tournaments from public and personal tabs", async ({ tournament: { data, organizer, member } }) => {
    const future = await data.seed(organizer, { players: [member] });
    const ongoing = await data.seed(organizer, { status: "InProgress" });
    const finished = await data.seed(organizer, { status: "Finished" });
    const page = organizer.page;
    await test.step("Browse status tabs and filter by name", async () => {
      await navigate(page, "/tournaments");
      for (const [label, target] of [["Future", future], ["In Progress", ongoing], ["Completed", finished]] as const) {
        await page.getByRole("link", { name: label, exact: true }).click();
        await expect(page.locator("article").filter({ hasText: target.name })).toBeVisible();
        const search = page.getByPlaceholder("Search tournaments by name");
        await search.fill(target.name.toLowerCase());
        await expect(page.locator("article")).toHaveCount(1);
        await search.fill("no such UI tournament");
        await expect(page.locator("article")).toHaveCount(0);
        await search.clear();
      }
    });
    await test.step("Open a hosted tournament and find it in the member's Joined tab", async () => {
      await page.getByRole("link", { name: "Hosting", exact: true }).click();
      await page.locator(`article a[href="/tournament/${future.nanoid}"]`).click();
      await expect(page.getByRole("heading", { name: future.name })).toBeVisible();
      await navigate(member.page, "/tournaments");
      await member.page.getByRole("link", { name: "Joined", exact: true }).click();
      await expect(member.page.locator("article").filter({ hasText: future.name })).toBeVisible();
      await expect(member.page.locator("article").filter({ hasText: ongoing.name })).toHaveCount(0);
    });
  });

  test("Configure a tournament and persist the selected settings", async ({ tournament: { data, organizer } }) => {
    const planned = await data.plan(organizer);
    const page = organizer.page;
    await navigate(page, "/tournaments/create");
    await test.step("Configure entry, format, time, and start settings", async () => {
      await page.getByLabel("Tournament name", { exact: true }).fill(planned.name);
      await page.getByPlaceholder("At least a 50 character description. Markdown supported.").fill(description);
      await expect(page.locator('select[name="Tournament Mode"] option')).toHaveText([
        "Double round robin", "Quadruple round robin", "Sextuple round robin",
      ]);
      await page.getByRole("combobox", { name: "Mode", exact: true }).selectOption("QuadrupleRoundRobin");
      // Existing input names are swapped relative to the visible Min/Max players labels.
      await slider(page, "Seats", 2);
      await slider(page, "Min Seats", 6);
      await toggle(page, "Invite Only", true);
      await toggle(page, "Manual start", false);
      const future = new Date(Date.now() + 2 * 86400_000);
      await page.getByLabel("Choose a start time").fill(`${future.getFullYear()}-${String(future.getMonth() + 1).padStart(2, "0")}-${String(future.getDate()).padStart(2, "0")}T12:00`);
      await toggle(page, "Manual start", true);
      await button(page, "Correspondence").click();
      await expect(page.getByText("Days per move", { exact: true })).toBeVisible();
      await page.getByText("Total time each", { exact: true }).click();
      await expect(page.getByText("Fixed round duration", { exact: true })).toBeHidden();
      await button(page, "Real time").click();
      await toggle(page, "Fixed round duration", true);
      await slider(page, "Round duration in days", 3);
      await slider(page, "Min rating", 400);
      await slider(page, "Max rating", 2600);
      await expect(page.getByText("Min Rating: Any/ Max Rating: Any", { exact: true })).toBeVisible();
    });
    await test.step("Create and reload the tournament", async () => {
      await button(page, "Create Tournament").click();
      await expect(page).toHaveURL(/\/tournaments$/);
      const card = page.getByRole("article").filter({ hasText: planned.name });
      // Create navigates before its WebSocket acknowledgement. The OnceResource
      // list can stay stale; retry only a fresh UI read, never the Create action.
      await expect(async () => {
        await navigate(page, "/tournaments/hosting");
        await expect(card).toBeVisible({ timeout: 1000 });
      }).toPass({ timeout: 15_000 });
      await card.getByRole("link").click();
      await expect(page.getByRole("heading", { name: planned.name })).toBeVisible();
      await data.createdThroughUI(planned);
      await openTournament(page, planned);
      await expect(panel(page, "Tournament Info")).toContainText("Quadruple round robin");
      await expect(page.getByText(description, { exact: true })).toBeVisible();
      // Readback checks form persistence, not pairing or scoring correctness.
      const row = (await data.client.query("SELECT seats,min_seats,invite_only,band_lower,band_upper,round_duration,start_mode FROM tournaments WHERE id=$1", [planned.id])).rows[0];
      expect(row).toEqual({ seats: 6, min_seats: 2, invite_only: true, band_lower: null, band_upper: null, round_duration: 3, start_mode: "Manual" });
    });
  });

  test("Validate required fields and switch between Markdown preview and editing", async ({ tournament: { organizer } }) => {
    const page = organizer.page;
    await navigate(page, "/tournaments/create");
    await test.step("Enforce text boundaries", async () => {
      const name = page.getByLabel("Tournament name", { exact: true });
      const body = page.getByPlaceholder("At least a 50 character description. Markdown supported.");
      const create = button(page, "Create Tournament");
      await expect(create).toBeDisabled();
      await name.fill("abc");
      await body.fill("x".repeat(50));
      await expect(create).toBeDisabled();
      await name.fill("abcd");
      await body.fill("x".repeat(49));
      await expect(create).toBeDisabled();
      await body.fill("x".repeat(50));
      await expect(create).toBeEnabled();
      await expect(name).toHaveAttribute("maxlength", "50");
      await expect(body).toHaveAttribute("maxlength", "2000");
    });
    await test.step("Preview Markdown without losing the editable text", async () => {
      const markdown = `**UI preview** ${description}`;
      await page.getByPlaceholder("At least a 50 character description. Markdown supported.").fill(markdown);
      await button(page, "Preview").click();
      await expect(page.locator("strong").filter({ hasText: "UI preview" })).toBeVisible();
      await button(page, "Edit").click();
      await expect(page.getByPlaceholder("At least a 50 character description. Markdown supported.")).toHaveValue(markdown);
    });
  });

  test("Preview, cancel, and save an organizer description edit", async ({ tournament: { data, organizer } }) => {
    const target = await data.seed(organizer);
    const page = organizer.page;
    await openTournament(page, target);
    await test.step("Cancel an edit and retain the original description", async () => {
      await button(page, "Edit Description").click();
      await page.getByPlaceholder("At least 50 characters required").fill("short");
      await expect(button(page, "Update Description")).toBeDisabled();
      await button(page, "Cancel").click();
      await expect(page.getByText(description, { exact: true })).toBeVisible();
    });
    await test.step("Preview and persist an edited description", async () => {
      await button(page, "Edit Description").click();
      await page.getByPlaceholder("At least 50 characters required").fill(`**Updated UI description** ${description}`);
      await button(page, "Preview").click();
      await expect(page.locator("strong").filter({ hasText: "Updated UI description" })).toBeVisible();
      await button(page, "Edit").click();
      await button(page, "Update Description").click();
      await expect(button(page, "Edit Description")).toBeVisible();
      await openTournament(page, target);
      await expect(page.locator("strong").filter({ hasText: "Updated UI description" })).toBeVisible();
    });
  });

  test("Join and leave with roster updates on both clients", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer);
    await openTournament(organizer.page, target);
    await openTournament(member.page, target);
    await test.step("Join and update the organizer's roster", async () => {
      await button(member.page, "Join").click();
      await expect(button(member.page, "Leave")).toBeVisible();
      await expect(roster(organizer.page, "Players")).toContainText(member.username);
      await openTournament(member.page, target);
      await expect(button(member.page, "Leave")).toBeVisible();
    });
    await test.step("Leave and remove the participant from the roster", async () => {
      await button(member.page, "Leave").click();
      await expect(button(member.page, "Join")).toBeEnabled();
      await expect(roster(organizer.page, "Players")).toHaveCount(0);
    });
  });

  test("Disable entry for full, invite-only, and rating-restricted tournaments", async ({ tournament: { data, organizer, member } }) => {
    const other = (await data.accounts(1))[0];
    for (const [reason, options] of [
      ["full", { seats: 2, players: [organizer, other] }],
      ["invite only", { inviteOnly: true }],
      ["lower rating boundary", { lower: 1500 }],
      ["upper rating boundary", { upper: 1500 }],
    ] as const) {
      await test.step(`Prevent entry when ${reason}`, async () => {
        const target = await data.seed(organizer, { ...options, players: "players" in options ? [...options.players] : [] });
        await openTournament(member.page, target);
        await expect(button(member.page, "Join")).toBeDisabled();
      });
    }
  });

  test("Send and accept an invitation from notifications", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer, { inviteOnly: true });
    await openTournament(organizer.page, target);
    await openTournament(member.page, target);
    await test.step("Invite the member and show an actionable notification", async () => {
      await invite(organizer.page, member.username);
      await notifications(member.page);
      await expect(notification(member.page, target.name)).toBeVisible();
      await notification(member.page, target.name).getByRole("button", { name: "Accept Invitation" }).click();
    });
    await test.step("Replace the invitation with membership on both clients", async () => {
      await expect(notification(member.page, target.name)).toHaveCount(0);
      await expect(roster(organizer.page, "Players")).toContainText(member.username);
      await expect(roster(organizer.page, "Invitees")).toHaveCount(0);
      await openTournament(member.page, target);
      await expect(button(member.page, "Leave")).toBeVisible();
    });
  });

  test("Decline and retract invitations, then remove a joined player", async ({ tournament: { data, organizer, member } }) => {
    const target = await data.seed(organizer);
    await openTournament(organizer.page, target);
    await openTournament(member.page, target);
    await test.step("Decline an invitation", async () => {
      await invite(organizer.page, member.username);
      await notifications(member.page);
      await notification(member.page, target.name).getByRole("button", { name: "Decline Invitation" }).click();
      await expect(roster(organizer.page, "Invitees")).toHaveCount(0);
      await expect(notification(member.page, target.name)).toHaveCount(0);
    });
    await test.step("Retract a new invitation", async () => {
      await invite(organizer.page, member.username);
      await expect(notification(member.page, target.name)).toBeVisible();
      await roster(organizer.page, "Invitees").getByRole("button", { name: "Remove from tournament" }).click();
      await expect(notification(member.page, target.name)).toHaveCount(0);
    });
    await test.step("Remove a joined participant", async () => {
      await openTournament(member.page, target);
      await button(member.page, "Join").click();
      await expect(roster(organizer.page, "Players")).toContainText(member.username);
      await roster(organizer.page, "Players").getByRole("button", { name: "Remove from tournament" }).click();
      await expect(button(member.page, "Join")).toBeVisible();
      await expect(roster(organizer.page, "Players")).toHaveCount(0);
    });
  });
});
