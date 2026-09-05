# Hive Playwright end-to-end tests

The smoke suite runs against a Hive server that you start locally. It covers
Chromium, Firefox, and WebKit at desktop and mobile layouts.

## Setup

This project uses the Manjaro `playwright` package rather than npm. The test
commands set `NODE_PATH` so Node can load Playwright's system-installed test
module from `/usr/lib/node_modules`.

## Run

Load the E2E fixture, then start Hive from the repository root in one terminal:

```sh
./scripts/reset-testware.sh
cargo leptos watch --hot-reload
```

Once the app is serving on port 3000, run the tests in another terminal:

```sh
cd apis/end2end
NODE_PATH=/usr/lib/node_modules playwright test
```

To run against a different deployment, provide its base URL:

```sh
PLAYWRIGHT_BASE_URL=https://example.test NODE_PATH=/usr/lib/node_modules playwright test
```

Use `NODE_PATH=/usr/lib/node_modules playwright test --headed` to watch the
browser run the suite. Playwright writes an HTML report after each run; open it
with:

```sh
npx playwright show-report
```
