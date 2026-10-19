import type { BrowserContext, Page } from "@playwright/test";
import type { Account } from "../data";

export type Player = Account & Readonly<{ context: BrowserContext; page: Page }>;
export type Players = Readonly<{ userOne: Player; userTwo: Player }>;
