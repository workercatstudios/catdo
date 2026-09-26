import assert from "node:assert/strict";

const [target, ...flags] = process.argv.slice(2);
if (!target || flags.some((flag) => flag !== "--allow-unconfigured-auth")) {
  console.error(
    "Usage: node scripts/check-web.mjs <origin> [--allow-unconfigured-auth]",
  );
  process.exit(1);
}
const origin = new URL(target);
assert(
  ["http:", "https:"].includes(origin.protocol),
  "Expected an HTTP(S) origin",
);
assert(
  !origin.username &&
    !origin.password &&
    origin.pathname === "/" &&
    !origin.search &&
    !origin.hash,
  "Supply only an origin, without credentials, path, query, or fragment",
);

async function get(path, status = 200) {
  const response = await fetch(new URL(path, origin), {
    redirect: "manual",
    signal: AbortSignal.timeout(10_000),
  });
  assert.equal(response.status, status, `${path}: unexpected HTTP status`);
  assert.equal(
    response.headers.get("x-content-type-options"),
    "nosniff",
    `${path}: missing nosniff`,
  );
  return response;
}
function privateResponse(response) {
  assert.equal(
    response.headers.get("cache-control"),
    "no-store",
    "Private response must not be cached",
  );
}

try {
  const health = await get("/api/health");
  privateResponse(health);
  assert.deepEqual(
    await health.json(),
    { status: "ok" },
    "Unexpected liveness response",
  );

  const home = await get("/");
  assert(
    home.headers.get("content-type")?.includes("text/html"),
    "Homepage must be HTML",
  );
  assert.equal(home.headers.get("x-frame-options"), "DENY");
  assert(
    home.headers
      .get("content-security-policy")
      ?.includes("frame-ancestors 'none'"),
  );
  const html = await home.text();
  assert(
    html.includes("<h1") && html.includes("CatDo"),
    "Homepage did not render",
  );
  const asset = html.match(
    /(?:src|href)="(\/assets\/[^"?#]+\.(?:js|css))"/,
  )?.[1];
  assert(asset, "Homepage must reference a built asset");
  await (await get(asset)).arrayBuffer();

  const app = await get("/app");
  privateResponse(app);
  assert(app.headers.get("x-robots-tag")?.includes("noindex"));
  assert((await app.text()).includes("CatDo"), "App shell did not render");

  const config = await get("/api/config");
  privateResponse(config);
  const settings = await config.json();
  for (const name of ["publishableKey", "issuer", "desktopClientId"])
    assert(
      typeof settings[name] === "string" && settings[name].length > 0,
      `Missing public auth configuration: ${name}`,
    );

  const anonymous = await fetch(new URL("/api/sync", origin), {
    redirect: "manual",
    signal: AbortSignal.timeout(10_000),
  });
  assert(
    flags.includes("--allow-unconfigured-auth")
      ? [401, 503].includes(anonymous.status)
      : anonymous.status === 401,
    "Anonymous sync must be denied (production must have auth configured)",
  );
  privateResponse(anonymous);
  await anonymous.arrayBuffer();

  const sw = await get("/sw.js");
  assert.equal(
    sw.headers.get("cache-control"),
    "no-cache",
    "Service worker must revalidate",
  );
  const worker = await sw.text();
  assert(
    worker.includes("catdo-shell-start-") &&
      !worker.includes("__VERSION__") &&
      !worker.includes("__ASSETS__"),
    "Service worker was not built",
  );
  console.log(
    `Web smoke checks passed for ${origin.origin}. Authenticated sync requires a separate test account.`,
  );
} catch (error) {
  console.error(
    `Web smoke check failed: ${error instanceof Error ? error.message : "unknown failure"}`,
  );
  process.exitCode = 1;
}
