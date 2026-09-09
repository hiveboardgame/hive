import { stripVTControlCharacters } from "node:util";

export type Status = "passed" | "failed" | "flaky" | "skipped" | "timed out"
  | "interrupted" | "not run" | "expected failure";
export type Location = { file: string; line: number };
export type Step = {
  title: string;
  status: Status;
  duration: number;
  children: Step[];
};
export type Attempt = {
  number: number;
  status: Status;
  duration: number;
  steps: Step[];
  errors: string[];
  failureLocation?: Location;
};
export type Scenario = {
  key: string;
  feature: string;
  title: string;
  project: string;
  location: Location;
  status: Status;
  attempts: Attempt[];
};
export type Summary = {
  status: string;
  duration: number;
  projects: string[];
  scenarios: Scenario[];
  errors: string[];
  sourceBaseURL?: string;
};

const labels: Record<Status, string> = {
  passed: "✅ Passed",
  failed: "❌ Failed",
  flaky: "⚠️ Flaky",
  skipped: "⏭️ Skipped",
  "timed out": "⏱️ Timed out",
  interrupted: "🛑 Interrupted",
  "not run": "➖ Not run",
  "expected failure": "✅ Expected failure",
};

// Test names and assertion messages are data, never Markdown or HTML markup.
function escapeText(value: string, limit = 300) {
  const clean = stripVTControlCharacters(value).replace(/[\r\n]+/g, " ");
  const bounded = clean.length > limit ? `${clean.slice(0, limit)}…` : clean;
  return bounded.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/[\\`*_{}\[\]()#+.!|~]/g, character => `&#${character.charCodeAt(0)};`);
}

function time(milliseconds: number) {
  return milliseconds < 1000 ? `${Math.round(milliseconds)} ms` : `${(milliseconds / 1000).toFixed(1)} s`;
}

function projectName(project: string) {
  return escapeText(project.replace(/-/g, " "));
}

function errorQuotes(errors: string[]) {
  return errors.map(error => `> ${escapeText(error, 2000)}\n\n`).join("");
}

function attemptHeading(attempt: Attempt) {
  return `**Attempt ${attempt.number}: ${labels[attempt.status]}** · ${time(attempt.duration)}\n\n`;
}

function sourceLink(summary: Summary, location: Location) {
  const label = `${escapeText(location.file)}:${location.line}`;
  if (!summary.sourceBaseURL || location.file.startsWith("../")) return label;
  const file = location.file.split("/").map(encodeURIComponent).join("/");
  return `[${label}](${summary.sourceBaseURL}/${file}#L${location.line})`;
}

function needsAttention(scenario: Scenario) {
  return ["failed", "flaky", "timed out", "interrupted"].includes(scenario.status);
}

function failedPaths(steps: Step[], parents: string[] = []): string[] {
  return steps.flatMap(step => {
    const path = [...parents, step.title];
    const children = failedPaths(step.children, path);
    if (children.length) return children;
    return ["failed", "timed out", "interrupted"].includes(step.status) ? [path.join(" › ")] : [];
  });
}

function attention(summary: Summary, scenario: Scenario) {
  const attempts = scenario.attempts.filter(attempt => !["passed", "skipped"].includes(attempt.status));
  const title = `${labels[scenario.status]} — ${escapeText(scenario.title)} · ${projectName(scenario.project)}`;
  const body = attempts.map(attempt => {
    const failed = failedPaths(attempt.steps);
    return attemptHeading(attempt)
      + `Step: ${failed.length ? failed.map(path => escapeText(path, 600)).join("; ") : "Setup, teardown, or test body outside a named step"}\n\n`
      + errorQuotes(attempt.errors)
      + `${sourceLink(summary, attempt.failureLocation ?? scenario.location)}\n\n`;
  }).join("");
  // Retried passes remain visible in the matrix; their failure diagnostics can
  // be expanded when needed without crowding out tests that still fail.
  return scenario.status === "flaky"
    ? `<details>\n<summary>${title}</summary>\n\n${body}</details>\n\n`
    : `### ${title}\n\n${body}`;
}

function stepLines(steps: Step[], depth = 0): string {
  return steps.map(step => `${"  ".repeat(depth)}- ${labels[step.status]} — ${escapeText(step.title)} · ${time(step.duration)}\n`
    + stepLines(step.children, depth + 1)).join("");
}

function details(summary: Summary, scenario: Scenario) {
  return `<details>\n<summary>${labels[scenario.status]} — ${escapeText(scenario.feature)}: ${escapeText(scenario.title)} · ${projectName(scenario.project)}</summary>\n\n`
    + `${sourceLink(summary, scenario.location)}\n\n`
    + (scenario.attempts.length ? scenario.attempts.map(attempt =>
      attemptHeading(attempt)
      + (stepLines(attempt.steps) || "No named steps executed.\n") + "\n"
      + errorQuotes(attempt.errors)
    ).join("") : "This test was not executed.\n\n")
    + "</details>\n\n";
}

export function renderSummary(summary: Summary, maxBytes = 900_000) {
  const counts = Object.keys(labels).map(status => {
    const count = summary.scenarios.filter(scenario => scenario.status === status).length;
    return `${count} ${status}`;
  });
  const scenarioCount = new Set(summary.scenarios.map(scenario => scenario.key)).size;
  const sections = [
    `# Playwright results: ${escapeText(summary.status)}\n\n`
      + `**${scenarioCount} scenarios · ${summary.scenarios.length} browser/layout cases · ${time(summary.duration)} elapsed**\n\n`
      + `${counts.join(" · ")}\n\n`
      + `Coverage: ${summary.projects.map(projectName).join(", ") || "No projects executed"}.\n\n`
      + "Case durations include all attempts. Step durations are nested and must not be added together.\n\n",
  ];
  if (summary.errors.length) {
    sections.push("## Run errors\n\n" + errorQuotes(summary.errors));
  }
  const problems = summary.scenarios.filter(needsAttention);
  if (problems.length) sections.push("## Needs attention\n\n", ...problems.map(scenario => attention(summary, scenario)));

  // Use source identity as well as titles: equally named tests in different
  // files or describe groups must never overwrite each other in the matrix.
  const features = Map.groupBy(summary.scenarios, scenario => scenario.feature);
  sections.push("## Scenario matrix\n\n");
  for (const [feature, scenarios] of features) {
    const rows = Map.groupBy(scenarios, scenario => scenario.key);
    sections.push(`### ${escapeText(feature)}\n\n`
      + `| Scenario | ${summary.projects.map(projectName).join(" | ")} |\n`
      + `| --- | ${summary.projects.map(() => "---").join(" | ")} |\n`
      + [...rows.values()].map(cases => {
        const cells = summary.projects.map(project => {
          const scenario = cases.find(candidate => candidate.project === project);
          if (!scenario) return "— Not selected";
          const duration = scenario.attempts.reduce((total, attempt) => total + attempt.duration, 0);
          return `${labels[scenario.status]}${scenario.attempts.length ? ` · ${time(duration)}` : ""}`;
        });
        return `| ${escapeText(cases[0].title)} | ${cells.join(" | ")} |\n`;
      }).join("") + "\n");
  }
  sections.push("## Executed steps\n\n");
  // Keep failure/retry details ahead of successful detail blocks when bounded.
  const ordered = [...problems, ...summary.scenarios.filter(scenario => !needsAttention(scenario))];
  sections.push(...ordered.map(scenario => details(summary, scenario)));

  const notice = "\n**Summary shortened to fit GitHub. Download the complete Playwright HTML report for omitted details.**\n";
  let output = "";
  let omitted = false;
  for (const section of sections) {
    if (Buffer.byteLength(output) + Buffer.byteLength(section) + Buffer.byteLength(notice) > maxBytes) {
      omitted = true;
      continue;
    }
    output += section;
  }
  return output + (omitted ? notice : "");
}
