# Browser checks

This suite covers browser wiring that Rust tests cannot establish: SVG hit testing,
mouse/touch event delivery, responsive controls, analysis file operations, session
transitions, and actual cross-client updates. Rules, pairing, scoring, transaction,
authorization, and serialization permutations remain in their Rust suites.

## Run locally

Use the native Hive environment for this checkout, replacing `<instance>` with its
managed instance name (`hive` for the main checkout):

```sh
hive-run <instance> shell
```

Build with the native watcher in tmux (`hive-run <instance> watch`) or, when no
watcher is active, build both feature graphs with `cargo leptos build`. Wait for
both watcher builds to finish. Then, from that instance shell:

```sh
cargo build -p db --features test-fixtures --bin hive-e2e
cd apis/end2end
npm ci
npx playwright install chromium firefox webkit
npm run typecheck
# llvm-strip must be on PATH (provided by LLVM or the flake dev shell).
npm run test:runner
npm test
```

`npm test` creates an empty `hive_e2e_test_<random>` database using the selected
`TEST_DATABASE_URL` connection. That role needs permission to create databases.
The runner snapshots this checkout's site assets under `target/e2e-sites/<run>` and
uses `llvm-strip --strip-all` to write the source WASM directly into the snapshot.
This removes non-executable debug symbols (the development name section can exceed
250 MB); the watcher keeps its original assets. The snapshot is removed after the run.
The runner starts this checkout's debug application on a free loopback port; normal
startup applies the production migrations. It stops its own process and drops only
its generated database on success, failure, or SIGINT/SIGTERM interruption. It never resets the
development database or the shared Rust integration-test database.

The app uses a debug build with normal development HTTP cookies. Tests do not
intercept API responses, rewrite cookies, or replace application WebSocket messages.
The reconnect scenario deliberately severs and restores one real WebSocket transport.
Service workers are disabled: these checks do not claim PWA/offline-shell coverage.

The fixture executable creates unique users, games, and arenas through production
Rust models and commands in a transaction. It returns test credentials and public
identities as JSON. It accepts only loopback `hive_e2e_test_<32 hex digits>` databases owned by a browser
run, rejecting development, shared Rust test, and seed databases. The executable
is available only with the `test-fixtures` feature; it is not an HTTP test endpoint.
Its two unit tests protect the database-target boundary before a connection opens:
reject remote/development/shared-test/seed targets and accept run-owned local targets.
Ordinary tests authenticate through the real login action, including its CSRF form
token. Signup, logout, and account deletion are explicitly exercised through the UI.
Account deletion is never fixture cleanup.

## CI asset build

CI builds the debug server once and the optimized frontend once. Native dev/test
profiles disable debug-info generation up front. The frontend retains development
assertions to match the server's HTTP cookie policy, while stripping symbols, enabling
LTO/size optimization and running `wasm-opt` through cargo-leptos.
Both builds use erased components so their hydration markers agree. `LEPTOS_WATCH`
is unset before compilation because Leptos also embeds compile-time reload markers:

```sh
env -u LEPTOS_WATCH LEPTOS_SITE_ROOT=target/e2e-site cargo leptos build --server-only
env -u LEPTOS_WATCH LEPTOS_SITE_ROOT=target/e2e-site \
  CARGO_PROFILE_WASM_RELEASE_DEBUG_ASSERTIONS=true \
  CARGO_PROFILE_WASM_RELEASE_STRIP=symbols \
  RUSTFLAGS='-Z threads=8 --cfg erase_components' \
  cargo leptos build --release --frontend-only
cd apis/end2end
PLAYWRIGHT_SITE_ROOT=target/e2e-site npm test
```

CI also strips the server and fixture executables after building them.
Only test reports, failure traces/screenshots, and logs are uploaded, not build outputs.
This separate output leaves an active watcher's site assets untouched. The local
watcher build remains supported for quick iteration; it receives the same final
symbol stripping when copied by the runner.

CI caches the pinned cargo-leptos executable separately from Rust dependencies.
Its cache key includes the Ubuntu image, architecture, tool version, and pinned
Rust toolchain. The dependency cache covers Cargo downloads, compiled dependencies
under `.cargo/target` (including frontend builds), and cargo-leptos's downloaded
frontend tools. It accounts for the compiler, manifests, lockfile, Cargo config,
and workflow build flags; workspace code and site assets are rebuilt each run.
Build caches survive browser-test failures. Relevant pushes to `main` populate
caches that new pull requests can reuse; the first run for a new toolchain or
dependency set may still need a full build.

An inexpensive change-detection job examines the whole pull request from its merge
base, or all commits in a push to `main`. Changes limited to `scripts/`,
`hive-hydra/`, `docs/`, or repository documentation skip the browser build and suite.
Documentation includes root Markdown files, `apis/README.md`,
`apis/end2end/README.md`, and README files outside the application crates.
Cargo manifests and lockfiles, toolchain files, and `.cargo/`
configuration always count as relevant, including those inside otherwise excluded
directories. Other paths run the suite, including application Markdown assets,
migrations, runtime configuration, and the browser workflow itself. Renames check
both old and new paths; uncertain comparisons run the suite.

The `browser` check always reports the final result. It passes only when the suite
passed or change detection confirmed that it was unnecessary; failed detection,
missing results, and cancelled required tests cannot produce a passing check.
Other workflows keep their existing triggers and checks.

## Select and repeat checks

Arguments after `--` are passed to Playwright:

```sh
npm test -- tests/analysis.spec.ts --project=firefox-narrow
npm test -- tests/play.spec.ts --project=chromium-desktop --workers=1
npm test -- --repeat-each=3
PLAYWRIGHT_FULL_MATRIX=1 npm test
PLAYWRIGHT_FULL_MATRIX=1 npm test -- --repeat-each=3 --workers=1
npm run test:list
```

The default matrix runs all journeys on Chromium desktop and mobile. Shared
interaction and navigation checks also run on Firefox desktop/narrow and WebKit
desktop/mobile. Mouse gesture cases run on mouse projects; native long-press and
pinch use Chromium's touch-input protocol and run on Chromium mobile only. WebKit
mobile uses real tap input for the shared analysis/file journeys. Firefox narrow is
desktop Firefox at a handset viewport, **not Firefox Android touch coverage**.

No tests are skipped or expected to fail. Retries are disabled, and flaky outcomes
fail the run even if retries are explicitly enabled for investigation. CI uses one
worker; local runs default to two. Per-test deadlines remain bounded, while full-matrix and repeat runs have no fixed suite deadline. Use `--global-timeout=<milliseconds>` when an overall limit is needed; CI retains its job deadline. Acceptance includes repeated first-attempt passes
at the configured concurrency. CI evaluates pushes to `main` and pull requests
targeting `main`, running the default matrix when the changes are relevant.
Manual workflow dispatch always runs browser tests and can select `full_matrix`
to run every compatible scenario across browsers and layouts.

Document reloads wait for the visible main landmark before checking page-specific
state. WASM download/compilation has its own bounded readiness wait; an ordinary
content assertion does not double as a startup deadline. Each page has one main
landmark, independent of CSS classes or inner layout containers.

The pan case controls animation frames to release the mouse before queued motion
renders, reproducing the lost-motion regression independently of machine speed.
It then resumes time for wheel zoom and hit testing. Pan assertions compare the
viewport origin and zoom assertions compare its size, without fixing pixel values
or accepting a resize as evidence of panning. Repeated passes provide evidence
against intermittent failures; they cannot guarantee every future change or runner.

`PLAYWRIGHT_APP_BIN` and `PLAYWRIGHT_FIXTURE_BIN` can select binaries built in this
checkout; defaults are `.cargo/target/debug/apis` and `.cargo/target/debug/hive-e2e`,
with `CARGO_TARGET_DIR` respected. Site assets default to this checkout's `target/site`;
`PLAYWRIGHT_SITE_ROOT` can select its separately built `target/e2e-site`.
`npm run test:attached` is a debugging escape hatch for an already running isolated
app: set both `PLAYWRIGHT_BASE_URL` and `PLAYWRIGHT_DATABASE_URL` for that same app.
It does not create or dispose the attached database, which must still have a run-owned `hive_e2e_test_<32 hex digits>` name.

## Coverage and why each scenario exists

| Scenario | Browser invariant |
| --- | --- |
| Analysis branch/history | Clicking or tapping changes the intended variation; board and reserve follow the selected node. |
| Reserve and board dragging | Native pointer motion crosses SVGs and lands on the intended target/stack. |
| Stack inspection and annotation | Holding/releasing the right button inspects a stack; annotation mode receives its intended gesture. |
| Pan and wheel zoom | Transformed SVG coordinates still select and move the intended piece. |
| Touch long press and pinch | Browser-generated touch events expand/collapse and zoom without leaving the board unusable. |
| Analysis save/load | A real downloaded file can be uploaded and restores the selected branch. |
| Challenge to live game | Real acceptance opens a playable board; moves reach the other client and survive reload. |
| Reconnect | A client reconciles a move missed during a severed connection and can continue playing. |
| Live history/takeback | Reviewing history remains local; accepting a control updates both live boards and persisted state. |
| Chat | The composer, remote delivery, message list, and persisted history connect correctly. |
| Signup/logout/login | A newly created session can immediately act; logout revokes access and credentials can establish a new session. |
| Account deletion | The password form, server deletion, and client session transition prevent later access. |
| Arena | Join/pause/resume persist; an assignment reaches the correct board and its result is accessible from the arena. |
| Guest navigation | Responsive navigation is operable and starting play leads to authentication. |

The single opening definition in `assets/stacked-opening.json` describes a legal
seven-ply position without player identities. Analysis uploads its generated PGN
through the actual file control. Prepared live games and browser move expectations
use the same definition through production game commands. Neither depends on
a scrubbed database dump, static accounts, shared-account locks, or UI cleanup.

Each run owns `test-results/runs/<run>/`: built-in HTML/JUnit reports, screenshots,
traces, `server.log`, and browser diagnostics captured before contexts close. Concurrent
runs do not erase one another's reports. `npm run test:runner` additionally verifies
that application signal termination, delayed output, and interruption during startup
cannot strand run-owned databases or copied assets. It also checks that an unset
`TEST_DATABASE_URL` produces setup guidance before resources are started.
A runtime product defect remains a failure until fixed; do not add a skip, forced
click, hidden-state override, or mutation retry to make the report green.
