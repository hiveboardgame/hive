import { expect, type CDPSession, type Locator, type Page } from "@playwright/test";

export type Point = Readonly<{ x: number; y: number }>;

export async function activate(locator: Locator, hasTouch: boolean) {
  if (hasTouch) await locator.tap();
  else await locator.click();
}

export async function center(locator: Locator): Promise<Point> {
  await locator.scrollIntoViewIfNeeded();
  const box = await locator.boundingBox();
  if (!box) throw new Error("The gesture target has no rendered bounds");
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

export async function mouseDrag(page: Page, source: Locator, target: Locator) {
  const start = await center(source);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  try {
    // Crossing the real pointer threshold selects the piece and renders its targets.
    await page.mouse.move(start.x + 15, start.y + 5, { steps: 3 });
    await expect(target).toBeVisible();
    const end = await center(target);
    await page.mouse.move(end.x, end.y, { steps: 12 });
  } finally {
    await page.mouse.up();
  }
}

export async function mousePan(page: Page, board: Locator) {
  await board.scrollIntoViewIfNeeded();
  const box = await board.boundingBox();
  if (!box) throw new Error("The board has no rendered bounds");
  // Start in the board's empty margin, away from selectable pieces.
  const start = { x: box.x + 24, y: box.y + 24 };
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  try {
    await page.mouse.move(start.x + 55, start.y + 30, { steps: 8 });
  } finally {
    await page.mouse.up();
  }
}

class ChromiumTouch {
  private active = false;

  constructor(private readonly session: CDPSession) {}

  async start(points: readonly Point[]) {
    await this.send("touchStart", points);
    this.active = true;
  }
  async move(points: readonly Point[]) { await this.send("touchMove", points); }
  async end() {
    if (!this.active) return;
    await this.send("touchEnd", []);
    this.active = false;
  }

  private async send(type: "touchStart" | "touchMove" | "touchEnd", points: readonly Point[]) {
    // DevTools delivers browser touch input, including the pointer/touch event sequence.
    // Calling application handlers or dispatching DOM TouchEvents would bypass that wiring.
    await this.session.send("Input.dispatchTouchEvent", {
      type,
      touchPoints: points.map((point, id) => ({ ...point, id, radiusX: 1, radiusY: 1, force: 1 })),
    });
  }
}

export async function withChromiumTouch(page: Page, use: (touch: ChromiumTouch) => Promise<void>) {
  const session = await page.context().newCDPSession(page);
  const touch = new ChromiumTouch(session);
  try {
    await use(touch);
  } finally {
    try { await touch.end(); } finally { await session.detach(); }
  }
}
