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

The HTML report contains the full action log, errors, and retained failure traces.
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

## Diagnosing challenge cancellation

The authenticated fixtures install the existing browser diagnostic logger,
now also recording WebSocket creation, closure and errors.
Its in-memory buffer is bounded to 500 entries per player. On an unexpected test
failure, both players' diagnostic logs and current accessibility snapshots are
attached **before account cleanup**, so cleanup cannot replace the failure evidence.
If sign-in fails before the test body starts, the authentication wrapper attaches
that player's diagnostics and snapshot before context closure, then rethrows the
original error. Test-body failures retain the existing capture for both players.
Snapshot collection is bounded and attachment failures do not prevent cleanup.
Playwright records every attempt and keeps traces for failed attempts, including
first failures on local runs without retries. This adds trace-recording overhead;
passing-attempt traces are discarded.

### Authenticated-page readiness

The September 13, 2026 run inspected after enabling traces had 71 passed and 13
failed cases, with no runner-level errors. All failures were in WebKit: 11 during
sign-in and two during challenge setup. Each failing username assertion began
while `main` still had its hydration `hidden` class. Subsequent account responses
returned HTTP 200 with the expected username, supporting a readiness timeout
rather than a lost session. Seven assertions also triggered onboarding dismissal
near their deadline; one reported a secondary page-closed error during dismissal.

Authenticated setup now reuses `expectHydrated(page)` immediately after full
navigations to login, home and opponent profiles, and after recovery/cleanup
navigations. The existing username and content checks follow hydration. This
separates the existing 45-second hydration allowance from the normal five-second
assertion budget without changing scenario, fixture or cleanup deadlines. The
30-second cleanup step remains an overall limit. Onboarding handlers keep their
normal dismissal behavior. The browser-free cleanup tests model an already
hydrated `main` in their page doubles to support the added assertions.

The traces also contained two audio-fetch errors around navigation, plus viewport,
blocked service-worker and unused-preload messages. These remain diagnostic
evidence; they were not the failed assertions. The earlier dropped cancellation
was not reproduced: Chromium and Firefox cancellation cases passed, while both
WebKit cases failed before cancellation. Cancellation actions and locators are
unchanged. Confirm the readiness fix by rerunning the existing browser and
recovery suites; static checking alone cannot establish that flakiness is resolved.

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
suite. These recovery checks are separate from the normal 210-case matrix.

Reporter checks include four focused synthetic Playwright runs for ordinary
statuses and project grouping, retries/failures, timeout, and fixture/global
teardown errors. They assert both the report contents and the child runner's expected exit status. They run independently of
the E2E suite and do not access its server or accounts.

The browser suite defines 35 scenarios across six projects (210 cases). CI
selects the 32 authenticated scenarios (192 cases); anonymous smoke tests remain
separately selectable. These counts were verified with Playwright discovery.
The [tournament UI guide](TOURNAMENTS.md) documents the 18 new scenarios,
their UI-only coverage boundary, database fixtures, and validation results.

The workflow uses locked npm dependencies, matching browser installation,
traces retained for every failed attempt, and 12 workers using exclusive account reservations. External actions
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

Each browser scenario owns fresh contexts. Gameplay scenarios reserve accounts
and use UI setup with only the required moves. Tournament scenarios own temporary
accounts and seed isolated prerequisites, then exercise their subject actions
through the UI. Related tournament actions are grouped into 18 concise journeys;
they do not verify result calculations. All desktop/mobile and browser projects remain.

### Suite report and coverage mapping

| Suite / spec | Scenarios | Previous coverage → new boundary and reasoning |
| --- | ---: | --- |
| Anonymous navigation (`tests/smoke.spec.ts`) | 3 | Combined smoke journey → home rendering, responsive navigation, and sign-in redirect; each can fail independently. |
| Challenges (`authenticated/challenges.spec.ts`) | 3 | Existing decline and public cancellation/permissions retained; acceptance stops at the shared game URL. |
| Board history (`authenticated/history.spec.ts`) | 2 | Long gameplay journey → two-move navigation and seven-move stacked-beetle history; preserves desktop notation and mobile board/navigation assertions. |
| Turn rules (`authenticated/turn_rules.spec.ts`) | 1 | Long gameplay journey → board movement and reserve placement restrictions after five moves. These are two ways to violate the same turn rule. |
| Takebacks (`authenticated/takebacks.spec.ts`) | 2 | Long gameplay journey → independent rejection and acceptance after seven moves; retains stack restoration coverage. |
| Draws (`authenticated/draws.spec.ts`) | 2 | Long gameplay journey → independent rejection and acceptance after two moves; removes preceding history and takeback work. |
| Chat (`authenticated/chat.spec.ts`) | 2 | Chat/resignation journey → open-chat exchange and unread-alert/read clearing; no board moves needed. |
| Game endings (`authenticated/endings.spec.ts`) | 2 | Challenge accept/abort and chat/resignation journeys → abort an unstarted game and resign after two moves. |
| Tournament UI (`authenticated/tournaments/`) | 18 | Discovery, forms, registration, invitations, role controls, lifecycle, scheduling, readiness, adjudication controls, Swiss controls, and chat. See [coverage and rationale](TOURNAMENTS.md). |

Authenticated paths above are relative to `tests`. The earlier gameplay refactor
expanded 6 scenarios / 36 cases to 17 / 102. Tournament coverage adds 18 / 108,
bringing the suite to 35 / 210 and the authenticated selection to 32 / 192.
Shorter individual journeys do not guarantee a shorter total run. Existing
gameplay budgets, retry policy, and 12-worker default remain unchanged.

| Supporting suite | Grouping and reason |
| --- | --- |
| Account allocation / ownership loss (`test:unit`) | Allocation failures are independent cases; lock loss retains its invalidation/release sequence. Pending reservations settle before mock teardown; the waiting case uses a five-second acquisition deadline instead of one second so ordinary polling can observe its first attempt. |
| Account cleanup (`test:unit`) | Named cancellation, decline, abort, resignation, single-user, and invalid-state cases replace mixed scenarios. A small ordering case retains challenges-before-games coverage. |
| Session lifecycle (`test:unit`) | Ordered setup/use/cleanup/release tests remain intact because ordering is their assertion; each failure mode remains independent. |
| Cookie handling (`test:unit`) | Browser/protocol interception boundaries stay together and separate from account lifecycle. |
| Seed lifecycle (`test:pool`) | Rollback/reapply stays one sequence to verify stable identities and complete cleanup. |
| Account reservations (`test:pool`) | Exclusivity, exhaustion, and waiting/reuse are independent for one- and two-user reservations. |
| Cross-process ownership (`test:pool`) | Acquisition, owner death, and reuse stay one causal sequence. |
| Connection loss (`test:pool`) | Session invalidation and credential-safe connection errors are independently reported. |
| Browser recovery (`test:recovery`) | Existing interrupted-state variants and failed-attempt cleanup stay separate. Initial cleanup remains necessary because interrupted-session simulation disables setup and teardown cleanup. |
| Report rendering / publishing / runner integration (`test:reporter`) | Fast formatting assertions, publisher outcomes, and four focused real-runner cases have separate specs. The runner cases retain status, step, retry, source-link, and error coverage. |

### Supporting code added or changed

The suite refactor adds no application hooks or dependencies. The account
expansion is included in the single testware seed, with no Rust application changes.

| Code | Why it is needed |
| --- | --- |
| `support/game/opening.ts`: `playOpening(players, 2 \| 5 \| 7)` and board positions | Shares only the repeated opening. Each move waits for its position on both clients, including stack level, before the next action. |
| `support/browser/hydration.ts`: `expectHydrated(page)` | Reuses the existing 45-second hydration assertion across three independent anonymous cases. |
| `support/game/panels.ts`: required layout argument to `showTab` | Chooses the correct mobile/desktop control from the fixture, avoiding an immediate visibility probe. Desktop-only `reviewHistory` passes `false` explicitly. |
| `support/game/controls.ts`: confirmation readiness | Waits for the control's adjacent Cancel button before the second click; the cleanup fake now verifies that wait occurs after the first click. |
| `integration/database.ts` | Extracts the existing worker-scoped disposable-database fixture for the split database suites. |
| Account waiting checks | Replace the fixed 150 ms sleep with a bounded observation of the new session's idle unlock query. Attach success/failure handlers immediately and settle pending acquisitions before release. No allocator API change. |
| Cross-process readiness marker | Write then rename the marker so the parent cannot parse partially written JSON; recognize signal exits when checking child state. |
| `reporters/synthetic-runner.ts` | Shares only synthetic file/configuration creation and child invocation across the four runner cases. Each uses its own output directory. |

Outcome assertions retain named steps for reporting. Fixture cleanup ends games
left by focused scenarios, so history, chat, and challenge acceptance do not also
test unrelated game endings. Runtime flakiness and performance improvements must
be verified by running the suites; static checking cannot establish them.

### Selecting suites

From `apis/end2end`:

```sh
npm test -- tests/authenticated
npm test -- tests/smoke.spec.ts
npm test -- tests/authenticated/takebacks.spec.ts --project=webkit-mobile
npm run test:unit
npm run test:pool
npm run test:recovery
npm run test:reporter
```

The commands above are for the maintainer to run. This refactor was checked with
TypeScript checking and `git diff --check` only; no tests or test discovery ran.

Authenticated scenarios import `test` and `expect` from `../../support/fixtures`. Request
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
import { expect, test } from "../../support/fixtures";
import { cancelChallenge, createPublicChallenge } from "../../support/game/challenges";

test.use({ publicChallenge: true });

test("cancel my public challenge", async ({ user }) => {
  await createPublicChallenge(user);
  await cancelChallenge(user);
  await expect(user.page.getByRole("button", { name: "Cancel Challenge", exact: true })).toHaveCount(0);
});
```

Shared helpers live in `support`, outside the scenario directory. Import actions
directly from their domain module. Account modules manage reservations and
recovery, browser modules manage authenticated contexts, and game modules provide
UI actions. `support/fixtures.ts` connects these pieces for the specs. Authenticated specs
are one directory deeper than anonymous specs, hence their `../../support` imports.

Paths below are relative to `support`:

| Module | Responsibility |
| --- | --- |
| `accounts/catalog.ts` | Test account identities and their public Quick Play queues |
| `accounts/pool.ts` | Validate seeded accounts and acquire exclusive database locks with bounded retries |
| `accounts/reservation.ts` | Monitor reservation ownership, inspect account state, and release the database session |
| `accounts/cleanup.ts` | Recover reserved accounts through application controls |
| `browser/authentication.ts` | UI sign-in and verification that the session survives navigation to home |
| `browser/player.ts`, `browser/session.ts` | Shared player types and browser/session lifecycle |
| `browser/onboarding.ts`, `browser/session_cookies.ts`, `browser/diagnostics.ts` | Onboarding dismissal, WebKit cookie handling, and browser diagnostics |
| `game/challenges.ts` | Prepare authenticated home pages, direct/public creation, row selection, acceptance, decline, and cancellation |
| `browser/hydration.ts` | Wait for the anonymous page to finish hydration |
| `game/opening.ts` | Play a short opening and synchronize both players after each move |
| `game/setup.ts` | Start a game with explicit white/black players and prepare mobile controls |
| `game/board.ts` | Locate board pieces, place and move pieces, confirm previews, and assert rendered board positions/stack levels |
| `game/controls.ts` | Open mobile controls and confirm game actions |
| `game/panels.ts` | Switch game/chat panels, locate history navigation controls, and inspect the desktop history list |

For example, a gameplay scenario can start with:

```ts
import { expect, test } from "../../support/fixtures";
import { boardPiece, placePiece } from "../../support/game/board";
import { startGame } from "../../support/game/setup";

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
});
```

Add scenarios to the relevant spec using the existing fixtures and domain actions.
Keep move sequences, expected outcomes, and named steps in the spec. Locator
helpers return ordinary Playwright locators: for example, import `historyControl`
from `../../support/game/panels` and use `await historyControl(page, "Previous").click()` or
`await expect(historyControl(page, "Previous")).toBeDisabled()`. Add a focused
helper to its domain module when multiple scenarios need the same operation.

Challenge setup explicitly navigates observers to the home page, including after
fixture recovery, and checks the authenticated username and home heading. The
small `openChallengeHome(player)` helper is shared by direct/public creation and
the public-challenge observer. This prevents waiting for rows on a sign-in or
finished-game page. Chat panel selection waits for its input to be visible; mobile
selection also verifies the requested expanded state. These are observable UI
checks, with no sleeps, forced clicks, or blanket timeout increases.

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
outstanding challenge per challenger. Assert game endings and challenge removal
in the suites dedicated to those behaviors. Other scenarios leave unfinished
state to fixture cleanup. Fixture setup also recovers
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

Thirty regular accounts (`user_1` through `user_30`) form two pools: 15 odd-numbered
primary accounts and 15 even-numbered partners. The existing `admin_1` remains
excluded. `user` reserves one primary; `players` reserves a primary and a partner,
without fixed pairings. The default is 12 workers; account capacity is 15 pairs.

Public Quick Play challenges require an exclusive queue as well as accounts.
`user_1`, `user_3`, `user_5`, `user_7`, and `user_9` own `1+2`, `3+3`, `5+4`,
`10+10`, and `20+20`, respectively. Put public-challenge scenarios in a describe
with `test.use({ publicChallenge: true })`. This option restricts the primary
reservation to these five users, while ordinary scenarios use all 15 primaries.
Both modes use the same account locks. Up to five public scenarios can coexist;
other scenarios can use remaining accounts. Direct challenges need no queue.

`createPublicChallenge(player)` derives the queue and rejects partners or primaries
without a queue. The unused `players.publicTimeControl` field was removed; callers
use this helper. Direct allocator users can pass `{ publicChallenge: true }`.

One allocator, `reserveAccounts(1 | 2)`, uses per-account PostgreSQL session
advisory locks. A two-user request releases partial acquisitions before waiting,
so it never holds a primary account while waiting for a partner. Separate
runners and machines using the same database coordinate through these locks.
All runners must use this allocator version: finish runners using the previous
pair-lock scheme before starting the new version against the same database.
Use a database with the consolidated thirty-account seed before starting runners.
Stop older runners first: they cannot recover state involving the new accounts.

Point `PLAYWRIGHT_DATABASE_URL` at the database served by `PLAYWRIGHT_BASE_URL`.
The allocator validates all thirty verified, non-admin accounts. Use a dedicated
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

The development-only fixtures create `admin_1`, `user_1` through `user_30`, and `SwissByePlayer`,
with matching `@example.test` email addresses and password `password`. One migration,
`db/testware/2026-09-05-000000_e2e_users`, contains the complete seed in `up.sql`
and its rollback in `down.sql` (both files are required by Diesel). It creates all
32 users, their six ratings each, and their notification preferences. Rollback
removes those accounts and their dependent records.

The Swiss sentinel uses stable UUID suffix `000000000020` and is excluded from
the gameplay account pool. It was added to the existing local database separately
without changing existing data or migration history. See the
[seed and additive-update record](TOURNAMENTS.md#persistent-seed-and-local-database-update).

For a fresh test database, run from the repository root:

```sh
cd db
DATABASE_URL=postgres://hive-dev@127.0.0.1:5433/hive-local diesel migration run --migration-dir testware
```

The local `hive-local` database was expanded to 30 regular accounts with an additive
update, preserving existing accounts, game data, and migration history. The retired
expansion version (`20260913000000`) had already been removed during the earlier
consolidation; the original seed version remains recorded. Diesel will
not rerun an already-recorded seed just because its SQL changed. Other databases
with an older eight- or twenty-account seed must be recreated as disposable test
databases or upgraded explicitly before using this pool. Do not roll back a seed
to upgrade a database whose fixture-owned game data you want to retain.

These migrations live outside application migrations and are for test databases.
The default worker count is 12. General reservations support up to 15 concurrent
pairs; public-challenge reservations retain the five-queue restriction.
The allocator orders users numerically through the catalog so `user_10` cannot
shift account lock identities through lexical sorting.

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
login checks that the session survives a full navigation to home and that the
Create a game heading is visible.
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
npm test -- tests/authenticated/history.spec.ts --project=webkit-desktop
```

`PLAYWRIGHT_IGNORE_HTTPS_ERRORS` only bypasses certificate validation; it does not
enable HTTPS on the application server. Leave it unset for trusted certificates.

Use `npm test -- --headed` to watch the
browser run the suite. Playwright writes an HTML report after each run; open it
with:

```sh
npx playwright show-report
```
