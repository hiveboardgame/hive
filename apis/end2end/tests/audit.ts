import { expect, type Page, type Response } from "@playwright/test";
import {
  expectNoHorizontalOverflow,
  expectTappableTargets,
  waitForHydration,
} from "./helpers";

/**
 * Console noise that is not a defect. Kept deliberately short — every entry here
 * is a check we have given up, so a vague pattern silently costs coverage.
 */
const IGNORED_CONSOLE = [
  /favicon/i,
  // The hub reconnects on navigation; a closed socket is expected, not a fault.
  /websocket.*clos/i,
  /\[HMR\]|hot.?reload/i,
  // `interactive-widget=resizes-content` in the viewport meta
  // (`document_shell.rs`) is Chrome-only and deliberate — it controls how the
  // on-screen keyboard resizes a PWA. WebKit correctly warns and ignores it,
  // which is a browser-support notice rather than a defect.
  /Viewport argument key "interactive-widget"/,
];

/** Same-origin requests that are allowed to fail. */
const IGNORED_REQUESTS = [/favicon/i, /\.map$/];


/**
 * Text that should never reach a user. These are the shapes a formatting or
 * arithmetic bug takes in rendered output, which matters here because standings
 * compute floats and every tournament view formats dates.
 */
/**
 * Word-like, so they need boundaries: "nullify" and "undefined behaviour" are
 * legitimate prose, a bare `null` in a standings cell is not. Case-sensitive,
 * because these are what the language prints, and "Null" in prose is innocent.
 */
const FORBIDDEN_WORDS = ["NaN", "undefined", "null", "Infinity"];

/** Cannot occur innocently, so a plain substring match is right. */
const FORBIDDEN_LITERALS = [
  "Invalid Date",
  "[object Object]",
  "{{",
  "NaN%",
];

export type Audit = {
  /** Opt out where a control is legitimately small. On by default. */
  tapTargets?: boolean;
  /**
   * Text that proves the page rendered its own content.
   *
   * Worth passing wherever an error boundary or an empty state would otherwise
   * satisfy every other check in here — all of which a blank page passes.
   */
  expectText?: RegExp | string;
};

/**
 * Navigates to `path` and runs every generic invariant we hold for all pages.
 *
 * Listeners are attached before navigation on purpose: console errors and failed
 * requests during load are exactly the ones worth catching, and they are gone by
 * the time the page settles.
 */
export async function auditPage(
  page: Page,
  path: string,
  options: Audit = {},
): Promise<void> {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];
  const failedRequests: string[] = [];

  page.on("console", (message) => {
    if (message.type() !== "error") return;
    const text = message.text();
    if (IGNORED_CONSOLE.some((pattern) => pattern.test(text))) return;
    consoleErrors.push(text);
  });

  page.on("pageerror", (error) => pageErrors.push(error.message));

  // Only our own origin: a third-party outage is not a defect in this codebase,
  // and `page.url()` is still about:blank while the first responses arrive.
  const origin = new URL(process.env.HIVE_BASE_URL ?? "http://localhost:3000")
    .origin;

  page.on("response", (response: Response) => {
    const url = response.url();
    if (response.status() < 400) return;
    if (!url.startsWith(origin)) return;
    if (IGNORED_REQUESTS.some((pattern) => pattern.test(url))) return;
    failedRequests.push(`${response.status()} ${url}`);
  });

  const response = await page.goto(path);
  expect(response?.status(), `${path} should not error`).toBeLessThan(400);

  // Hydration first: until it happens `main` is `display: none`, so every check
  // below would pass against a page that renders nothing at all.
  //
  // Deliberately not `networkidle`, which never fires here — the websocket hub
  // holds a connection open for the life of the page.
  await waitForHydration(page);

  if (options.expectText) {
    await expect(page.getByText(options.expectText).first()).toBeVisible();
  }

  await expectNoHorizontalOverflow(page);
  await expectDocumentBasics(page);
  await expectNoForbiddenText(page);
  await expectImagesDescribed(page);
  if (options.tapTargets ?? true) {
    await expectTappableTargets(page);
  }

  expect(pageErrors, `uncaught exceptions on ${path}:\n${pageErrors.join("\n")}`).toEqual([]);
  expect(
    consoleErrors,
    `console errors on ${path}:\n${consoleErrors.join("\n")}`,
  ).toEqual([]);
  expect(
    failedRequests,
    `failed requests on ${path}:\n${failedRequests.join("\n")}`,
  ).toEqual([]);
}

/**
 * A non-empty title, and no more than one first-level heading.
 *
 * Deliberately not *exactly* one: several pages here are panel compositions with
 * no page-level heading at all, and failing them would be a redesign demand
 * dressed up as a test. Two `h1`s, though, is always a mistake.
 */
async function expectDocumentBasics(page: Page): Promise<void> {
  expect(await page.title(), "every page needs a title").not.toBe("");

  const headings = await page.locator("h1").count();
  expect(headings, "a page must not have more than one h1").toBeLessThanOrEqual(1);
}

async function expectNoForbiddenText(page: Page): Promise<void> {
  const found = await page.evaluate(
    ({ words, literals }) => {
      // `innerText` rather than `textContent`: only what is actually rendered
      // counts, so a hidden template cannot fail the page.
      const text = document.body.innerText;
      return [
        ...words.filter((word) => new RegExp(`\\b${word}\\b`).test(text)),
        ...literals.filter((literal) => text.includes(literal)),
      ];
    },
    { words: FORBIDDEN_WORDS, literals: FORBIDDEN_LITERALS },
  );

  expect(
    found,
    found.length
      ? `rendered placeholder or broken values: ${found.join(", ")}`
      : "clean",
  ).toEqual([]);
}

/** Images carry alt text, even if empty for decorative ones. */
async function expectImagesDescribed(page: Page): Promise<void> {
  const undescribed = await page.evaluate(() =>
    Array.from(document.images)
      .filter((image) => !image.hasAttribute("alt"))
      .map((image) => image.currentSrc || image.src)
      .slice(0, 5),
  );
  expect(
    undescribed,
    `images without an alt attribute:\n${undescribed.join("\n")}`,
  ).toEqual([]);
}
