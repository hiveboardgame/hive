import type { Page } from "@playwright/test";
import { activate } from "../gestures";

export function historyControl(page: Page, direction: "First" | "Previous" | "Next" | "Last") {
  return page.getByRole("button", { name: `${direction} move`, exact: true });
}

export async function historyStep(page: Page, direction: "First" | "Previous" | "Next" | "Last", hasTouch: boolean) {
  await activate(historyControl(page, direction), hasTouch);
}
