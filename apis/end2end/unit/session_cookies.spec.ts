import { expect, test, type BrowserContext, type Route } from "@playwright/test";
import { stripSecureCookiesForWebKit } from "../support/browser/session_cookies";

function contextFor(browserName: string) {
  const routes: Array<(url: URL) => boolean> = [];
  const handlers: Array<(route: Route) => Promise<void>> = [];
  const context = {
    browser: () => ({ browserType: () => ({ name: () => browserName }) }),
    route: async (matches: (url: URL) => boolean, handler: (route: Route) => Promise<void>) => {
      routes.push(matches);
      handlers.push(handler);
    },
  } as unknown as BrowserContext;
  return { context, routes, handlers };
}

test("API document navigations fall through without fetching or rewriting a response", async () => {
  const { context, handlers } = contextFor("webkit");
  await stripSecureCookiesForWebKit(context, "http://localhost:3000/login");
  const calls: string[] = [];
  const route = {
    request: () => ({ resourceType: () => "document" }),
    fallback: async () => { calls.push("fallback"); },
    fetch: async () => { throw new Error("Document response must stay with the browser"); },
    fulfill: async () => { throw new Error("Document response must not be rewritten"); },
  } as unknown as Route;
  await handlers[0](route);
  expect(calls).toEqual(["fallback"]);
});

test("HTTPS WebKit sessions retain normal cookie handling", async () => {
  const { context, routes } = contextFor("webkit");
  await stripSecureCookiesForWebKit(context, "https://localhost/login");
  expect(routes).toHaveLength(0);
});

test("HTTP WebKit interception is installed once and limited to application APIs", async () => {
  const { context, routes } = contextFor("webkit");
  await stripSecureCookiesForWebKit(context, "http://localhost:3000/login");
  await stripSecureCookiesForWebKit(context, "http://localhost:3000/login");
  expect(routes).toHaveLength(1);
  expect(routes[0](new URL("http://localhost:3000/api/login"))).toBe(true);
  expect(routes[0](new URL("http://localhost:3000/login"))).toBe(false);
  expect(routes[0](new URL("http://example.test/api/login"))).toBe(false);
  expect(routes[0](new URL("https://localhost:3000/api/login"))).toBe(false);
});

for (const browser of ["chromium", "firefox"]) {
  test(`${browser} sessions retain normal cookie handling`, async () => {
    const { context, routes } = contextFor(browser);
    await stripSecureCookiesForWebKit(context, "http://localhost/login");
    expect(routes).toHaveLength(0);
  });
}
