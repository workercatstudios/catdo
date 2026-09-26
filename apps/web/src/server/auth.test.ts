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
  push: vi.fn(async () => ({ status: 200, body: { revision: 1 } })),
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
it("accepts a verified native session without an origin or azp", async () => {
  auth.verify.mockImplementation(async (_request, options) => ({
    reason: options.authorizedParties
      ? "token-invalid-authorized-parties"
      : undefined,
    toAuth: () =>
      options.acceptsToken === "session_token" && !options.authorizedParties
        ? {
            isAuthenticated: true,
            tokenType: "session_token",
            userId: "user_a",
            sessionClaims: {},
          }
        : null,
  }));
  expect(
    (await worker.fetch(request({ authorization: "Bearer fixture" }), env))
      .status,
  ).toBe(200);
  expect(getByName).toHaveBeenCalledWith("user_a");
  expect(auth.verify).toHaveBeenCalledWith(expect.any(Request), {
    acceptsToken: "session_token",
  });
});
it("does not accept a session from a different browser origin as native", async () => {
  auth.verify.mockImplementation(async (_request, options) => ({
    reason: options.authorizedParties
      ? "token-invalid-authorized-parties"
      : undefined,
    toAuth: () =>
      options.acceptsToken === "session_token" && !options.authorizedParties
        ? {
            isAuthenticated: true,
            tokenType: "session_token",
            userId: "user_a",
            sessionClaims: { azp: "https://elsewhere.example" },
          }
        : null,
  }));
  expect(
    (await worker.fetch(request({ authorization: "Bearer fixture" }), env))
      .status,
  ).toBe(401);
  expect(getByName).not.toHaveBeenCalled();
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

it.each([
  ["/api/config", "POST", "GET"],
  ["/api/me", "POST", "GET"],
  ["/api/sync", "DELETE", "GET, POST"],
  ["/api/health", "POST", "GET, HEAD"],
])("rejects %s %s before authentication", async (path, method, allow) => {
  const response = await worker.fetch(
    new Request(`https://catdo.example${path}`, { method }),
    env,
  );
  expect(response.status).toBe(405);
  expect(response.headers.get("Allow")).toBe(allow);
  expect(auth.verify).not.toHaveBeenCalled();
  expect(getByName).not.toHaveBeenCalled();
});

it("reports liveness without secrets, authentication, or storage", async () => {
  for (const method of ["GET", "HEAD"]) {
    const response = await worker.fetch(
      new Request("https://catdo.example/api/health", { method }),
      {} as Env,
    );
    expect(response.status).toBe(200);
    expect(response.headers.get("Cache-Control")).toBe("no-store");
    expect(await response.text()).toBe(
      method === "GET" ? '{"status":"ok"}' : "",
    );
  }
  expect(auth.verify).not.toHaveBeenCalled();
  expect(getByName).not.toHaveBeenCalled();
});

it.each(["", "not a URL", "https://catdo.example/path", "file:///tmp"])(
  "fails closed on malformed APP_ORIGIN %s",
  async (origin) => {
    const response = await worker.fetch(
      request({ authorization: "Bearer fixture" }),
      { ...env, APP_ORIGIN: origin },
    );
    expect(response.status).toBe(503);
    expect(response.headers.get("Retry-After")).toBe("30");
    expect(auth.verify).not.toHaveBeenCalled();
    expect(getByName).not.toHaveBeenCalled();
  },
);

it("rejects malformed uploads before invoking account storage", async () => {
  for (const [body, type, status] of [
    ["{}", "application/jsonp", 415],
    ["{", "application/json", 400],
    ["{}", "application/json", 400],
  ] as const) {
    const response = await worker.fetch(
      new Request("https://catdo.example/api/sync", {
        method: "POST",
        headers: { Authorization: "Bearer fixture", "Content-Type": type },
        body,
      }),
      env,
    );
    expect(response.status).toBe(status);
  }
  expect(getByName).not.toHaveBeenCalled();
});

it("keeps dependency failures out of API responses", async () => {
  auth.verify.mockRejectedValueOnce(new Error("secret token or internal URL"));
  const response = await worker.fetch(
    request({ authorization: "Bearer fixture" }),
    env,
  );
  expect(response.status).toBe(503);
  expect(await response.text()).not.toContain("secret");
  expect(response.headers.get("Retry-After")).toBe("30");
});
