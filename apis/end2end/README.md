# Hive Playwright end-to-end tests

The suite runs against a Hive server that you start locally. It covers
Chromium, Firefox, and WebKit at desktop and mobile layouts.

## Reading results

The GitHub Actions run summary starts with the outcome and a scenario-by-browser
matrix, grouped by feature. Each cell shows the result and total time across
attempts. Failed and flaky cases appear above the matrix with their failed step,
assertion message, attempt number, and a source link at the tested commit.
Flaky diagnostics start collapsed and can be expanded or hidden with their
disclosure control; their counts and matrix results always remain visible.

Expand a scenario/browser entry to read its executed steps, including each
player's sign-in. Nested steps show their own durations; those durations overlap
and must not be added together. Retries appear as separate attempts. A test that
passes on retry is marked **Flaky**, and skipped, interrupted, and unexecuted
tests remain distinct from passes. An interrupted run can only report steps
that Playwright delivered before shutdown.

The summary's **Download the Playwright report and server diagnostics** link downloads
the `e2e-results` artifact, retained for seven days. Extract the archive,
then run the installed Playwright runner from this directory:

```sh
npx playwright show-report /path/to/extracted-e2e-results/apis/end2end/playwright-report
```

The HTML report contains the full action log, errors, and available retry traces.
It is an artifact to download and serve locally, not a hosted report website.
Playwright supplies GitHub failure annotations. The same archive includes Docker
logs covering deployment and testing, plus the final container status, at its root.
Collection and upload run together before stack cleanup. Diagnostics are still
uploaded when an earlier failure prevents Playwright from producing a report.

Local runs also write `test-results/summary.md`, alongside the HTML report. The
summary limits lengthy messages and truncates detail blocks when necessary to
stay below GitHub's size limit; it prioritizes failures and retries and explicitly
marks truncation. The HTML artifact retains the complete report.

## Maintaining reporting

`reporters/github-summary.ts` collects results through Playwright's public
reporter API. It traverses fixture and hook children to retain named steps while
omitting routine locator operations. `reporters/summary.ts` formats the matrix,
attempts, and steps as escaped Markdown. `reporters/publish-summary.mjs` publishes
that file and the artifact download link to `GITHUB_STEP_SUMMARY`.

Use concise `test.describe` feature names and scenario titles, and give
`test.step` calls user-visible action or outcome names. These are the report's
labels; no separate reporting catalog needs updating. Keep assertion details in
the tests and preserve nested steps where they help explain a longer journey.

Run the browser-free reporter checks with:

```sh
npm run test:reporter
```

Run the browser-free session helper checks with `npm run test:unit`.

These checks include a synthetic Playwright run with intentional failures,
retries, skips, a timeout, and setup/teardown errors. They assert both the report
contents and the child runner's failing exit status. They run independently of
the E2E suite and do not access its server or accounts. Full E2E discovery remains
36 cases; CI currently selects the five challenge/gameplay scenarios across six
projects, for 30 cases.

The workflow uses locked npm dependencies, matching browser installation,
first-retry traces, and the existing sequential account usage. External actions
use explicit release tags; verify new versions against the official action
repositories when updating them. GitHub token
permissions are limited to content reads. Reporting and artifact uploads run after failures, while
test failures continue to determine the job's failing status.

The workflow runs for pull requests targeting `main` and can be dispatched
manually with a selected branch. Once this workflow is present on the default
branch, enable a `main` branch protection rule or ruleset requiring pull requests
and the `Test run` status check to prevent merges when it fails. YAML triggers
alone do not make the check mandatory. A temporary push trigger also runs this
workflow on `testware_finally`; remove it after validation in Actions.

## Writing tests

The specs describe behaviors and keep their move sequences and outcome assertions
visible. `challenges.spec.ts` has independent decline, public cancellation, and
accept/abort scenarios. `gameplay.spec.ts` keeps the two longer gameplay journeys
together, and `smoke.spec.ts` covers anonymous navigation.

Authenticated scenarios import `test` and `expect` from `./fixtures`. Requesting
`players` signs in `user_1` and `user_2` in separate browser contexts and exposes
each player's `username` and `page`. The fixture closes the second context even
when setup or the test fails; Playwright owns the first context. Both contexts
inherit the project's browser settings. `isMobileLayout` includes Firefox's
narrow viewport project.

Import actions directly from their domain module in `tests/test_utils`:

| Module | Responsibility |
| --- | --- |
| `authentication.ts` | UI sign-in and verification that the session survives reload |
| `challenges.ts` | Direct/public creation, row selection, acceptance, decline, and cancellation |
| `game_setup.ts` | Start a game with explicit white/black players and prepare mobile controls |
| `board.ts` | Locate board pieces, place and move pieces, confirm previews, and assert rendered board positions/stack levels |
| `game_controls.ts` | Open mobile controls and confirm game actions |
| `game_panels.ts` | Switch game/chat panels, locate history navigation controls, and inspect the desktop history list |
| `player.ts` | Shared username/page type |
| `onboarding.ts`, `session_cookies.ts`, `browser_diagnostics.ts` | Browser support, separate from domain actions |

For example, a gameplay scenario can start with:

```ts
import { expect, test } from "./fixtures";
import { boardPiece, placePiece } from "./test_utils/board";
import { confirmControl } from "./test_utils/game_controls";
import { startGame } from "./test_utils/game_setup";

// Configure this before fixtures run, so login shares the scenario's time budget.
test.describe.configure({ timeout: 90_000 });

test("a player can place an opening ant", async ({ players, isMobileLayout }) => {
  const white = players.userOne;
  const black = players.userTwo;
  await startGame({ white, black, isMobileLayout });

  await test.step("Place the opening ant", async () => {
    await placePiece(white.page, "White Ant 1", "16, 16");
    await expect(boardPiece(white.page, "White Ant 1")).toBeVisible();
  }, { box: true });

  await test.step("Abort the opening game", async () => {
    await confirmControl(white.page, "Abort");
    for (const player of [white, black]) {
      await expect(player.page).toHaveURL(/\/$/);
    }
  }, { box: true });
});
```

Add scenarios to the relevant spec using the existing fixtures and domain actions.
Keep move sequences, expected outcomes, and named steps in the spec. Locator
helpers return ordinary Playwright locators: for example, import `historyControl`
from `game_panels` and use `await historyControl(page, "Previous").click()` or
`await expect(historyControl(page, "Previous")).toBeDisabled()`. Add a focused
helper to its domain module when multiple scenarios need the same operation.

Tests of challenges should call the individual actions instead of `startGame`:
`createDirectChallenge(challenger.page, opponent.username, "Random")`, followed
by `acceptChallenge({ challenger, opponent })`. Direct challenges require an
explicit `"White"`, `"Black"`, or `"Random"` color for the challenger. Public
challenges take an explicit quick-play label, such as
`createPublicChallenge(challenger.page, "1+2")`. Acceptance waits for both players
to reach the same game URL; permissions, removal, and game outcomes stay in the
spec's assertions.

Each challenge scenario creates its own challenge. The row helper assumes one
outstanding challenge per challenger. The fixture cleans up browser contexts,
not server state: finish games and decline/cancel challenges in the scenario.
Keep one worker because the seeded accounts share server state. Anonymous tests
can continue importing directly from `playwright/test` without signing in.

## Setup

Install the locked Playwright version and its matching browser builds from this
directory. Use the local runner so a system installation cannot select a
different version:

```sh
npm ci
npx playwright install
```

The suite blocks service workers so application requests reach the session-cookie
interceptor on WebKit desktop and mobile, and to avoid WebKit's frame/navigation
failure with the PWA worker enabled. PWA-specific tests should opt back in with
`test.use({ serviceWorkers: "allow" })`.

The shared `signIn` helper dismisses install and browser-notification banners
when they appear, for both players, so they cannot cover the mobile game pieces.
It uses each banner's Dismiss button, which saves the normal dismissal preference
across page reloads.

Mobile gameplay runs in portrait and uses the header's chat dropdown, including
its unread indicator. History checks click the visible Previous, Next, First,
and Last move buttons without rotating the viewport. Assertions check pieces
appearing/disappearing, their board coordinates and stack levels, and disabled
navigation at the beginning/end. Desktop tests retain move-list assertions.

## Run

With PostgreSQL running and `DATABASE_URL` pointing to your development database,
apply the schema and E2E fixture migrations, then start Hive from the repository
root in one terminal:

```sh
cd db
diesel migration run
diesel migration run --migration-dir testware
cd ..
cargo leptos watch --hot-reload
```

The development-only fixture creates `admin_1`, `user_1`, and `user_2`, with
matching `@example.test` email addresses and password `password`. Fixture
migrations live outside the application migrations and must not be applied to
production databases.

Once the app is serving on port 3000, run the tests in another terminal:

```sh
cd apis/end2end
npm test
```

To run against a different deployment, provide its base URL:

```sh
PLAYWRIGHT_BASE_URL=https://example.test npm test
```

### Testing release builds locally

Release builds set `Secure` session cookies, which WebKit does not retain on
`http://127.0.0.1`. For WebKit desktop and mobile tests, the shared `signIn` helper
installs a context-wide response interceptor before login. It strips only the
`Secure` attribute from application-origin `/api/` fetch/XHR response cookies,
including later API session updates, so authenticated tests can run against a
release server over HTTP. Both players' contexts receive the workaround, and
login checks that the session survives a full page reload.
The interceptor also normalizes those exact cookies in the context's cookie jar,
which `route.fetch` populates before WebKit receives the rewritten response.

Use the shared `signIn` helper for authenticated UI tests. The interceptor is
test-only and restricted to HTTP: HTTPS runs retain their Secure cookies.
It leaves Chromium and Firefox responses unchanged and does not apply to direct
Playwright API requests. The application's cookie settings are unchanged.

The interceptor distinguishes **page navigation** from **background API calls**:

- The URL filter only matches the application's origin and `/api/` paths. Loading
  `/login` or `/game/...` never enters this handler.
- A page load or normal form submission to `/api/...` does match the URL filter,
  but its request type is `document`. The handler calls `route.fallback()` and
  returns immediately. That passes the request to any remaining matching handler
  or, if there is none, the browser's network stack. It does not cancel the request
  or rewrite its response; the browser follows redirects normally.
- Only `fetch` and `xhr` requests reach `route.fetch`, cookie rewriting, and
  `route.fulfill`. Hydrated Leptos login uses a background request returning HTTP
  200 with a client-redirect header. The helper preserves that header so the app
  can navigate after login.
- This distinction avoids trying to replay a real HTTP redirect through WebKit's
  `route.fulfill`, which WebKit does not support. Service workers are separate:
  they are blocked so they cannot handle requests before the interceptor sees them.

See `tests/test_utils/session_cookies.ts` for the URL/type guards and
`unit/session_cookies.spec.ts` for the check that document requests fall through
without fetching or rewriting their responses.

Docker test mode runs `cargo leptos serve --release`, using the server's release
profile and the frontend's size-optimized `wasm-release` profile, including
cargo-leptos's release-only `wasm-opt` processing and JavaScript minification.
The WebKit interceptor also applies to this HTTP test endpoint. Normal development
continues to use `cargo leptos watch --hot-reload` with development profiles.

If a local HTTPS reverse proxy uses a self-signed certificate, opt in to accepting
that certificate for the test run (the proxy must also forward WebSockets):

```sh
PLAYWRIGHT_BASE_URL=https://127.0.0.1:3443 \
PLAYWRIGHT_IGNORE_HTTPS_ERRORS=1 \
npm test -- tests/gameplay.spec.ts --project=webkit-desktop
```

`PLAYWRIGHT_IGNORE_HTTPS_ERRORS` only bypasses certificate validation; it does not
enable HTTPS on the application server. Leave it unset for trusted certificates.

Use `npm test -- --headed` to watch the
browser run the suite. Playwright writes an HTML report after each run; open it
with:

```sh
npx playwright show-report
```
