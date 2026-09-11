# Hive Playwright end-to-end tests

The suite runs against a Hive server that you start locally. It covers
Chromium, Firefox, and WebKit at desktop and mobile layouts.

## Reading results

The GitHub Actions run summary starts with the outcome and a scenario-by-browser
matrix, grouped by feature. Each populated cell links to that test's browser/layout
steps and shows the result and total time across attempts. Failed and flaky cases
appear above the matrix with their failed step, assertion message, attempt number,
and a source link at the tested commit.
Flaky diagnostics start collapsed and can be expanded or hidden with their
disclosure control; their counts and matrix results always remain visible.

Executed steps are grouped into expandable test sections, each containing
browser/layout sections in matrix-column order. Both levels start collapsed;
selecting a matrix result reveals the matching sections and jumps to their steps.
You can also expand them manually to read the steps, including each player's
sign-in. Nested steps show their own durations; those durations overlap
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
stay below GitHub's size limit; it prioritizes test groups containing failures or
retries and keeps each retained test's browser sections together. Matrix cells
whose details were omitted say **Details omitted** and have no link. Cases marked
**Not selected** also have no link. Truncation is explicitly marked, and the HTML
artifact retains the complete report.

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

Run the browser-free session, account cleanup, and fixture lifecycle checks with
`npm run test:unit`.

Run `npm run test:pool` with `PLAYWRIGHT_DATABASE_URL` set to a test PostgreSQL
connection that can create databases. These checks create and drop a separate
database, apply the real migrations, and verify rollback, account contention,
cross-process reservations, and recovery after a killed owner. They do not
modify the application's database.

With the Hive test server running, `npm run test:recovery` exercises actual
challenge cancellation, game abort/resignation, and cleanup after a failing
scenario on Chromium desktop and WebKit mobile. It reserves accounts from the
same pool, using `PLAYWRIGHT_DATABASE_URL` and `PLAYWRIGHT_BASE_URL` like the main
suite. These recovery checks are separate from the normal 36-case matrix.

These checks include a synthetic Playwright run with intentional failures,
retries, skips, a timeout, and setup/teardown errors. They assert both the report
contents and the child runner's failing exit status. They run independently of
the E2E suite and do not access its server or accounts. Full E2E discovery remains
36 cases; CI currently selects the five challenge/gameplay scenarios across six
projects, for 30 cases.

The workflow uses locked npm dependencies, matching browser installation,
first-retry traces, and four workers using exclusive account reservations. External actions
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

Authenticated scenarios import `test` and `expect` from `../support/fixtures`. Request
`user` for one authenticated user, or `players` for two. Both fixtures reserve
accounts only when requested, create fresh isolated browser contexts, and own
recovery, cleanup, context closure, and reservation release. Use one fixture or
the other in a scenario; requesting both makes separate reservations.

Each `Player` contains readonly `id`, `username`, `page`, and `context` properties.
Pass the player to domain actions that need its identity; pass `player.page` to
generic UI helpers and assertions. Both contexts inherit the project's browser
settings. `isMobileLayout` includes Firefox's narrow viewport project.

A single-user scenario needs no manual sign-in or teardown:

```ts
import { expect, test } from "../support/fixtures";
import { cancelChallenge, createPublicChallenge } from "../support/game/challenges";

test("cancel my public challenge", async ({ user }) => {
  await createPublicChallenge(user);
  await cancelChallenge(user);
  await expect(user.page.getByRole("button", { name: "Cancel Challenge", exact: true })).toHaveCount(0);
});
```

Shared helpers live in `support`, outside the scenario directory. Import actions
directly from their domain module. Account modules manage reservations and
recovery, browser modules manage authenticated contexts, and game modules provide
UI actions. `support/fixtures.ts` connects these pieces for the specs.

Paths below are relative to `support`:

| Module | Responsibility |
| --- | --- |
| `accounts/catalog.ts` | Test account identities and their public Quick Play queues |
| `accounts/pool.ts` | Validate seeded accounts and acquire exclusive database locks with bounded retries |
| `accounts/reservation.ts` | Monitor reservation ownership, inspect account state, and release the database session |
| `accounts/cleanup.ts` | Recover reserved accounts through application controls |
| `browser/authentication.ts` | UI sign-in and verification that the session survives reload |
| `browser/player.ts`, `browser/session.ts` | Shared player types and browser/session lifecycle |
| `browser/onboarding.ts`, `browser/session_cookies.ts`, `browser/diagnostics.ts` | Onboarding dismissal, WebKit cookie handling, and browser diagnostics |
| `game/challenges.ts` | Direct/public creation, row selection, acceptance, decline, and cancellation |
| `game/setup.ts` | Start a game with explicit white/black players and prepare mobile controls |
| `game/board.ts` | Locate board pieces, place and move pieces, confirm previews, and assert rendered board positions/stack levels |
| `game/controls.ts` | Open mobile controls and confirm game actions |
| `game/panels.ts` | Switch game/chat panels, locate history navigation controls, and inspect the desktop history list |

For example, a gameplay scenario can start with:

```ts
import { expect, test } from "../support/fixtures";
import { boardPiece, placePiece } from "../support/game/board";
import { confirmControl } from "../support/game/controls";
import { startGame } from "../support/game/setup";

// Gameplay has its own time budget; fixture setup has a separate timeout.
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
from `../support/game/panels` and use `await historyControl(page, "Previous").click()` or
`await expect(historyControl(page, "Previous")).toBeDisabled()`. Add a focused
helper to its domain module when multiple scenarios need the same operation.

Tests of challenges should call the individual actions instead of `startGame`:
`createDirectChallenge(challenger, opponent, "Random")`, followed
by `acceptChallenge({ challenger, opponent })`. Direct challenges require an
explicit `"White"`, `"Black"`, or `"Random"` color for the challenger. Public
challenges use `createPublicChallenge(user)` or
`createPublicChallenge(players.userOne)`; the helper selects the reserved
primary account's Quick Play label automatically. Acceptance waits for both players
to reach the same game URL; permissions, removal, and game outcomes stay in the
spec's assertions.

Each challenge scenario creates its own challenge. The row helper assumes one
outstanding challenge per challenger. Finish games and decline/cancel challenges
in the scenario so their outcomes remain asserted. Fixture setup also recovers
leftover challenges and unfinished games from an interrupted attempt, and
teardown repeats cleanup before closing its browser contexts and releasing
the accounts. Cleanup queries the database to identify state, then acts through
an authenticated reserved participant to cancel outgoing challenges, decline
incoming challenges, and abort/resign games. The opponent may be another seeded
test account without being reserved by this test. Cleanup tolerates state already
resolved by the other participant, and refuses non-test opponents or tournaments.
Recovery failure prevents the scenario from running; teardown failures are attached
separately when a test has already failed.

Anonymous tests can continue importing directly from `@playwright/test` without
signing in or providing a database connection.

### Shared account pool

Eight regular users form two pools: primary accounts `user_1`, `user_3`,
`user_5`, `user_7`, and partner accounts `user_2`, `user_4`, `user_6`, `user_8`.
`admin_1` is excluded. `user` reserves one primary account; `players` reserves
one primary and one partner, without fixed pairings. Four authenticated tests
can run at once. The six browser projects share the default four-worker limit.
Use `--workers=2` to reduce local resource use.

Quick Play automatically matches existing public challenges. Each primary
account respectively owns the `1+2`, `3+3`, `5+4`, or `10+10` queue, so concurrent
public challenges cannot match each other. `createPublicChallenge` derives this
label from the creator and rejects partner accounts. The pair fixture also
exposes the label as `players.publicTimeControl`.

One allocator, `reserveAccounts(1 | 2)`, uses per-account PostgreSQL session
advisory locks. A two-user request releases partial acquisitions before waiting,
so it never holds a primary account while waiting for a partner. Separate
runners and machines using the same database coordinate through these locks.
All runners must use this allocator version: finish runners using the previous
pair-lock scheme before starting the new version against the same database.
No database migration is needed beyond the existing eight-account seed.

Point `PLAYWRIGHT_DATABASE_URL` at the database served by `PLAYWRIGHT_BASE_URL`.
The allocator validates all eight verified, non-admin accounts. Use a dedicated
test database and do not use its seeded accounts interactively during a run.

Acquisition waits up to 180 seconds with bounded polling backoff. Both fixtures
have a separate 270-second budget covering acquisition and session work, so
queuing does not consume the scenario's timeout. Recovery and teardown cleanup
each have a 30-second limit. Pool exhaustion produces an explicit error.

The reservation connection remains open until its browser contexts close.
PostgreSQL releases its locks if the owning process dies. The next owner recovers
leftover state before running a scenario. A connection error or failed
five-second heartbeat fails the fixture and closes its contexts. Teardown
failures are attached separately when a test has already failed.

Concurrent runs should use separate checkouts. If sharing a checkout, supply
distinct `--output` directories, `PLAYWRIGHT_HTML_OUTPUT_DIR` values, and a
custom configuration with a distinct summary reporter `outputFile` for each
run. `--output` alone does not relocate the HTML or Markdown report. The main
suite stores artifacts under `test-results/e2e-tests`, separate from helper
test outputs.

## Setup

Install the locked Playwright version and its matching browser builds from this
directory. Use the local runner so a system installation cannot select a
different version:

```sh
npm ci
npx playwright install
```

For server-free validation, run `npm run typecheck` and `npm run test:unit`.
The strict TypeScript check covers helpers, specs, configurations, and reporters.
E2E (`npm test`), database pool (`npm run test:pool`), and browser recovery
(`npm run test:recovery`) checks run separately and require their own environment.

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

The development-only fixtures create `admin_1` and `user_1` through `user_8`, with
matching `@example.test` email addresses and password `password`. A single testware
migration creates all nine accounts, their ratings, and notification preferences;
its rollback removes all nine accounts and their dependent data. Existing databases
that already recorded the earlier seed will not rerun the consolidated migration;
use a fresh test database to apply it. The fixture migration lives outside the
application migrations and must not be applied to production databases.

Once the app is serving on port 3000, run the tests in another terminal:

```sh
cd apis/end2end
export PLAYWRIGHT_DATABASE_URL=postgres://hive-dev@127.0.0.1:5433/hive-local
npm test
```

That database URL matches the repository's Docker Compose stack. For a server
started directly on the host, set it to that server's development `DATABASE_URL`
instead. Every runner needs database network access as well as application
access. Normal tests only read fixture state and acquire advisory locks; the
`test:pool` validation command additionally needs database creation permission.

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

Use the `user` or `players` fixture for authenticated UI tests. The interceptor is
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

See `support/browser/session_cookies.ts` for the URL/type guards and
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
