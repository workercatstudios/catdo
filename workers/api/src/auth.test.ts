import { beforeEach, it, expect, vi } from "vitest";
const auth = vi.hoisted(() => ({
  value: null as null | Record<string, unknown>,
  verify: vi.fn(),
}));
vi.mock("@clerk/backend", () => ({
  createClerkClient: () => ({ authenticateRequest: auth.verify }),
}));
vi.mock("./account", () => ({ CatDoAccount: class {} }));
import worker, { type Env } from "./index";
const account = {
  snapshot: vi.fn(async () => ({
    revision: 0,
    data: { workspaces: [], projects: [], tasks: [], history: [] },
  })),
};
const getByName = vi.fn(() => account);
const env = {
  ACCOUNTS: { getByName },
  APP_ORIGIN: "https://catdo.workercat.com",
  CLERK_SECRET_KEY: "test-fixture",
  CLERK_PUBLISHABLE_KEY: "test-fixture",
  CLERK_DESKTOP_CLIENT_ID: "catdo-desktop",
  CLERK_ISSUER: "https://clerk.workercat.com",
} as unknown as Env;
beforeEach(() => {
  vi.clearAllMocks();
  auth.value = {
    isAuthenticated: true,
    tokenType: "session_token",
    userId: "user_a",
  };
  auth.verify.mockImplementation(async (_request, options) => ({
    toAuth: () =>
      options.acceptsToken === auth.value?.tokenType ? auth.value : null,
  }));
});
const request = (headers: Record<string, string> = {}) =>
  new Request("https://catdo.workercat.com/api/sync?user_id=user_b", {
    headers,
  });
it("rejects anonymous and cookie-only calls without touching storage", async () => {
  expect((await worker.fetch(request(), env)).status).toBe(401);
  expect(
    (await worker.fetch(request({ cookie: "session=fixture" }), env)).status,
  ).toBe(401);
  expect(getByName).not.toHaveBeenCalled();
});
it("uses the verified Clerk identity rather than any client-supplied account ID", async () => {
  const result = await worker.fetch(
    request({ authorization: "Bearer fixture", "x-user-id": "user_b" }),
    env,
  );
  expect(result.status).toBe(200);
  expect(getByName).toHaveBeenCalledWith("user_a");
  expect(auth.verify).toHaveBeenCalledWith(expect.any(Request), {
    acceptsToken: "session_token",
    authorizedParties: ["https://catdo.workercat.com"],
  });
});
it("rejects an unrelated origin even with a bearer token", async () => {
  expect(
    (
      await worker.fetch(
        request({
          authorization: "Bearer fixture",
          origin: "https://elsewhere.example",
        }),
        env,
      )
    ).status,
  ).toBe(403);
  expect(auth.verify).not.toHaveBeenCalled();
});
it("rejects expired sessions and OAuth tokens issued to another client", async () => {
  auth.value = {
    isAuthenticated: false,
    tokenType: "session_token",
    userId: null,
  };
  expect(
    (await worker.fetch(request({ authorization: "Bearer fixture" }), env))
      .status,
  ).toBe(401);
  auth.value = {
    isAuthenticated: true,
    tokenType: "oauth_token",
    userId: "user_a",
    clientId: "unrelated-app",
    scopes: ["openid"],
  };
  expect(
    (await worker.fetch(request({ authorization: "Bearer fixture" }), env))
      .status,
  ).toBe(403);
  expect(getByName).not.toHaveBeenCalled();
});
it("accepts only the configured desktop OAuth client", async () => {
  auth.value = {
    isAuthenticated: true,
    tokenType: "oauth_token",
    userId: "user_a",
    clientId: "catdo-desktop",
    scopes: ["openid"],
  };
  expect(
    (await worker.fetch(request({ authorization: "Bearer fixture" }), env))
      .status,
  ).toBe(200);
  expect(getByName).toHaveBeenCalledWith("user_a");
  expect(auth.verify).toHaveBeenCalledWith(expect.any(Request), {
    acceptsToken: "oauth_token",
  });
});
