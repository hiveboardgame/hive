import { expect, type Locator, type Page } from "@playwright/test";
import { expectHydrated } from "../browser/hydration";
import type { Tournament } from "./data";

export async function navigate(page: Page, url: string) {
  await page.goto(url);
  await expectHydrated(page);
}
export async function openTournament(page: Page, tournament: Tournament) {
  await navigate(page, `/tournament/${tournament.nanoid}`);
  await expect(page.getByRole("heading", { name: tournament.name, exact: true })).toBeVisible();
}
export function panel(page: Page, title: string) {
  return page.locator("section").filter({ has: page.getByRole("heading", { name: title, exact: true }) }).last();
}
export function roster(page: Page, title: "Players" | "Invitees") {
  return page.locator("div.ui-setting-group").filter({ has: page.getByText(title, { exact: true }) });
}
export function button(page: Page | Locator, name: string) { return page.getByRole("button", { name, exact: true }); }
export async function toggle(page: Page, text: string, checked: boolean) {
  // SimpleSwitch has no accessible name: scope its visible label to the adjacent switch.
  const row = page.locator("div").filter({ has: page.getByText(text, { exact: true }) })
    .filter({ has: page.locator('input[type="checkbox"]') }).last();
  const input = row.locator('input[type="checkbox"]');
  if (await input.isChecked() !== checked) await row.locator("label").click();
  await expect(input).toBeChecked({ checked });
}
export async function slider(page: Page, name: string, value: number) {
  const input = page.locator(`input[type="range"][name="${name}"]`);
  await input.focus();
  await input.press("Home");
  const min = Number(await input.getAttribute("min"));
  const step = Number(await input.getAttribute("step"));
  for (let n = min; n < value; n += step) await input.press("ArrowRight");
  await expect(input).toHaveValue(String(value));
}
export async function details(page: Page, name: string | RegExp) {
  const summary = page.locator("summary").filter({ hasText: name });
  const container = summary.locator("..");
  if (await container.getAttribute("open") === null) await summary.click();
  await expect(container).toHaveAttribute("open", "");
  return container;
}
export async function unplayed(page: Page) { return details(page, /^\d+ unplayed games?$/); }
export function unplayedGameLink(page: Page, game: { nanoid: string }) {
  return page.locator("details").filter({ has: page.locator("summary").filter({ hasText: /^\d+ unplayed games?$/ }) })
    .locator(`a[href="/game/${game.nanoid}"]`);
}
export async function gameRow(page: Page, game: { nanoid: string }, schedules = false) {
  const section = schedules ? await details(page, "My Schedules") : await unplayed(page);
  return section.locator(".ui-card-row").filter({ has: page.locator(`a[href="/game/${game.nanoid}"]`) });
}
export async function invite(page: Page, username: string) {
  const search = page.getByPlaceholder("Invite player", { exact: true });
  await search.fill(username);
  const row = page.locator(".ui-dense-table-row").filter({ has: page.getByRole("link", { name: username, exact: true }) })
    .filter({ has: page.getByRole("button", { name: "Invite to tournament" }) });
  await row.getByRole("button", { name: "Invite to tournament" }).click();
  await expect(roster(page, "Invitees")).toContainText(username);
}
export async function notifications(page: Page) {
  const control = page.getByRole("button", { name: "Open notifications", exact: true });
  // The header control is shared by desktop and mobile.
  if (await control.getAttribute("aria-expanded") !== "true") await control.click();
}
export function notification(page: Page, name: string) { return page.locator(".ui-notification-item").filter({ hasText: name }); }
export async function start(page: Page) {
  await button(page, "Start").click();
  await expect(button(page, "Finish")).toBeVisible();
}
export async function bulkForfeit(page: Page) {
  await button(page, "Double forfeit unstarted games").click();
  await page.getByRole("button", { name: /^Confirm \d+ unstarted games?$/ }).click();
  await expect(button(page, "Undo adjudications")).toBeVisible();
}
