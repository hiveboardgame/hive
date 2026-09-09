export const usernames = Array.from({ length: 8 }, (_, i) => `user_${i + 1}`);
// Odd-numbered users are primary accounts; even-numbered users are partners.
// Each primary account owns a distinct Quick Play queue, including single users.
export const publicTimeControls = ["1+2", "3+3", "5+4", "10+10"] as const;

export type TestAccount = Readonly<{ id: string; username: string }>;
export function publicTimeControl(account: TestAccount): string {
  const index = usernames.indexOf(account.username);
  if (index < 0 || index % 2 !== 0) throw new Error("Public challenges require a primary test account.");
  return publicTimeControls[index / 2];
}
