import { appendFileSync, readFileSync } from "node:fs";

const destination = process.env.GITHUB_STEP_SUMMARY;
if (!destination) throw new Error("GITHUB_STEP_SUMMARY is required to publish test results");

let summary;
try {
  summary = readFileSync("test-results/summary.md", "utf8");
  if (!summary.trim()) throw new Error("Empty summary");
} catch {
  summary = "# Playwright results unavailable\n\nThe test runner did not produce a summary. Inspect the test logs and diagnostic artifacts.\n";
  process.exitCode = 1;
}

const reportURL = process.env.PLAYWRIGHT_REPORT_URL;
if (reportURL && /^https:\/\/[^\s<>()[\]]+$/.test(reportURL)) {
  summary += `\n[Download the Playwright report and server diagnostics](${reportURL}) (available for seven days).\n\n`
    + "Extract the archive, then run `npx playwright show-report <extracted-folder>/apis/end2end/playwright-report` from the E2E directory.\n";
} else {
  summary += "\nThe HTML report download is unavailable; inspect the artifact upload step.\n";
}
appendFileSync(destination, summary);
