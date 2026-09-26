import { DatabaseSync } from "node:sqlite";
import { afterEach, expect, it, vi } from "vitest";
import { AccountStore, type AccountStorage } from "./sync";
import { legalPolicy } from "./legal";
import { PrivacyInbox } from "./privacy";

function storage() {
  const db = new DatabaseSync(":memory:");
  const value: AccountStorage = {
    sql: {
      exec: (query, ...values) =>
        db
          .prepare(query)
          .all(...(values as (string | number | null)[])) as Record<
          string,
          string | number | null
        >[],
    },
    transactionSync(fn) {
      db.exec("BEGIN");
      try {
        const result = fn();
        db.exec("COMMIT");
        return result;
      } catch (error) {
        db.exec("ROLLBACK");
        throw error;
      }
    },
  };
  return value;
}
const acceptance = {
  version: legalPolicy.version,
  accepted: true,
  ageConfirmed: true,
};
afterEach(() => vi.useRealTimers());
it("records explicit current acceptance durably and preserves its original server timestamp on retry", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-09-26T00:00:00.000Z"));
  const db = storage();
  const account = new AccountStore(db);
  expect(account.legal()).toEqual({ ...legalPolicy, accepted: false });
  const result = account.acceptLegal(acceptance);
  expect(result).toEqual({
    status: 200,
    body: {
      ...legalPolicy,
      accepted: true,
      acceptedAt: "2026-09-26T00:00:00.000Z",
    },
  });
  vi.setSystemTime(new Date("2026-09-27T00:00:00.000Z"));
  expect(new AccountStore(db).acceptLegal(acceptance)).toEqual(result);
  expect(new AccountStore(storage()).legal().accepted).toBe(false);
});
it.each([
  { ...acceptance, accepted: false },
  { ...acceptance, ageConfirmed: false },
  { ...acceptance, ageConfirmed: "true" },
  { ...acceptance, version: "old" },
  { ...acceptance, acceptedAt: "2020-01-01" },
  { ...acceptance, userId: "other" },
])("does not record incomplete, stale, or forged acceptance: %j", (input) => {
  const account = new AccountStore(storage());
  expect(account.acceptLegal(input).status).toBe(400);
  expect(account.legal().accepted).toBe(false);
});
it("does not mistake a previous version's receipt for current acceptance", () => {
  const db = storage();
  const account = new AccountStore(db);
  db.sql.exec(
    "INSERT INTO legal_acceptances VALUES (?,?,?,?,?)",
    "old",
    new Date().toISOString(),
    "old-terms",
    "old-privacy",
    13,
  );
  expect(account.legal().accepted).toBe(false);
});
it("isolates private request views and makes repeated pending requests idempotent", () => {
  const db = storage();
  const inbox = new PrivacyInbox(db);
  const first = inbox.submit("user_a", {
    kind: "deletion",
    message: "Delete my cloud data",
  });
  expect(first.status).toBe(201);
  expect(
    inbox.submit("user_a", { kind: "deletion", message: "Retry" }),
  ).toEqual({ ...first, status: 200 });
  expect(inbox.list("user_b")).toEqual({ requests: [] });
  const saved = new PrivacyInbox(db).list("user_a").requests;
  expect(saved).toHaveLength(1);
  expect(saved[0]).toMatchObject({
    kind: "deletion",
    message: "Delete my cloud data",
    status: "pending",
  });
  expect(saved[0]).not.toHaveProperty("userId");
  expect(inbox.pending().requests[0]).toMatchObject({
    userId: "user_a",
    id: saved[0].id,
  });
});
it("validates requests and returns an operator response only to its account", () => {
  const inbox = new PrivacyInbox(storage());
  expect(
    inbox.submit("user_a", { kind: "other", message: "a".repeat(2001) }).status,
  ).toBe(400);
  inbox.submit("user_a", { kind: "access" });
  const id = inbox.list("user_a").requests[0].id;
  expect(
    inbox.resolve({
      id,
      status: "resolved",
      response: "Your data is available using Export cloud data.",
    }).status,
  ).toBe(200);
  expect(inbox.pending().requests).toEqual([]);
  expect(inbox.list("user_a").requests[0]).toMatchObject({
    status: "resolved",
    response: "Your data is available using Export cloud data.",
  });
  expect(inbox.list("user_b").requests).toEqual([]);
  expect(
    inbox.resolve({ id, status: "resolved", response: "Overwrite" }).body,
  ).toEqual({ request: inbox.list("user_a").requests[0] });
  expect(
    inbox.resolve({
      id: crypto.randomUUID(),
      status: "resolved",
      response: "Missing",
    }).status,
  ).toBe(404);
});
it("bounds and paginates the operator inbox without exposing other accounts to users", () => {
  const inbox = new PrivacyInbox(storage());
  for (let i = 0; i < 105; i++) inbox.submit(`user_${i}`, { kind: "other" });
  const first = inbox.pending();
  expect(first.requests).toHaveLength(100);
  expect(first.nextCursor).not.toBeNull();
  const next = inbox.pending(first.nextCursor!);
  expect(next.requests).toHaveLength(5);
  expect(next.nextCursor).toBeNull();
  expect(
    new Set([...first.requests, ...next.requests].map((request) => request.id))
      .size,
  ).toBe(105);
  expect(inbox.list("user_0").requests).toHaveLength(1);
});
it("expires resolved messages after 90 days while preserving pending requests", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-09-26T00:00:00Z"));
  const inbox = new PrivacyInbox(storage());
  inbox.submit("user_a", { kind: "access" });
  inbox.submit("user_a", { kind: "deletion" });
  inbox.resolve({
    id: inbox.list("user_a").requests.find((r) => r.kind === "access")!.id,
    status: "resolved",
    response: "Completed",
  });
  vi.setSystemTime(new Date("2026-12-26T00:00:00Z"));
  expect(inbox.list("user_a").requests.map((r) => r.kind)).toEqual([
    "deletion",
  ]);
});

it("keeps only 20 completed requests and at most one pending request per kind", () => {
  vi.useFakeTimers();
  const inbox = new PrivacyInbox(storage());
  for (let i = 0; i < 25; i++) {
    vi.setSystemTime(new Date(Date.UTC(2026, 8, 26, 0, i)));
    inbox.submit("user_a", { kind: "other" });
    const item = inbox.pending().requests[0];
    inbox.resolve({ id: item.id, status: "resolved", response: "Completed" });
  }
  expect(inbox.list("user_a").requests).toHaveLength(20);
  for (const kind of ["access", "correction", "deletion", "other"] as const) {
    inbox.submit("user_a", { kind });
    inbox.submit("user_a", { kind });
  }
  expect(inbox.list("user_a").requests).toHaveLength(24);
  expect(inbox.pending().requests).toHaveLength(4);
  expect(inbox.nextCleanup()).toBe(Date.UTC(2026, 11, 25, 0, 5));
});

it("fails visibly at the global pending limit while preserving retries and existing requests", () => {
  const db = storage();
  const inbox = new PrivacyInbox(db);
  inbox.submit("existing_user", { kind: "access" });
  db.sql.exec(
    "WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<9999) INSERT INTO privacy_requests SELECT 'capacity-'||i, 'capacity-user-'||i, 'other', '', 'pending', '2026-09-26T00:00:00.000Z', '2026-09-26T00:00:00.000Z', NULL FROM n",
  );
  expect(inbox.submit("new_user", { kind: "access" }).status).toBe(503);
  expect(inbox.list("new_user").requests).toEqual([]);
  expect(inbox.submit("existing_user", { kind: "access" }).status).toBe(200);
});
