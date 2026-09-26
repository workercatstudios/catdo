import { beforeEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({
  fetch: vi.fn(),
  download: vi.fn(),
  api: vi.fn(),
}));
vi.mock("@tanstack/react-start/server-entry", () => ({
  default: { fetch: mocks.fetch },
}));
vi.mock("./server/account", () => ({ CatDoAccount: class {} }));
vi.mock("./server/download", () => ({ androidDownload: mocks.download }));
vi.mock("cloudflare:workers", () => ({ env: {} }));
vi.mock("./server/index", () => ({ default: { fetch: mocks.api } }));
import worker from "./server";

beforeEach(() => vi.resetAllMocks());
const request = (path = "/", headers = {}, method = "GET") =>
  new Request(`https://catdo.example${path}`, { headers, method });

it("passes WebSocket upgrades directly to the API", async () => {
  const upgrade = new Response("upgrade passthrough");
  mocks.api.mockResolvedValue(upgrade);
  expect(await worker.fetch(request("/api/sync/events"))).toBe(upgrade);
  expect(mocks.fetch).not.toHaveBeenCalled();
});

it.each([
  "/app",
  "/app/workspaces/test",
  "/privacy-requests",
  "/api/config",
  "/api/health",
])("never puts %s in shared caches", async (path) => {
  mocks.fetch.mockResolvedValue(new Response("ok"));
  const response = await worker.fetch(request(path));
  expect(response.headers.get("Cache-Control")).toBe("no-store");
  expect(response.headers.get("X-Robots-Tag")).toContain("noindex");
});

it("caches anonymous public pages but not credential-bearing requests or responses", async () => {
  mocks.fetch.mockImplementation(() => new Response("ok"));
  expect(
    (await worker.fetch(request())).headers.get("Cache-Control"),
  ).toContain("s-maxage=300");
  for (const headers of [
    { authorization: "Bearer secret" },
    { cookie: "session=secret" },
  ])
    expect(
      (await worker.fetch(request("/", headers))).headers.get("Cache-Control"),
    ).toBe("no-store");
  mocks.fetch.mockResolvedValue(
    new Response("ok", { headers: { "Set-Cookie": "session=secret" } }),
  );
  expect((await worker.fetch(request())).headers.get("Cache-Control")).toBe(
    "no-store",
  );
});

it("preserves deliberate cache policy and applies security headers to download redirects", async () => {
  mocks.download.mockResolvedValue(
    new Response(null, {
      status: 302,
      headers: {
        Location: "https://github.com/example",
        "Cache-Control": "public, max-age=60",
      },
    }),
  );
  const response = await worker.fetch(request("/download/android"));
  expect(response.status).toBe(302);
  expect(response.headers.get("Cache-Control")).toBe("public, max-age=60");
  expect(response.headers.get("X-Content-Type-Options")).toBe("nosniff");
  expect(response.headers.get("Content-Security-Policy")).toContain(
    "frame-ancestors 'none'",
  );
  expect(mocks.fetch).not.toHaveBeenCalled();
});

it("returns a non-cacheable, sanitized response when rendering fails", async () => {
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  mocks.fetch.mockRejectedValue(
    new Error("secret credentials and task contents"),
  );
  const response = await worker.fetch(request());
  expect(response.status).toBe(503);
  expect(response.headers.get("Retry-After")).toBe("30");
  expect(response.headers.get("Cache-Control")).toBe("no-store");
  expect(await response.text()).not.toContain("secret");
  expect(JSON.stringify(log.mock.calls)).not.toContain("secret");
  log.mockRestore();
});

it("does not cache error responses or send a HEAD body", async () => {
  mocks.fetch.mockImplementation(
    () => new Response("not found", { status: 404 }),
  );
  const response = await worker.fetch(request("/missing", {}, "HEAD"));
  expect(response.status).toBe(404);
  expect(response.headers.get("X-Robots-Tag")).toBe("noindex");
  expect(response.headers.get("Cache-Control")).toBe("no-store");
  expect(await response.text()).toBe("");
});
