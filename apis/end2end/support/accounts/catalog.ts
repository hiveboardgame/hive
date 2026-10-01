// Odd-numbered users are primary accounts; even-numbered users are partners.
// The first five primary accounts own distinct Quick Play queues.
export const publicTimeControls = ["1+2", "3+3", "5+4", "10+10", "20+20"] as const;
export const poolCapacity = 15;
export const usernames = Array.from({ length: poolCapacity * 2 }, (_, i) => `user_${i + 1}`);

export type TestAccount = Readonly<{ id: string; username: string }>;
export function publicTimeControl(account: TestAccount): string {
  const index = usernames.indexOf(account.username);
  if (index < 0 || index % 2 !== 0) throw new Error("Public challenges require a primary test account.");
  const control = publicTimeControls[index / 2];
  if (!control) throw new Error("Public challenges require a queue-owning account; use publicChallenge: true.");
  return control;
}
