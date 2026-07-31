import { expect, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

export type Stage = "upcoming" | "live" | "done";

export type Seeded = {
  name: string;
  nanoid: string;
  mode: string;
  stage: Stage;
};

const MANIFEST = join(__dirname, "..", "seeded.json");

/**
 * The tournaments `script tournaments` created, as written out by the seeder.
 *
 * Names and nanoids are random per run because `tournaments.name` is unique, so
 * there is no stable URL to hardcode — the manifest is the only reliable way in.
 */
export function seeded(): Seeded[] {
  try {
    return JSON.parse(readFileSync(MANIFEST, "utf8")) as Seeded[];
  } catch {
    throw new Error(
      `No fixtures at ${MANIFEST}. Run: cargo run --bin script tournaments`,
    );
  }
}

export function seededOne(mode: string, stage: Stage): Seeded {
  const match = seeded().find((t) => t.mode === mode && t.stage === stage);
  if (!match) {
    throw new Error(`No seeded ${mode} in stage ${stage}`);
  }
  return match;
}

/** Seeded accounts all share this; `tt-org` organizes everything. */
export const PASSWORD = "hivegame";

/**
 * Generous because a debug wasm bundle is enormous — around 240MB, roughly 17
 * seconds to hydrate — and Playwright gives each test a cold browser context, so
 * nothing is cached between them. A release build is far quicker; see the README.
 */
export const HYDRATION_TIMEOUT_MS = 90_000;

/**
 * Waits until the app is interactive.
 *
 * `<main>` is server-rendered with Tailwind's `hidden` and an `Effect` in
 * `base_layout.rs` clears it once hydration runs — an anti-flash trick that
 * doubles as the app's own "ready" signal. Waiting on it is what makes every
 * later assertion meaningful: before it, `main` is `display: none`, so the page
 * has no layout and no visible text. Checks for overflow, tap targets or stray
 * `NaN`s all pass trivially against a page that renders nothing.
 */
export async function waitForHydration(page: Page): Promise<void> {
  await page.waitForSelector("main:not(.hidden)", {
    timeout: HYDRATION_TIMEOUT_MS,
  });
}

export async function logIn(page: Page, username: string): Promise<void> {
  await page.goto("/login");
  // The form is inert until hydration: it posts through a server action, not a
  // plain form submit.
  await waitForHydration(page);
  // The field is named `email`, so this needs the seeded address, not the name.
  await page.fill("#email", `${username}@example.com`);
  await page.fill("#password", PASSWORD);
  await page.click('button[type="submit"]');
  // Asserting on the URL rather than on any post-login chrome, which differs by
  // viewport — the mobile header collapses.
  await expect(page).not.toHaveURL(/\/login/);
}

/**
 * Asserts the page does not scroll sideways.
 *
 * This is the assertion the responsive work actually needed. A component wider
 * than the viewport that is not inside its own scroll container drags the whole
 * document with it, which is the difference between "scrolls within a panel" and
 * "the site is broken on a phone". One pixel of slack absorbs sub-pixel layout
 * rounding, which is not a real overflow.
 */
export async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const overflow = await page.evaluate(() => {
    const root = document.documentElement;
    const slop = root.scrollWidth - root.clientWidth;
    if (slop <= 1) return null;

    // Naming the culprit, because "something overflows" is nearly useless on a
    // page with a few hundred elements.
    const limit = root.clientWidth;
    const guilty: string[] = [];
    for (const el of Array.from(document.body.querySelectorAll("*"))) {
      const box = el.getBoundingClientRect();
      if (box.width === 0 || box.right <= limit + 1) continue;
      // Keep only the outermost offenders: if the parent overflows too, this
      // element is a symptom rather than the cause.
      const parent = el.parentElement?.getBoundingClientRect();
      if (parent && parent.right > limit + 1) continue;
      guilty.push(
        `<${el.tagName.toLowerCase()} class="${el.getAttribute("class") ?? ""}"> right=${Math.round(box.right)}`,
      );
      if (guilty.length >= 5) break;
    }
    return { slop, limit, guilty };
  });

  expect(
    overflow,
    overflow
      ? `page scrolls ${overflow.slop}px past its ${overflow.limit}px viewport.\nWidest elements:\n${overflow.guilty.join("\n")}`
      : "no overflow",
  ).toBeNull();
}

/**
 * Every button-shaped control meets WCAG 2.5.8's 24x24 CSS px minimum.
 *
 * Deliberately not every `a[href]`: inline links inside body text are legitimately
 * line-height tall, and including them would bury real findings in noise.
 */
export async function expectTappableTargets(
  page: Page,
  min = 24,
): Promise<void> {
  const small = await page.evaluate((min) => {
    const offenders: string[] = [];
    const targets = document.body.querySelectorAll(
      'button, a[class*="ui-button"], [role="button"], input:not([type="hidden"]), select',
    );
    for (const el of Array.from(targets)) {
      // A visually-hidden input paired with a styled label is the correct
      // accessible pattern — the label is the tap target, so measuring the input
      // reports a 1x1 defect that does not exist.
      if (el.classList.contains("sr-only")) continue;
      // A range slider's hit area is its track, not the short side of its box.
      if (el instanceof HTMLInputElement && el.type === "range") continue;
      // WCAG 2.5.8 exempts a small control whose function is also available from
      // an adequate one, and a linked <label> is exactly that: clicking it
      // toggles the input, so the real target is the box plus the label text.
      // Not measured, because a single line of label text is ~20px tall and
      // would fail on height while being an entirely comfortable thing to tap.
      // A checkbox with no label still fails, as it should.
      if (
        el instanceof HTMLInputElement &&
        (el.type === "checkbox" || el.type === "radio") &&
        el.labels?.length
      ) {
        continue;
      }

      const box = el.getBoundingClientRect();
      // Skip anything not rendered — collapsed menus, inactive tab panels.
      if (box.width === 0 || box.height === 0) continue;
      if (Math.min(box.width, box.height) >= min) continue;
      offenders.push(
        `<${el.tagName.toLowerCase()} class="${el.getAttribute("class") ?? ""}"> ${Math.round(box.width)}x${Math.round(box.height)}`,
      );
    }
    return offenders.slice(0, 8);
  }, min);

  expect(
    small,
    small.length ? `tap targets under ${min}px:\n${small.join("\n")}` : "all fine",
  ).toEqual([]);
}
