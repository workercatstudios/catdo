import { DatabaseSync } from "node:sqlite";
import { beforeEach, expect, it, vi } from "vitest";
import { emptyData, initialData } from "../../../../packages/domain/src/model";
const auth = vi.hoisted(() => ({ userId: "user_a", verify: vi.fn() }));
vi.mock("@clerk/backend", () => ({
  createClerkClient: () => ({ authenticateRequest: auth.verify }),
}));
import worker, { type Env } from "./index";
import { AccountStore, type AccountStorage } from "./sync";
import { PrivacyInbox, privacyInboxName } from "./privacy";
import { legalPolicy } from "./legal";

function account() {
  const db = new DatabaseSync(":memory:");
  const storage: AccountStorage = {
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
  const store = new AccountStore(storage),
    inbox = new PrivacyInbox(storage);
  return {
    snapshot: () => store.snapshot(),
    push: (value: Parameters<AccountStore["push"]>[0]) => store.push(value),
    legal: () => store.legal(),
    acceptLegal: (value: unknown) => store.acceptLegal(value),
    exportCloudData: () => store.exportCloudData(),
    eraseCloudData: () => store.eraseCloudData(),
    privacyRequests: (id: string) => inbox.list(id),
    submitPrivacyRequest: (
      id: string,
      input: Parameters<PrivacyInbox["submit"]>[1],
    ) => inbox.submit(id, input),
    pendingPrivacyRequests: (cursor?: Parameters<PrivacyInbox["pending"]>[0]) =>
      inbox.pending(cursor),
    resolvePrivacyRequest: (input: Parameters<PrivacyInbox["resolve"]>[0]) =>
      inbox.resolve(input),
    privacyRequestForOperator: (id: string) => inbox.requestForOperator(id),
  };
}
const accounts = new Map<string, ReturnType<typeof account>>();
const getByName = vi.fn((name: string) => {
  if (!accounts.has(name)) accounts.set(name, account());
  return accounts.get(name)!;
});
const adminToken = "private-operator-test-token-at-least-thirty-two-characters";
const env = {
  ACCOUNTS: { getByName },
  APP_ORIGIN: "https://catdo.workercat.com",
  CLERK_SECRET_KEY: "fixture",
  CLERK_PUBLISHABLE_KEY: "fixture",
  PRIVACY_ADMIN_TOKEN: adminToken,
} as unknown as Env;
function call(
  path: string,
  body?: unknown,
  token = "fixture",
  extra: Record<string, string> = {},
) {
  return worker.fetch(
    new Request(`https://catdo.workercat.com/api/${path}`, {
      method: body === undefined ? "GET" : "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
        ...extra,
      },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    }),
    env,
  );
}
const accepted = {
  version: legalPolicy.version,
  accepted: true,
  ageConfirmed: true,
};
const pending = () => ({
  id: crypto.randomUUID(),
  base: emptyData(),
  data: initialData(),
});
beforeEach(() => {
  accounts.clear();
  vi.clearAllMocks();
  auth.userId = "user_a";
  auth.verify.mockImplementation(async (_req, options) => ({
    toAuth: () =>
      options.acceptsToken === "session_token"
        ? {
            isAuthenticated: true,
            userId: auth.userId,
            tokenType: "session_token",
          }
        : null,
  }));
});
it("requires explicit acceptance before cloud writes while preserving authenticated export and identity access", async () => {
  expect(await (await call("legal")).json()).toEqual({
    ...legalPolicy,
    accepted: false,
  });
  const denied = await call("sync", pending());
  expect(denied.status).toBe(428);
  expect(await denied.json()).toMatchObject({
    code: "terms_required",
    reviewUrl: "https://catdo.workercat.com/app",
    ...legalPolicy,
  });
  expect((await call("sync")).status).toBe(200);
  expect(await (await call("me")).json()).toEqual({ userId: "user_a" });
  expect(
    (await call("legal", { ...accepted, ageConfirmed: false })).status,
  ).toBe(400);
  expect((await call("legal", { ...accepted, version: "old" })).status).toBe(
    400,
  );
  const response = await call("legal", accepted);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchObject({
    ...legalPolicy,
    accepted: true,
  });
  expect((await call("sync", pending())).status).toBe(200);
  auth.userId = "user_b";
  expect((await call("sync", pending())).status).toBe(428);
});
it("records requests without terms acceptance, derives identity from auth, and rejects forged fields", async () => {
  expect(
    (await call("privacy-requests", { kind: "deletion", userId: "user_b" }))
      .status,
  ).toBe(400);
  const response = await call("privacy-requests", {
    kind: "deletion",
    message: "Delete cloud tasks",
  });
  expect(response.status).toBe(201);
  const submitted = await response.json();
  expect(
    await (await call("privacy-requests", { kind: "deletion" })).json(),
  ).toEqual(submitted);
  expect(getByName).toHaveBeenCalledWith(privacyInboxName);
  auth.userId = "user_b";
  expect(await (await call("privacy-requests")).json()).toEqual({
    requests: [],
  });
});
it("rejects user sessions, browser origins, missing secrets, and malformed cursors from the operator endpoint", async () => {
  expect((await call("admin/privacy-requests")).status).toBe(401);
  expect(
    (
      await call("admin/privacy-requests", undefined, adminToken, {
        Origin: env.APP_ORIGIN,
      })
    ).status,
  ).toBe(401);
  expect(
    (
      await worker.fetch(
        new Request("https://catdo.workercat.com/api/admin/privacy-requests", {
          headers: { Authorization: `Bearer ${adminToken}` },
        }),
        { ...env, PRIVACY_ADMIN_TOKEN: undefined },
      )
    ).status,
  ).toBe(401);
  expect(
    (await call("admin/privacy-requests?id=bad", undefined, adminToken)).status,
  ).toBe(400);
  expect(auth.verify).not.toHaveBeenCalled();
});
it("privately resolves a request and exposes the response only to the requesting account", async () => {
  await call("privacy-requests", {
    kind: "correction",
    message: "Please help",
  });
  const { requests } = (await (
    await call("admin/privacy-requests", undefined, adminToken)
  ).json()) as { requests: { id: string; userId: string }[] };
  expect(requests[0].userId).toBe("user_a");
  expect(
    (
      await call(
        "admin/privacy-requests",
        {
          id: requests[0].id,
          status: "resolved",
          response: "Your correction is complete.",
        },
        adminToken,
      )
    ).status,
  ).toBe(200);
  const own = (await (await call("privacy-requests")).json()) as {
    requests: { response: string }[];
  };
  expect(own.requests[0].response).toBe("Your correction is complete.");
  auth.userId = "user_b";
  expect(await (await call("privacy-requests")).json()).toEqual({
    requests: [],
  });
});
it("allows export only for a pending access request and erasure only for a matching deletion request", async () => {
  await call("legal", accepted);
  const data = pending();
  await call("sync", data);
  const { request } = (await (
    await call("privacy-requests", { kind: "access" })
  ).json()) as { request: { id: string } };
  expect(
    (
      await call(
        "admin/privacy-requests",
        {
          id: request.id,
          action: "erase-cloud-data",
          confirm: "erase-cloud-data",
        },
        adminToken,
      )
    ).status,
  ).toBe(409);
  const exported = await call(
    "admin/privacy-requests",
    { id: request.id, action: "export" },
    adminToken,
  );
  expect(exported.status).toBe(200);
  expect(await exported.json()).toMatchObject({
    userId: "user_a",
    cloud: { revision: 1, data: data.data },
    acceptances: [{ version: legalPolicy.version }],
  });
  const deleted = (await (
    await call("privacy-requests", { kind: "deletion" })
  ).json()) as { request: { id: string } };
  expect(
    (
      await call(
        "admin/privacy-requests",
        { id: deleted.request.id, action: "erase-cloud-data" },
        adminToken,
      )
    ).status,
  ).toBe(400);
  const erased = await call(
    "admin/privacy-requests",
    {
      id: deleted.request.id,
      action: "erase-cloud-data",
      confirm: "erase-cloud-data",
    },
    adminToken,
  );
  expect(await erased.json()).toEqual({ erased: true, cloudDisabled: true });
  expect(await (await call("sync")).json()).toEqual({
    revision: 0,
    data: emptyData(),
  });
  expect(await (await call("legal")).json()).toMatchObject({ accepted: true });
  const retry = await call("sync", data);
  expect(retry.status).toBe(403);
  expect(await retry.json()).toMatchObject({ code: "cloud_data_deleted" });
  await call("legal", accepted);
  expect((await call("sync", pending())).status).toBe(403);
});
