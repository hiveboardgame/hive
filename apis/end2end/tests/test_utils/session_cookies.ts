import type { BrowserContext } from "playwright/test";

const instrumentedContexts = new WeakMap<BrowserContext, Promise<void>>();

function cookieKey(name: string, domain: string, path: string) {
  return JSON.stringify([name, domain.replace(/^\./, "").toLowerCase(), path]);
}

function responseCookieKey(header: string, url: URL) {
  const [pair, ...attributes] = header.split(";").map(part => part.trim());
  const name = pair.slice(0, pair.indexOf("="));
  let domain = url.hostname;
  const defaultPath = url.pathname.slice(0, url.pathname.lastIndexOf("/")) || "/";
  let path = defaultPath;
  for (const attribute of attributes) {
    const separator = attribute.indexOf("=");
    if (separator < 0) continue;
    const key = attribute.slice(0, separator).trim().toLowerCase();
    const value = attribute.slice(separator + 1).trim();
    if (key === "domain" && value) domain = value;
    if (key === "path") path = value.startsWith("/") ? value : defaultPath;
  }
  return cookieKey(name, domain, path);
}

export async function stripSecureCookiesForWebKit(
  context: BrowserContext,
  applicationURL: string,
) {
  if (context.browser()?.browserType().name() !== "webkit") return;
  const application = new URL(applicationURL);
  if (application.protocol !== "http:") return;

  const existing = instrumentedContexts.get(context);
  if (existing) return existing;

  const origin = application.origin;
  const installation = context.route(url => (
    url.origin === origin && url.pathname.startsWith("/api/")
  ), async route => {
    // A page load or normal form submission has resource type "document", even
    // when its URL starts with /api/. Give those requests to the next matching
    // handler (or the browser's network stack) without fetching or rewriting
    // their responses. The browser then follows HTTP redirects itself.
    if (!["fetch", "xhr"].includes(route.request().resourceType())) {
      await route.fallback();
      return;
    }
    // Only background API fetch/XHR requests reach this point. Hydrated Leptos
    // login returns 200 with a client-redirect header, which we preserve below.
    // WebKit cannot replay a real HTTP redirect through route.fulfill; document
    // requests therefore take the fallback path above.
    const response = await route.fetch({ maxRedirects: 0 });
    const headers = response.headers();
    const rewrittenKeys = new Set<string>();
    const cookies = response.headersArray()
      .filter(header => header.name.toLowerCase() === "set-cookie")
      .map(header => {
        const value = header.value.replace(/;[\t ]*secure[\t ]*(?=;|$)/gi, "");
        if (value !== header.value) {
          rewrittenKeys.add(responseCookieKey(header.value, new URL(response.url())));
        }
        return value;
      });
    if (cookies.length) {
      // Playwright splits newline-separated Set-Cookie values into headers.
      // Never split on commas: Expires attributes contain them.
      headers["set-cookie"] = cookies.join("\n");
    }
    // route.fetch also populates the context's cookie jar. WebKit does not
    // replace those Secure cookies from a fulfilled HTTP response, so update
    // only the exact cookies whose response attributes we stripped.
    if (rewrittenKeys.size) {
      const storedCookies = (await context.cookies())
        .filter(cookie => cookie.secure && rewrittenKeys.has(
          cookieKey(cookie.name, cookie.domain, cookie.path),
        ))
        .map(cookie => ({ ...cookie, secure: false }));
      if (storedCookies.length) await context.addCookies(storedCookies);
    }
    await route.fulfill({ response, headers });
  });
  instrumentedContexts.set(context, installation);
  try {
    await installation;
  } catch (error) {
    instrumentedContexts.delete(context);
    throw error;
  }
}
