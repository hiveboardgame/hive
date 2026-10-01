import type { Page, TestInfo } from "@playwright/test";

const diagnosticsByPage = new WeakMap<Page, string[]>();

export function logBrowserDiagnostics(page: Page, username: string) {
  const existingDiagnostics = diagnosticsByPage.get(page);
  if (existingDiagnostics) return existingDiagnostics;
  const diagnostics: string[] = [];
  diagnosticsByPage.set(page, diagnostics);

  const startedAt = Date.now();
  const log = (message: string) => {
    if (diagnostics.length >= 500) diagnostics.shift();
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
  page.on("websocket", socket => {
    log(`WebSocket created: ${path(socket.url())}`);
    socket.on("close", () => log("WebSocket closed"));
    socket.on("socketerror", error => log(`WebSocket error: ${error}`));
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

// Capture while the failing pages still exist, before recovery changes their state.
export async function attachFailureDiagnostics(
  pages: readonly { page: Page; username: string }[], testInfo: TestInfo,
) {
  for (const { page, username } of pages) {
    try {
      await testInfo.attach(`${username} browser diagnostics before cleanup`, {
        body: diagnosticsByPage.get(page)?.join("\n") ?? "No browser events recorded",
        contentType: "text/plain",
      });
      if (!page.isClosed()) {
        await testInfo.attach(`${username} page before cleanup`, {
          body: await page.locator("body").ariaSnapshot({ timeout: 2_000 }),
          contentType: "text/plain",
        });
      }
    } catch (error) {
      // Diagnostics must not prevent fixture cleanup or replace the test failure.
      console.warn(`Could not attach diagnostics for ${username}: ${String(error)}`);
    }
  }
}
