# Tournament UI coverage

These tests check what a tournament user can see and do: controls, navigation,
feedback, access, cross-client updates, and persistence. They **do not establish
correctness of points, standings order, tiebreakers, pairing algorithms, bye
awards, or automatic tournament scheduling**. A result applied during a journey
is only a prerequisite for another UI state. There are no backend-only WebSocket
tests, and no application changes or test hooks.

## Scenarios and purpose

The three specs in `tests/authenticated/tournaments` define 18 scenarios. Each
runs in the six existing browser/layout projects: **108 new browser cases**.
Together with the existing 17 scenarios, discovery should find **210 cases**.
The authenticated CI selection includes 192 cases; anonymous smoke contributes
the remaining 18. The three new database fixture checks run separately.

| Spec / journey | What is exercised and why |
| --- | --- |
| Registration / Browse | Status and personal tabs, case-insensitive search, and detail links make tournaments discoverable. |
| Registration / Configure | Mode, seats, invitation switch, time controls, date/manual switching, rating limits, and fixed-duration settings can be operated; the created tournament retains the final selections. This does not validate the scheduling job or duration enforcement. |
| Registration / Validate | Required text boundaries and preview/edit switching prevent incomplete submissions without losing Markdown. |
| Registration / Description | Cancel preserves the original, invalid text disables save, and a valid edited description survives reload. |
| Registration / Join and leave | Both clients see membership changes; membership survives reload. |
| Registration / Restricted entry | Full, invite-only, and lower/upper rating-boundary prerequisites disable Join. This checks the UI guard, not backend authorization. |
| Registration / Accept invitation | Search/invite, actionable notification, acceptance, and roster update connect organizer and invitee. |
| Registration / Decline, retract, remove | Invitation decline/retraction clear the corresponding UI; kicking a joined player restores their Join control. |
| Game controls / Lifecycle | Minimum-player and completion gates, Start/Finish, notifications, and the Completed list reflect the journey. Bulk results unlock Finish; their scoring is not checked. |
| Game controls / Agreed date | Proposal, acceptance, reload, public scheduling text, and cancellation stay consistent across both clients. |
| Game controls / Reject date | Only the recipient can accept/reject a proposal; rejection restores the proposal UI. |
| Game controls / Readiness | Game links lead to waiting controls; closing a readiness popup leaves the game waiting; accepting a request for another game opens its board. |
| Game controls / Individual adjudication | Result choices, cancellation, one representative applied result, and Delete/reset update the game controls. No exhaustive result matrix. |
| Game controls / Bulk adjudication | Both confirmation dialogs support cancellation; applying and undoing adjudications update pending/completion controls. |
| Game controls / Swiss | Admin format availability and next-round gating work. Subsequent opponents and game counts are deliberately not asserted. |
| Access / Roles | Visitors and members lack management controls; organizers and admins see them; an admin can delete an unstarted tournament. |
| Access / Chat persistence | Organizer and participant exchange messages and retain them after reload. |
| Access / Chat restrictions | Anonymous and authenticated outsiders receive a restriction notice; leaving removes chat input access. |

## Playwright practices

The suite follows Playwright's [best practices](https://playwright.dev/docs/best-practices),
[isolation guidance](https://playwright.dev/docs/browser-contexts), and
[retrying assertions](https://playwright.dev/docs/test-assertions):

- Each scenario owns fresh accounts and browser contexts, including each retry.
  Tests have no shared login state, serial dependencies, or ordering requirements.
- Use role, accessible name, label, placeholder, and visible text locators first.
  Locator assertions wait for outcomes on each client. There are no fixed sleeps,
  forced clicks, network-idle waits, or JavaScript calls into application state.
- Test steps describe user actions and outcomes in the existing report. Contexts
  inherit the configured viewport, browser, service-worker, and trace options.
- Setup and teardown have their own fixture budget. Scenario timeouts are 60 seconds,
  with 90 seconds for the longer lifecycle journey. Extra admin/outsider sign-ins
  are dependent fixtures with a separate 90-second setup allowance, rather than
  taking time from the UI journey. No global timeout/retry changes.
- Use real UI actions for the behavior being tested. SQL provides owned prerequisites
  and limited identity/settings readback; it does not perform the asserted transition.

Some existing controls need narrowly scoped DOM locators because application code
must remain unchanged. Switch inputs have no accessible names; helpers select the
switch beside its exact visible text. Min/Max player sliders have swapped `name`
attributes (`Seats` and `Min Seats`), so that mapping is documented beside the
form interaction. Sliders use keyboard input. Date controls repeat `id=start-time`
across game cards, so scheduling locators are scoped to a specific game's link.
Tournament-card links and some roster sections lack accessible names, requiring
scoping by visible tournament/player text. Semantic labels, unique IDs, and named
links would simplify these tests and improve accessibility; none are added here.

The reported admin-delete failure exposed an application race: the Delete handler
in `pages/tournament.rs` sends a WebSocket action and immediately navigates to the
list. `pages/tournaments.rs` loads that list with `OnceResource`, so a request that
beats deletion can retain the old card even after the `Deleted` update arrives.
Waiting longer on the same card does not refresh that snapshot. The role scenario
now waits for the participant's redirect, which the client performs on the server's
`Deleted` update, then loads a fresh list and asserts the card is absent after
loading succeeds. This establishes deletion persistence; it does not claim the
optimistic list refresh is correct. An application fix would navigate after the
delete acknowledgement or invalidate the list on `Deleted`. No app code or
expected-failure annotation is added for this workaround.
The corrected role scenario passed in all six projects (six cases, six workers),
and TypeScript and whitespace checks passed.

Creation has the same optimistic-navigation race: `pages/tournament_create.rs`
sends Create and navigates before the server acknowledges it, while the list does
not refresh on `Created`. The settings scenario therefore clicks Create once,
then uses `expect.toPass` with a 15-second limit to reload the Hosting list and
check for the unique tournament card. Only the UI read is retried; creation is
never resubmitted. It then opens the card and checks the persisted settings as
before. This verifies creation and persistence without claiming the initial list
refresh works. Navigating after `Created`, or invalidating the list on that update,
would remove this workaround; no application changes are made.
Validation repeated the corrected settings scenario three times in each of the
six projects: **18 passed**, including all nine mobile executions. TypeScript and
whitespace checks also passed.

## Persistent seed and local database update

There is one persistent seed source:
`db/testware/2026-09-05-000000_e2e_users/up.sql`. Its existing `down.sql` remains
the Diesel rollback companion. There is no second seed or upgrade migration.

The seed contains `admin_1`, `user_1`–`user_30`, and `SwissByePlayer`: **32 users,
192 ratings, and 32 notification-preference rows**. Existing IDs 1–31 are unchanged.
SwissByePlayer uses UUID `00000000-0000-4000-8000-000000000020`, normalized username
`swissbyeplayer`, email `SwissByePlayer@example.test`, verified email, and no admin
privileges. Like other seed accounts, its password is `password`. It has one
initial rating for each of Bullet, Blitz, Rapid, Classic, Correspondence, and
Puzzle: zero played/won/lost/drawn, rating 1500, deviation 500, volatility 0.09.
Notification preferences use the application's column defaults.

The application looks up SwissByePlayer even for an even-sized Swiss field. The
sentinel stays outside the ordinary account pool and is never used to sign in.

On September 13, 2026, the local `hive-local` database at `127.0.0.1:5433` was
updated separately. A one-off transaction evaluated the consolidated seed in
temporary staging tables, checked identity conflicts, and copied only missing
Swiss records to the application tables. It inserted **1 user, 6 ratings, and
1 preference row**. Before/after counts and row digests verified that all existing
users, ratings, preferences, games, tournaments, and migration-history rows were
unchanged. The transaction verified Swiss dependent records before committing.
The temporary update script is not a second maintained seed.

Editing an already-applied migration does not rerun it. A fresh disposable database
uses the consolidated seed normally. Another existing development database needs
the same explicitly targeted additive procedure, with conflict checks; do not
roll back its seed or reset existing ratings to upgrade it.

## Runtime prerequisites, ownership, and recovery

`support/tournaments/data.ts` creates temporary accounts with random UUIDs and
unique `tu_...` usernames. It copies only the test password hash from the seeded
`user_1`; existing accounts and ratings are not modified. Temporary users receive
verified test emails, the six initial ratings above, and default notification
preferences. Admin accounts are created only for privileged journeys.

Each seeded tournament has a unique name/ID, a fixture-owned organizer, optional
fixture-owned participants, valid default settings, and the status needed by that
journey. Discovery uses empty ongoing/completed fixtures solely to exercise tabs.
Game-control journeys start a small tournament through the UI so the application
creates its games. They resolve games through UI adjudication rather than seeding
computed outcomes. No standings oracle is maintained in this suite.

The fixture journals accounts and planned tournament names before insertion,
using an atomic manifest replacement, and holds a PostgreSQL session advisory
lock in a namespace separate from the existing account pool. A heartbeat detects
lost ownership and closes contexts. Setup is transactional. Cleanup finds a
UI-created tournament by its exact planned name and owned organizer, even if the
runner died before saving the returned ID. Foreign account identities,
participants, invitations, organizers, or games cause cleanup to refuse changes.
Manifest names must match their owner token, and stable seed identities cannot be
claimed as temporary accounts, even in an edited manifest.

Failure logs and accessibility snapshots are captured before contexts close.
Cleanup explicitly deletes owned games first because `games.tournament_id` has
no foreign key, then owned tournaments and users. Dependent memberships,
invitations, schedules, ratings, preferences, and chat records cascade. Cleanup
checks for remaining owned records. The persistent Swiss sentinel is retained.
Cleanup errors are attached without replacing an existing test failure.

An interrupted run leaves an ownership JSON manifest under
`test-results/tournament-ownership/`, outside Playwright's automatically cleaned
`e2e-tests` output. Cleanup failures also attach the manifest to the report.
Recover against the same
database with Node 24+ from `apis/end2end`:

```sh
PLAYWRIGHT_DATABASE_URL=postgres://hive-dev@127.0.0.1:5433/hive-local \
  node support/tournaments/recover.mjs /path/to/tournament-ownership.json
```

Recovery acquires the same advisory lock and refuses an active owner. Never edit
ownership manifests to claim additional records or point recovery at another
deployment. It is idempotent after successful cleanup.

## Running and interpreting results

```sh
npm run typecheck
npm run test:unit
PLAYWRIGHT_DATABASE_URL=postgres://hive-dev@127.0.0.1:5433/hive-local \
  npm run test:pool -- seed.spec.ts tournament_fixtures.spec.ts
PLAYWRIGHT_DATABASE_URL=postgres://hive-dev@127.0.0.1:5433/hive-local \
  npm test -- tests/authenticated/tournaments --project=chromium-desktop --workers=6
PLAYWRIGHT_DATABASE_URL=postgres://hive-dev@127.0.0.1:5433/hive-local \
  npm test -- tests/authenticated/tournaments --workers=6
PLAYWRIGHT_DATABASE_URL=postgres://hive-dev@127.0.0.1:5433/hive-local \
  npm test -- --workers=6
```

The three new fixture checks cover rollback after partial setup, targeted cleanup
and foreign-record protection, and recovery after a killed database session with
active-owner refusal. They use the existing disposable-database harness; they do
not run against application data. The updated seed lifecycle check also verifies
rollback/reapply and stable identities.

### Validation on September 13, 2026

Discovery found **210 browser cases in 11 files**, including all 108 new tournament
cases. Discovery counts are separate from execution results:

| Check | Executed outcome |
| --- | --- |
| TypeScript and whitespace checks | Passed (`npm run typecheck`, `git diff --check`). |
| Existing unit/helper suite | 42 passed. |
| Database integration suite | 13 passed, including the three new fixture checks. Seed and tournament fixture checks were rerun after recovery hardening: all four passed. |
| Chromium desktop, within the full matrix | All 35 passed, including all 18 tournament scenarios. |
| Full six-project matrix, six workers | All 210 executed: 204 passed, six tournament role cases timed out during additional authentication; all 102 existing browser cases passed. |
| Fresh focused rerun after the fixture correction | All 18 passed: role visibility, outsider chat, and Swiss admin scenarios across all six projects. |

The six failures used the earlier fixture implementation, which authenticated
additional roles inside the 60-second test body. Admin and outsider authentication
now use dependent fixtures with their own setup budget. The focused rerun verified
that correction, without increasing scenario timeouts. Across the full run and
focused rerun, every case has a passing execution; **there was no single clean
210-case run after the correction**. No skips or expected failures were added.
No application defect was confirmed during that initial validation; the subsequent
reported deletion race and its test workaround are documented above.

An earlier run at the unchanged default of 12 workers was stopped after Firefox
timeouts: 58 passed, seven failed, 11 interrupted, and 134 unexecuted. Local
validation therefore used six workers. The default concurrency has not been
cleanly validated with the final fixtures; the commands above make the tested
local concurrency explicit.

Final database inspection found two temporary accounts from the interrupted run.
The documented recovery command acquired their inactive manifest's lock and
removed only those accounts and their dependents. A subsequent inspection found
zero remaining manifest-owned users, ratings, preferences, tournaments, games,
schedules, or chat channels. The database retained all 32 seed accounts, including
SwissByePlayer with six ratings and one preference row.

The full run's HTML report is under `playwright-report/`; the focused rerun is
under `test-results/tournament-role-report/`, with separate trace output at
`test-results/tournament-role-validation/`. These local artifacts are ignored by
Git. A skipped, unexecuted, or expected-failed test is not passing feature coverage.
Confirm an application defect with a precise reproduction before adding narrowly
scoped expected-failure handling; unrelated failures must remain ordinary failures.
