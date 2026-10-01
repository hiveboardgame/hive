import { createHash } from "node:crypto";
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

function caseAnchor(scenario: Scenario) {
  return `steps-${createHash("sha256").update(JSON.stringify([scenario.key, scenario.project])).digest("hex")}`;
}

function caseResult(scenario: Scenario) {
  const duration = scenario.attempts.reduce((total, attempt) => total + attempt.duration, 0);
  return `${labels[scenario.status]}${scenario.attempts.length ? ` · ${time(duration)}` : ""}`;
}

function details(summary: Summary, scenario: Scenario) {
  return `<details>\n<summary>${projectName(scenario.project)} · ${caseResult(scenario)}</summary>\n\n`
    + `<a name="${caseAnchor(scenario)}"></a>\n\n`
    + `${sourceLink(summary, scenario.location)}\n\n`
    + (scenario.attempts.length ? scenario.attempts.map(attempt =>
      attemptHeading(attempt)
      + (stepLines(attempt.steps) || "No named steps executed.\n") + "\n"
      + errorQuotes(attempt.errors)
    ).join("") : "This test was not executed.\n\n")
    + "</details>\n\n";
}

function testDetails(summary: Summary, cases: Scenario[]) {
  const first = cases[0];
  const ordered = [...cases].sort((a, b) => summary.projects.indexOf(a.project) - summary.projects.indexOf(b.project));
  return `<details>\n<summary>${escapeText(first.feature)}: ${escapeText(first.title)}</summary>\n\n`
    + ordered.map(scenario => details(summary, scenario)).join("")
    + "</details>\n\n";
}

function matrix(summary: Summary, feature: string, scenarios: Scenario[], included?: Set<string>) {
  const rows = Map.groupBy(scenarios, scenario => scenario.key);
  return `### ${escapeText(feature)}\n\n`
    + `| Scenario | ${summary.projects.map(projectName).join(" | ")} |\n`
    + `| --- | ${summary.projects.map(() => "---").join(" | ")} |\n`
    + [...rows.values()].map(cases => {
      const cells = summary.projects.map(project => {
        const scenario = cases.find(candidate => candidate.project === project);
        if (!scenario) return "— Not selected";
        const result = caseResult(scenario);
        // GitHub prefixes custom anchor names during Markdown sanitization.
        // Target the rendered name so native navigation reveals closed details.
        return !included || included.has(scenario.key)
          ? `[${result}](#user-content-${caseAnchor(scenario)})`
          : `${result} · Details omitted`;
      });
      return `| ${escapeText(cases[0].title)} | ${cells.join(" | ")} |\n`;
    }).join("") + "\n";
}

export function renderSummary(summary: Summary, maxBytes = 900_000) {
  const counts = Object.keys(labels).map(status => {
    const count = summary.scenarios.filter(scenario => scenario.status === status).length;
    return `${count} ${status}`;
  });
  const scenarioCount = new Set(summary.scenarios.map(scenario => scenario.key)).size;
  const sections: { text: string; key?: string; render?: (included: Set<string>) => string }[] = [
    { text: `# Playwright results: ${escapeText(summary.status)}\n\n`
      + `**${scenarioCount} scenarios · ${summary.scenarios.length} browser/layout cases · ${time(summary.duration)} elapsed**\n\n`
      + `${counts.join(" · ")}\n\n`
      + `Coverage: ${summary.projects.map(projectName).join(", ") || "No projects executed"}.\n\n`
      + "Case durations include all attempts. Step durations are nested and must not be added together.\n\n"
      + "Select a matrix result to open its test and browser/layout steps.\n\n" },
  ];
  if (summary.errors.length) {
    sections.push({ text: "## Run errors\n\n" + errorQuotes(summary.errors) });
  }
  const problems = summary.scenarios.filter(needsAttention);
  if (problems.length) sections.push({ text: "## Needs attention\n\n" }, ...problems.map(scenario => ({ text: attention(summary, scenario) })));

  // Use source identity as well as titles: equally named tests in different
  // files or describe groups must never overwrite each other in the matrix.
  const features = Map.groupBy(summary.scenarios, scenario => scenario.feature);
  sections.push({ text: "## Scenario matrix\n\n" });
  for (const [feature, scenarios] of features) {
    sections.push({
      text: matrix(summary, feature, scenarios),
      render: included => matrix(summary, feature, scenarios, included),
    });
  }
  sections.push({ text: "## Executed steps\n\n" });
  // Keep whole tests together and prioritize groups with failures or retries.
  const groups = [...Map.groupBy(summary.scenarios, scenario => scenario.key).values()];
  const priority = (cases: Scenario[]) => cases.some(scenario => needsAttention(scenario) || scenario.attempts.length > 1);
  groups.sort((a, b) => Number(priority(b)) - Number(priority(a)));
  sections.push(...groups.map(cases => ({ text: testDetails(summary, cases), key: cases[0].key })));

  const notice = "\n**Summary shortened to fit GitHub. Download the complete Playwright HTML report for omitted details.**\n";
  const retained: typeof sections = [];
  const included = new Set<string>();
  let bytes = Buffer.byteLength(notice);
  let omitted = false;
  for (const section of sections) {
    if (bytes + Buffer.byteLength(section.text) > maxBytes) {
      omitted = true;
      continue;
    }
    retained.push(section);
    bytes += Buffer.byteLength(section.text);
    if (section.key !== undefined) included.add(section.key);
  }
  // Reserve linked cells above; replacing an unavailable link with the shorter
  // omission label cannot exceed that budget or leave a dangling destination.
  const output = retained.map(section => section.render ? section.render(included) : section.text).join("");
  return output + (omitted ? notice : "");
}
