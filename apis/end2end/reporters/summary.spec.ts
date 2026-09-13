import { expect, test } from "@playwright/test";
import { renderSummary, type Attempt, type Scenario, type Summary } from "./summary";

const passingAttempt: Attempt = {
  number: 1, status: "passed", duration: 1200, errors: [],
  steps: [{ title: "Sign in", status: "passed", duration: 1000, children: [
    { title: "Sign in as user_1", status: "passed", duration: 600, children: [] },
  ] }],
};
function scenario(overrides: Partial<Scenario> = {}): Scenario {
  return {
    key: "challenge", feature: "Challenges", title: "Accept a challenge",
    project: "chromium-desktop", location: { file: "tests/authenticated/challenges.spec.ts", line: 10 },
    status: "passed", attempts: [passingAttempt], ...overrides,
  };
}
function summary(scenarios: Scenario[], overrides: Partial<Summary> = {}): Summary {
  return {
    status: "passed", duration: 2000, projects: [...new Set(scenarios.map(test => test.project))],
    scenarios, errors: [], sourceBaseURL: "https://github.com/example/hive/blob/tested-commit", ...overrides,
  };
}

test.describe("Report rendering", () => {
  test("groups browsers by scenario and reports nested durations without summing them", () => {
    const report = renderSummary(summary([scenario(), scenario({ project: "webkit-mobile" })]));
    expect(report).toContain("1 scenarios · 2 browser/layout cases");
    expect(report).toContain("| Scenario | chromium desktop | webkit mobile |");
    expect(report).toMatch(/\| Accept a challenge \| \[✅ Passed · 1\.2 s\]\(#user-content-steps-[a-f0-9]+\) \| \[✅ Passed · 1\.2 s\]\(#user-content-steps-[a-f0-9]+\) \|/);
    expect(report.match(/<summary>Challenges: Accept a challenge<\/summary>/g)).toHaveLength(1);
    expect(report).toContain("<summary>chromium desktop · ✅ Passed · 1.2 s</summary>");
    expect(report).toContain("<summary>webkit mobile · ✅ Passed · 1.2 s</summary>");
    expect(report).toContain("  - ✅ Passed — Sign in as user&#95;1 · 600 ms");
    expect(report).toContain("/blob/tested-commit/tests/authenticated/challenges.spec.ts#L10");
  });

  test("matrix links target unique steps inside both collapsed groups in project order", () => {
    const cases = [scenario({ project: "webkit-mobile" }), scenario(), scenario({ key: "other-file" })];
    const report = renderSummary(summary(cases, { projects: ["chromium-desktop", "webkit-mobile", "firefox-desktop"] }));
    const links = [...report.matchAll(/\]\(#user-content-(steps-[a-f0-9]+)\)/g)].map(match => match[1]);
    const anchors = [...report.matchAll(/<a name="(steps-[a-f0-9]+)"><\/a>/g)].map(match => match[1]);
    expect(new Set(links).size).toBe(3);
    expect(anchors).toEqual(links);
    expect(report).toContain("— Not selected");
    expect(report).not.toContain("[— Not selected]");
    expect(report).not.toContain("<details open");
    const steps = report.slice(report.indexOf("## Executed steps"));
    let depth = 0;
    for (const [tag] of steps.matchAll(/<details>|<\/details>|<a name="steps-[a-f0-9]+"><\/a>/g)) {
      if (tag === "<details>") depth++;
      else if (tag === "</details>") depth--;
      else expect(depth).toBe(2);
      expect(depth).toBeGreaterThanOrEqual(0);
    }
    expect(depth).toBe(0);
    expect(steps.indexOf("<summary>chromium desktop")).toBeLessThan(steps.indexOf("<summary>webkit mobile"));
    const reordered = renderSummary(summary([...cases].reverse()));
    expect([...reordered.matchAll(/<a name="(steps-[a-f0-9]+)"><\/a>/g)].map(match => match[1]).sort()).toEqual([...anchors].sort());
  });

  test("retains failed attempts for flaky cases and surfaces the deepest failed step", () => {
    const failed: Attempt = {
      number: 1, status: "failed", duration: 800, errors: ["Expected same game URL"],
      steps: [{ title: "Start game", status: "failed", duration: 800, children: [
        { title: "Accept challenge", status: "failed", duration: 300, children: [] },
      ] }],
    };
    const report = renderSummary(summary([scenario({ status: "flaky", attempts: [failed, { ...passingAttempt, number: 2 }] })]));
    expect(report).toContain("1 flaky");
    expect(report).toContain("⚠️ Flaky · 2.0 s");
    expect(report).toContain("Step: Start game › Accept challenge");
    expect(report).toContain("Attempt 1: ❌ Failed");
    expect(report).toContain("Attempt 2: ✅ Passed");
    expect(report).toContain("<details>\n<summary>⚠️ Flaky — Accept a challenge · chromium desktop</summary>");
    expect(report).not.toContain("### ⚠️ Flaky");
    expect(report.indexOf("## Needs attention")).toBeLessThan(report.indexOf("## Scenario matrix"));
  });

  test("keeps final failures expanded while flaky diagnostics are collapsible", () => {
    const report = renderSummary(summary([scenario({ status: "failed", attempts: [
      { ...passingAttempt, status: "failed", errors: ["Still failing"] },
    ] })]));
    expect(report).toContain("### ❌ Failed — Accept a challenge · chromium desktop");
    expect(report).toContain("> Still failing");
    expect(report).not.toContain("<summary>❌ Failed — Accept a challenge");
  });

  test("distinguishes skipped, interrupted, timed out, expected failure and unexecuted cases", () => {
    const statuses = ["skipped", "interrupted", "timed out", "expected failure", "not run"] as const;
    const report = renderSummary(summary(statuses.map(status => scenario({
      key: status, title: status, status, attempts: status === "not run" ? [] : [{ ...passingAttempt, status }],
    }))));
    for (const status of statuses) expect(report).toContain(`1 ${status}`);
    expect(report).toContain("This test was not executed.");
    expect(report).toContain("0 passed");
  });

  test("does not merge equal titles from different source identities", () => {
    const report = renderSummary(summary([scenario(), scenario({ key: "other-file" })]));
    expect(report).toContain("2 scenarios · 2 browser/layout cases");
    expect(report.match(/\| Accept a challenge \|/g)).toHaveLength(2);
    expect(report.match(/<summary>Challenges: Accept a challenge<\/summary>/g)).toHaveLength(2);
  });

  test("truncation keeps complete priority groups and marks omitted matrix destinations", () => {
    const large = scenario({ key: "large", title: "Large passing test", attempts: [{
      ...passingAttempt,
      steps: Array.from({ length: 100 }, () => ({ title: "Long passing step", status: "passed" as const, duration: 1, children: [] })),
    }] });
    const failed = scenario({ key: "priority", title: "Priority test", status: "failed", attempts: [{
      ...passingAttempt, status: "failed", errors: ["Keep this failure"],
    }] });
    const passedBrowser = scenario({ key: "priority", title: "Priority test", project: "webkit-mobile" });
    const cases = [large, scenario({ key: "ordinary", title: "Ordinary test" }), passedBrowser, failed];
    const full = renderSummary(summary(cases));
    const report = renderSummary(summary(cases), 5000);
    expect(Buffer.byteLength(report)).toBeLessThanOrEqual(5000);
    expect(report).toContain("Keep this failure");
    expect(report).toContain("| Large passing test | ✅ Passed · 1.2 s · Details omitted | — Not selected |");
    expect(report).not.toContain("<summary>Challenges: Large passing test</summary>");
    const steps = report.slice(report.indexOf("## Executed steps"));
    expect(steps).toContain("<summary>Challenges: Priority test</summary>");
    expect(steps).toContain("<summary>chromium desktop · ❌ Failed · 1.2 s</summary>");
    expect(steps).toContain("<summary>webkit mobile · ✅ Passed · 1.2 s</summary>");
    expect(steps.indexOf("Priority test")).toBeLessThan(steps.indexOf("Ordinary test"));
    expect(report).toContain("Summary shortened to fit GitHub");
    expect(report.match(/<details>/g)?.length).toBe(report.match(/<\/details>/g)?.length);
    const links = [...report.matchAll(/\]\(#user-content-(steps-[a-f0-9]+)\)/g)].map(match => match[1]);
    expect(links.length).toBeGreaterThan(0);
    for (const anchor of links) {
      expect(report.split(`<a name="${anchor}"></a>`)).toHaveLength(2);
      expect(full).toContain(`](#user-content-${anchor})`);
    }
  });

  test("escapes Markdown and HTML, strips terminal codes and bounds error messages", () => {
    const report = renderSummary(summary([scenario({
      title: '<script>|[link](https://bad.test)\n**title**', status: "failed",
      attempts: [{ ...passingAttempt, status: "failed", errors: ['\x1b[31m<script>bad</script>\x1b[0m' + 'x'.repeat(20_000)] }],
    })], { errors: ["Global worker error"] }));
    expect(report).not.toContain("<script>");
    expect(report).not.toContain("\x1b[");
    expect(report).toContain("&lt;script&gt;&#124;&#91;link&#93;");
    expect(report).toContain("## Run errors");
    expect(report).toContain("Setup, teardown, or test body outside a named step");
    expect(report.length).toBeLessThan(10_000);
  });

  test("bounds large reports, preserves failure detail first and closes details blocks", () => {
    const failed = scenario({ key: "failed", status: "failed", title: "Important failure", attempts: [
      { ...passingAttempt, status: "failed", errors: ["Do not lose this failure"] },
    ] });
    const cases = [...Array.from({ length: 200 }, (_, index) => scenario({ key: String(index), title: `Passing ${index}` })), failed];
    const report = renderSummary(summary(cases), 6000);
    expect(Buffer.byteLength(report)).toBeLessThanOrEqual(6000);
    expect(report).toContain("Do not lose this failure");
    expect(report).toContain("Summary shortened to fit GitHub");
    expect(report.match(/<details>/g)?.length).toBe(report.match(/<\/details>/g)?.length);
  });

  test("handles discovery errors without claiming tests passed", () => {
    const report = renderSummary(summary([], { status: "failed", errors: ["Cannot load test module"] }));
    expect(report).toContain("Playwright results: failed");
    expect(report).toContain("0 scenarios · 0 browser/layout cases");
    expect(report).toContain("Cannot load test module");
  });
});
