import { test, expect, Browser, BrowserContext, Page } from "@playwright/test";

const SELECTORS = {
  gameBoard: 'svg.touch-none',
  resignButton: 'button[title="Resign"]',
  timer: 'button.bg-grasshopper-green[class*="cursor-"]',
  target: 'use[href="/assets/tiles/common/all.svg#target"]',
  loginButton: 'text="Login"',
  mobile: {
    reserve: 'svg:has(use[href*="white"], use[href*="black"])',
    toggleControls: 'button[title*="toggle"], button[title*="Toggle"], button[title*="show"], button[title*="Show"]'
  },
  desktop: {
    reserve: 'svg.duration-300.p-1'
  },
};

const log = (msg: string, data?: any) => {
  if (process.env.DEBUG) {
    console.log(msg, data || '');
  }
};

async function createAuthenticatedUser(browser: Browser, userSuffix: string): Promise<{
  context: BrowserContext;
  page: Page;
  username: string;
  email: string;
}> {
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
  });

  const page = await context.newPage();


  const timestamp = Date.now();
  const random = Math.floor(Math.random() * 10000);
  const username = `multi${userSuffix}${timestamp}${random}`;
  const email = `multi${userSuffix}${timestamp}${random}@example.com`;
  const password = "testpass123";

  await page.goto("/register");
  await page.waitForLoadState("networkidle");

  await page.locator('input[name="username"]').fill(username);

  await page.locator('input[name="email"]').fill(email);
  await page.locator('input[name="password"]').fill(password);
  await page.locator('input[name="password_confirmation"]').fill(password);
  await page.locator('input[type="checkbox"]').check();

  const submitButton = page.locator('input[type="submit"]');
  await expect(submitButton).toBeEnabled({ timeout: 1500 });
  await submitButton.click();
  await page.waitForURL(/^(?!.*\/register).*$/);

  await page.goto("/");
  await page.waitForLoadState("networkidle");

  return { context, page, username, email };
}

async function isMobileViewport(page: Page): Promise<boolean> {
  const viewport = page.viewportSize();
  return viewport ? viewport.width < 768 : false;
}

async function handleResignation(player: any, isMobile: boolean) {
  if (isMobile) {
    const toggleBtn = player.page.locator(SELECTORS.mobile.toggleControls);
    await toggleBtn.click();
  }

  const resignBtn = player.page.locator(SELECTORS.resignButton);
  await expect(resignBtn).toBeVisible({ timeout: 5000 });
  await resignBtn.click();
  await resignBtn.click();
}

async function identifyPlayerColors(userA: any, userB: any) {
  const getWhitePieceCount = async (user: any) => {
    const isMobile = await isMobileViewport(user.page);
    const selector = isMobile ? SELECTORS.mobile.reserve : SELECTORS.desktop.reserve;
    const reserve = user.page.locator(selector).last();
    return await reserve.locator('use[href*="white"]').count();
  };

  const [countA, countB] = await Promise.all([
    getWhitePieceCount(userA),
    getWhitePieceCount(userB)
  ]);

  log('White piece counts', { userA: countA, userB: countB });

  if (countA > countB) {
    return { whitePlayer: userA, blackPlayer: userB };
  } else {
    return { whitePlayer: userB, blackPlayer: userA };
  }
}

async function makeMove(
  player: Awaited<ReturnType<typeof createAuthenticatedUser>>,
  pieceType: string = "Ant",
  expectedTargetCount: number = 1
) {
  const isMobile = await isMobileViewport(player.page);

  if (isMobile) {
    const reserves = player.page.locator(SELECTORS.mobile.reserve);
    const bottomReserve = reserves.last();
    const playerPieces = bottomReserve.locator(`g:has(use[href*="${pieceType}"])`);

    await playerPieces.first().click({ force: true });
  } else {
    const bottomReserve = player.page.locator(SELECTORS.desktop.reserve).last();
    const playerPieces = bottomReserve.locator(`g:has(use[href*="${pieceType}"], use[href*="Grasshopper"], use[href*="Beetle"], use[href*="Spider"])`);

    await playerPieces.first().click({ force: true });
  }

  const blueTargets = player.page.locator(SELECTORS.target);
  await expect(blueTargets).toHaveCount(expectedTargetCount, { timeout: 3000 });
  await blueTargets.first().click({ force: true });

  const previewPiece = player.page.locator(`g:has(use[href*="${pieceType}"]) > use[href*="${pieceType}"]`).last();
  await expect(previewPiece).toBeVisible({ timeout: 3000 });
  await previewPiece.click({ force: true });
}

test.describe("Multi-User Real-Time Tests", () => {
  test("multi-user quickplay auto-matching", async ({ browser }) => {
    let userA!: Awaited<ReturnType<typeof createAuthenticatedUser>>;
    let userB!: Awaited<ReturnType<typeof createAuthenticatedUser>>;

    try {
      await test.step("Create and authenticate two users", async () => {
        [userA, userB] = await Promise.all([
          createAuthenticatedUser(browser, "A"),
          createAuthenticatedUser(browser, "B")
        ]);

        await Promise.all([
          expect(userA.page.locator(SELECTORS.loginButton)).not.toBeVisible({ timeout: 10000 }),
          expect(userB.page.locator(SELECTORS.loginButton)).not.toBeVisible({ timeout: 10000 })
        ]);
      });

      await test.step("Start quickplay and auto-match players", async () => {
        await userA.page.getByRole("button", { name: "1+2" }).click();
        await new Promise(resolve => setTimeout(resolve, 100));
        await userB.page.getByRole("button", { name: "1+2" }).click();

        await Promise.all([
          userA.page.waitForURL(/\/game\//, { timeout: 30000 }),
          userB.page.waitForURL(/\/game\//, { timeout: 30000 })
        ]);

        const userAUrl = userA.page.url();
        const userBUrl = userB.page.url();
        const gameIdA = userAUrl.split('/game/')[1];
        const gameIdB = userBUrl.split('/game/')[1];

        expect(gameIdA).toBe(gameIdB);
        log('Game matched', { gameId: gameIdA });

      });

      await test.step("Verify game board is visible for both players", async () => {
        await Promise.all([
          userA.page.waitForLoadState("networkidle"),
          userB.page.waitForLoadState("networkidle")
        ]);

        await Promise.all([
          expect(userA.page.locator(SELECTORS.gameBoard)).toBeVisible({ timeout: 10000 }),
          expect(userB.page.locator(SELECTORS.gameBoard)).toBeVisible({ timeout: 10000 })
        ]);
      });

      let whitePlayer: Awaited<ReturnType<typeof createAuthenticatedUser>>;
      let blackPlayer: Awaited<ReturnType<typeof createAuthenticatedUser>>;

      await test.step("Identify white and black players", async () => {
        const playerColors = await identifyPlayerColors(userA, userB);
        whitePlayer = playerColors.whitePlayer;
        blackPlayer = playerColors.blackPlayer;
      });

      await test.step("White player makes first move", async () => {
        await makeMove(whitePlayer, "Ant", 1);
      });

      await test.step("Black player makes second move", async () => {
        await new Promise(resolve => setTimeout(resolve, 200));
        await makeMove(blackPlayer, "Ant", 6);
        await new Promise(resolve => setTimeout(resolve, 200));
      });

      await test.step("White player resigns from game", async () => {
        const isMobile = await isMobileViewport(whitePlayer.page);
        await handleResignation(whitePlayer, isMobile);

        await Promise.all([
          expect(blackPlayer.page.locator(SELECTORS.timer)).not.toBeVisible({ timeout: 10000 }),
          expect(whitePlayer.page.locator(SELECTORS.resignButton)).not.toBeVisible({ timeout: 10000 })
        ]);
      });

    } finally {
      if (userA?.context) {
        await userA.context.close();
      }
      if (userB?.context) {
        await userB.context.close();
      }
    }
  });
});
