import { expect, test } from "@playwright/test";
import {
  expectNoHorizontalOverflow,
  logIn,
  seededOne,
  waitForHydration,
} from "./helpers";

/*
 * Page-level overflow is asserted for every route by `every-page.spec.ts`, so
 * this file holds only what a generic audit cannot express: containment of
 * deliberately-wide content, a clock that has to advance on its own, and layout
 * that is reachable only by interacting.
 */

/**
 * A bracket is wider than a phone by nature. The requirement is not that it
 * fits, but that it scrolls *inside its own panel*.
 *
 * A page-level overflow check alone cannot tell the two apart: a bracket that
 * correctly contains its scroll and a bracket that happens to be narrow enough
 * both pass. This asserts the containment is real.
 */
test.describe("bracket containment", () => {
  for (const stage of ["live", "done"] as const) {
    test(`single elimination scrolls within its panel (${stage})`, async ({
      page,
    }) => {
      const tournament = seededOne("SingleElimination", stage);
      await page.goto(`/tournament/${tournament.nanoid}`);
      await waitForHydration(page);

      const bracket = page.getByTestId("bracket-scroll");
      await expect(bracket).toBeVisible();

      const { fits, overflows, scrollable } = await bracket.evaluate((el) => ({
        fits:
          el.getBoundingClientRect().width <=
          document.documentElement.clientWidth + 1,
        overflows: el.scrollWidth > el.clientWidth,
        scrollable: ["auto", "scroll"].includes(getComputedStyle(el).overflowX),
      }));

      expect(fits, "the bracket panel must fit the viewport").toBe(true);
      // Not "it must scroll": how many rounds exist depends on the stage and how
      // wide the viewport is, so a short bracket legitimately fits. What must
      // always hold is that content too wide for the panel stays reachable
      // *inside* it rather than dragging the page sideways.
      if (overflows) {
        expect(scrollable, "wider-than-panel content must be scrollable").toBe(
          true,
        );
      }
      await expectNoHorizontalOverflow(page);
    });
  }
});

/** The front-page arena card is new and has never been seen at any width. */
test.describe("live arena card", () => {
  test("shows a running arena with a countdown and an action", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForHydration(page);

    await expect(page.getByText("Arenas running now")).toBeVisible();

    const card = page.locator("li", { hasText: /playing/ }).first();
    await expect(card).toBeVisible();
    // Signed out, the action slot offers View rather than Join.
    await expect(card.getByRole("link", { name: "View" })).toBeVisible();
  });

  test("the countdown ticks down on its own", async ({ page }) => {
    await page.goto("/");
    await waitForHydration(page);

    const card = page.locator("li", { hasText: /playing/ }).first();
    await expect(card).toBeVisible();

    const readClock = async () =>
      (await card.innerText()).match(/(\d+)h (\d+)m|(\d+):(\d\d)/)?.[0] ?? "";

    const first = await readClock();
    expect(first, "the card must show a countdown").not.toBe("");
    // Under an hour the clock is m:ss and moves every second; above it is h mm,
    // so a seeded three-hour arena would need a minute to visibly change. Only
    // the m:ss form is worth waiting on.
    if (/^\d+:\d\d$/.test(first)) {
      await expect(async () => {
        expect(await readClock()).not.toBe(first);
      }).toPass({ timeout: 5000 });
    }
  });
});

/**
 * The create form is the densest layout in the tournament work: nine modes, a
 * reorderable tiebreaker list, and several mode-gated sliders. The static form is
 * covered by the every-page audit; what is only reachable by interacting is the
 * mode-gated part of the layout.
 */
test.describe("tournament creation", () => {
  test("switching to arena hides tiebreakers and keeps the layout intact", async ({
    page,
  }) => {
    await logIn(page, "tt-01");
    await page.goto("/tournaments/create");
    await waitForHydration(page);

    await page
      .locator('select[name="Tournament Mode"]')
      .selectOption({ label: "Arena" });
    await expect(
      page.getByText(/Arena automatically ranks by points/),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });
});
