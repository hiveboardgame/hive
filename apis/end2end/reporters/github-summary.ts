import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { FullConfig, FullResult, Reporter, Suite, TestCase, TestError, TestResult, TestStep } from "@playwright/test/reporter";
import { renderSummary, type Attempt, type Location, type Scenario, type Status, type Step } from "./summary";

function message(error: TestError) {
  return error.message ?? error.value ?? error.stack ?? "Unknown test error";
}

function resultStatus(status: TestResult["status"]): Status {
  return status === "timedOut" ? "timed out" : status;
}

export default class GitHubSummaryReporter implements Reporter {
  private suite?: Suite;
  private rootDir = process.env.GITHUB_WORKSPACE ?? process.cwd();
  private completed = new WeakSet<TestStep>();
  private errors: string[] = [];
  private outputFile: string;

  constructor(options: { outputFile?: string } = {}) {
    this.outputFile = options.outputFile ?? "test-results/summary.md";
  }

  printsToStdio() { return false; }

  onBegin(_config: FullConfig, suite: Suite) { this.suite = suite; }

  onStepEnd(_test: TestCase, _result: TestResult, step: TestStep) {
    this.completed.add(step);
  }

  onError(error: TestError) { this.errors.push(message(error)); }

  private location(location: Location): Location {
    return { file: path.relative(this.rootDir, location.file).split(path.sep).join("/"), line: location.line };
  }

  private namedSteps(steps: TestStep[]): Step[] {
    return steps.flatMap(step => {
      // Named steps inside fixtures/hooks are as useful as those in the body.
      const children = this.namedSteps(step.steps);
      if (step.category !== "test.step") return children;
      const status: Status = step.annotations.some(annotation => annotation.type === "skip") ? "skipped"
        : step.error ? "failed" : this.completed.has(step) ? "passed" : "interrupted";
      return [{ title: step.title, status, duration: step.duration, children }];
    });
  }

  private attempt(result: TestResult): Attempt {
    const failureLocation = result.errors.find(error => error.location)?.location;
    return {
      number: result.retry + 1,
      status: resultStatus(result.status),
      duration: result.duration,
      steps: this.namedSteps(result.steps),
      errors: result.errors.map(message),
      failureLocation: failureLocation ? this.location(failureLocation) : undefined,
    };
  }

  private scenario(test: TestCase): Scenario {
    const groups: string[] = [];
    for (let parent: Suite | undefined = test.parent; parent; parent = parent.parent) {
      if (parent.type === "describe") groups.unshift(parent.title);
    }
    const last = test.results.at(-1);
    let status: Status;
    if (!last || (last.status === "skipped" && test.expectedStatus !== "skipped")) status = "not run";
    else if (last.status === "interrupted") status = "interrupted";
    else if (test.outcome() === "flaky") status = "flaky";
    else if (test.outcome() === "unexpected") status = last.status === "timedOut" ? "timed out" : "failed";
    else if (test.expectedStatus === "failed" && last.status === "failed") status = "expected failure";
    else status = resultStatus(last.status);
    return {
      key: JSON.stringify([test.location.file, test.location.line, test.titlePath().slice(2), test.repeatEachIndex]),
      feature: groups[0] ?? path.basename(test.location.file, ".spec.ts"),
      title: [...groups.slice(1), test.title].join(" › "),
      project: test.parent.project()?.name ?? "default",
      location: this.location(test.location),
      status,
      attempts: test.results.map(result => this.attempt(result)),
    };
  }

  onEnd(result: FullResult) {
    const scenarios = this.suite?.allTests().map(test => this.scenario(test)) ?? [];
    const { GITHUB_SERVER_URL, GITHUB_REPOSITORY, GITHUB_SHA } = process.env;
    const sourceBaseURL = GITHUB_SERVER_URL && GITHUB_REPOSITORY && GITHUB_SHA
      ? `${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/blob/${GITHUB_SHA}` : undefined;
    const summary = renderSummary({
      status: result.status,
      duration: result.duration,
      projects: [...new Set(scenarios.map(scenario => scenario.project))],
      scenarios,
      errors: this.errors,
      sourceBaseURL,
    });
    // A publishing step checks for this file because Playwright deliberately
    // does not propagate exceptions thrown by reporter callbacks.
    mkdirSync(path.dirname(this.outputFile), { recursive: true });
    writeFileSync(this.outputFile, summary);
  }
}
