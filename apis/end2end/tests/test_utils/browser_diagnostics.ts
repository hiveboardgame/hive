import type { Page } from "playwright/test";

const diagnosticsByPage = new WeakMap<Page, string[]>();

export function logBrowserDiagnostics(page: Page, username: string) {
  const existingDiagnostics = diagnosticsByPage.get(page);
  if (existingDiagnostics) return existingDiagnostics;
  const diagnostics: string[] = [];
  diagnosticsByPage.set(page, diagnostics);

  const startedAt = Date.now();
  const log = (message: string) => {
    diagnostics.push(`[browser ${username} +${Date.now() - startedAt}ms] ${message}`);
  };
  // Keep query strings, request bodies, and cookies out of network logs.
  const path = (url: string) => new URL(url).pathname;
  const isAsset = (url: string) => /\.(?:m?js|wasm)$/.test(path(url));

  page.on("pageerror", error => log(`Uncaught error: ${error.stack ?? error.message}`));
  page.on("console", message => {
    if (message.type() === "error" || message.type() === "warning") {
      log(`Console ${message.type()}: ${message.text()}`);
    }
  });
  page.on("crash", () => log("Page crashed"));
  page.on("request", request => {
    if (isAsset(request.url())) log(`Asset requested: ${path(request.url())}`);
  });
  page.on("requestfinished", request => {
    if (isAsset(request.url())) log(`Asset downloaded: ${path(request.url())}`);
  });
  page.on("requestfailed", request => {
    log(`Request failed: ${request.method()} ${path(request.url())}: ${request.failure()?.errorText}`);
  });
  page.on("response", response => {
    if (response.status() >= 400) {
      log(`HTTP ${response.status()}: ${path(response.url())}`);
    }
  });
  return diagnostics;
}
