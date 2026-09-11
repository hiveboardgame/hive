import { test, expect, type Browser, type TestInfo } from "@playwright/test";
import type { AccountReservation } from "../support/accounts/reservation";
import { withUsers } from "../support/browser/session";

function setup(count: 1 | 2 = 2) {
  const events: string[] = [];
  let created = 0;
  let loss: Error | undefined;
  let onLost: ((error: Error) => void) | undefined;
  const browser = {
    newContext: async () => {
      const id = ++created;
      return {
        newPage: async () => ({ id }),
        close: async () => { events.push(`close ${id}`); },
      };
    },
  } as unknown as Browser;
  const reservation = {
    accounts: [{ id: "seven", username: "user_7" }, { id: "eight", username: "user_8" }].slice(0, count),
    release: async () => { events.push("release"); },
    assertHeld: () => { if (loss) throw loss; },
    onLost: (listener: (error: Error) => void) => {
      onLost = listener;
      return () => { onLost = undefined; };
    },
  } as unknown as AccountReservation;
  const testInfo = {
    status: "passed",
    attach: async (name: string) => { events.push(name); },
  } as Pick<TestInfo, "status" | "attach">;
  const options: Parameters<typeof withUsers>[0] = {
    browser, count, baseURL: "http://localhost:3000", testInfo, isMobileLayout: false,
    reserve: async requested => {
      expect(requested).toBe(count);
      events.push(`reserve ${requested}`);
      return reservation;
    },
    authenticate: async (_page, username, baseURL) => {
      expect(baseURL).toBe(options.baseURL);
      events.push(`login ${username}`);
    },
    cleanup: async () => { events.push("cleanup"); },
  };
  return {
    events, options, reservation,
    lose: () => { loss = new Error("reservation lost"); onLost!(loss); },
  };
}

for (const count of [1, 2] as const) {
  test(`${count} user(s) are reserved, authenticated, recovered, used, cleaned, closed and released`, async () => {
    const { options, events } = setup(count);
    await withUsers(options, async users => {
      expect(users).toHaveLength(count);
      expect(users[0]).toMatchObject({ id: "seven", username: "user_7" });
      expect(new Set(users.map(user => user.page)).size).toBe(count);
      expect(new Set(users.map(user => user.context)).size).toBe(count);
      events.push("test");
    });
    expect(events).toEqual([
      `reserve ${count}`, "login user_7", ...(count === 2 ? ["login user_8"] : []),
      "cleanup", "test", "cleanup", "close 1", ...(count === 2 ? ["close 2"] : []), "release",
    ]);
  });
}

test("a second-user login failure closes both contexts and releases without running the test", async () => {
  const { options, events } = setup();
  options.authenticate = async (_page, username) => {
    if (username === "user_8") throw new Error("login failed");
  };
  await expect(withUsers(options, async () => { events.push("test"); })).rejects.toThrow("login failed");
  expect(events).toEqual(["reserve 2", "close 1", "close 2", "release"]);
});

test("partial context creation failure closes the first context before releasing", async () => {
  const { options, events } = setup();
  const create = options.browser.newContext;
  let attempts = 0;
  options.browser.newContext = async () => {
    if (++attempts === 2) throw new Error("context failed");
    return create();
  };
  await expect(withUsers(options, async () => {})).rejects.toThrow("context failed");
  expect(events).toEqual(["reserve 2", "login user_7", "close 1", "release"]);
});

test("recovery failure prevents the test and retains the setup error when teardown also fails", async () => {
  const { options, events } = setup();
  let attempts = 0;
  options.cleanup = async () => { throw new Error(++attempts === 1 ? "recovery failed" : "cleanup failed"); };
  await expect(withUsers(options, async () => { events.push("test"); })).rejects.toThrow("recovery failed");
  expect(events).not.toContain("test");
  expect(events.slice(-4)).toEqual(["close 1", "close 2", "release", "Account cleanup failure"]);
});

test("teardown errors do not replace an existing test failure", async () => {
  const { options, events } = setup();
  let attempts = 0;
  options.cleanup = async () => { if (++attempts === 2) throw new Error("cleanup failed"); };
  await withUsers(options, async () => { options.testInfo.status = "failed"; });
  expect(events.slice(-4)).toEqual(["close 1", "close 2", "release", "Account cleanup failure"]);
});

test("cleanup failure after a passing test fails the fixture but closes and releases", async () => {
  const { options, events } = setup();
  let attempts = 0;
  options.cleanup = async () => { if (++attempts === 2) throw new Error("cleanup failed"); };
  await expect(withUsers(options, async () => {})).rejects.toThrow("cleanup failed");
  expect(events.slice(-3)).toEqual(["close 1", "close 2", "release"]);
});

test("lost reservations immediately close contexts once and prevent subsequent cleanup", async () => {
  const { options, events, lose } = setup();
  await expect(withUsers(options, async () => {
    lose();
    expect(events.slice(-2)).toEqual(["close 1", "close 2"]);
  })).rejects.toThrow("reservation lost");
  expect(events.filter(event => event === "cleanup")).toHaveLength(1);
  expect(events.filter(event => event.startsWith("close"))).toHaveLength(2);
  expect(events.at(-1)).toBe("release");
});

test("reservation loss during context creation still closes the newly returned context", async () => {
  const { options, events, lose } = setup(1);
  const create = options.browser.newContext;
  options.browser.newContext = async () => {
    const context = await create();
    lose();
    return context;
  };
  await expect(withUsers(options, async () => {})).rejects.toThrow("reservation lost");
  expect(events).toEqual(["reserve 1", "close 1", "release"]);
});

test("context close errors still release the reservation", async () => {
  const { options, events } = setup(1);
  const create = options.browser.newContext;
  options.browser.newContext = async () => {
    const context = await create();
    context.close = async () => { throw new Error("close failed"); };
    return context;
  };
  await expect(withUsers(options, async () => {})).rejects.toThrow("close failed");
  expect(events.at(-1)).toBe("release");
});

test("release failure is reported without replacing a primary error", async () => {
  const { options, events, reservation } = setup(1);
  reservation.release = async () => { throw new Error("release failed"); };
  await expect(withUsers(options, async () => { throw new Error("test failed"); })).rejects.toThrow("test failed");
  expect(events.at(-1)).toBe("Account cleanup failure");
});
