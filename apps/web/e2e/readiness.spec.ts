import { test, expect } from "@playwright/test";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

test("production smoke script and static asset headers", async ({
  request,
  baseURL,
}) => {
  const { stdout } = await promisify(execFile)(process.execPath, [
    "scripts/check-web.mjs",
    baseURL!,
    "--allow-unconfigured-auth",
  ]);
  expect(stdout).toContain("Web smoke checks passed");
  const response = await request.get("/sw.js");
  expect(response.headers()["cache-control"]).toBe("no-cache");
  expect(response.headers()["x-content-type-options"]).toBe("nosniff");
});

test("real HTTP routing preserves method errors and health HEAD semantics", async ({
  request,
}) => {
  for (const [path, allow] of [
    ["/api/health", "GET, HEAD"],
    ["/api/config", "GET"],
    ["/api/me", "GET"],
  ]) {
    const response = await request.post(path);
    expect(response.status()).toBe(405);
    expect(response.headers().allow).toBe(allow);
    expect(response.headers()["cache-control"]).toBe("no-store");
  }
  const health = await request.head("/api/health");
  expect(health.status()).toBe(200);
  expect(await health.text()).toBe("");
  const unknown = await request.get("/api/missing");
  expect(unknown.status()).toBe(404);
  expect(unknown.headers()["cache-control"]).toBe("no-store");
});
