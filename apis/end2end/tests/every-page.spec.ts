import { test } from "@playwright/test";
import { auditPage, type Audit } from "./audit";
import { logIn, seeded } from "./helpers";

/**
 * The generic contract every page is held to, at every configured viewport.
 *
 * Adding a route here is the cheapest coverage available: it asserts the page
 * renders, does not scroll sideways, throws nothing, requests nothing broken,
 * and puts no `NaN`/`undefined` in front of a user.
 *
 * `expectText` is worth setting wherever an error boundary or an empty state
 * would pass everything else — a blank page overflows nothing.
 */
type Route = { path: string; name: string } & Audit;

const PUBLIC_ROUTES: Route[] = [
  { path: "/", name: "front page" },
  { path: "/tournaments", name: "tournaments" },
  { path: "/tournaments/future", name: "upcoming tournaments" },
  { path: "/tournaments/inprogress", name: "running tournaments" },
  { path: "/tournaments/finished", name: "finished tournaments" },
  { path: "/top_players", name: "top players" },
  { path: "/archive", name: "archive" },
  { path: "/login", name: "login", expectText: /Sign in/i },
  { path: "/register", name: "register" },
  { path: "/faq", name: "faq" },
  { path: "/rules", name: "rules" },
  { path: "/rules_summary", name: "rules summary" },
  { path: "/strategy", name: "strategy" },
  { path: "/resources", name: "resources" },
  { path: "/donate", name: "donate" },
];

test.describe("every public page", () => {
  for (const { path, name, ...audit } of PUBLIC_ROUTES) {
    test(name, async ({ page }) => {
      await auditPage(page, path, audit);
    });
  }
});

test.describe("every seeded tournament", () => {
  for (const tournament of seeded()) {
    test(`${tournament.mode} (${tournament.stage})`, async ({ page }) => {
      // The name proves the tournament itself rendered rather than an error
      // boundary, which would otherwise satisfy every other assertion.
      await auditPage(page, `/tournament/${tournament.nanoid}`, {
        expectText: tournament.name,
      });
    });
  }
});

test.describe("every signed-in page", () => {
  test.beforeEach(async ({ page }) => {
    await logIn(page, "tt-01");
  });

  const routes: Route[] = [
    {
      path: "/tournaments/create",
      name: "create a tournament",
      expectText: "Tiebreakers",
    },
    { path: "/tournaments/joined", name: "my tournaments" },
    { path: "/@/me", name: "own profile" },
    { path: "/config", name: "settings" },
    { path: "/notifications", name: "notifications" },
  ];

  for (const { path, name, ...audit } of routes) {
    test(name, async ({ page }) => {
      await auditPage(page, path, audit);
    });
  }
});

// Dark mode is a project in playwright.config.ts, so every test in this file
// already runs against both themes. Listing routes again here would only
// duplicate them.
