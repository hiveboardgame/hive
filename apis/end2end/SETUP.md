# Playwright Setup Instructions

## Quick Start (Recommended)

```bash
cd apis/end2end
npm install
npx playwright install chromium firefox
npm test
```

## GitHub Actions

The CI workflow includes `--with-deps` flag which automatically installs system dependencies:

```bash
npx playwright install --with-deps
```

## Alternative: Use Playwright with System Chrome

If you prefer using your system Chrome instead:

```bash
# Install only Playwright without browsers
npm install @playwright/test
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm install

# Use system Chrome
export PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH="/usr/bin/google-chrome"
npm test
```
