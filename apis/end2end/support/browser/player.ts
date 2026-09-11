import type { BrowserContext, Page } from "@playwright/test";
import type { TestAccount } from "../accounts/catalog";

export type Player = Readonly<TestAccount & {
  page: Page;
  context: BrowserContext;
}>;
