#!/usr/bin/env node
import { readFile, open, mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";

// Private operator tool. Never put the token or request contents in arguments,
// logs, shell history, or Git. Output is written to a new mode-0600 file.
const [action, ...args] = process.argv.slice(2);
const options = new Map();
const allowed = new Set([
  "--out",
  "--id",
  "--response-file",
  "--confirm",
  "--cursor-file",
]);
for (let i = 0; i < args.length; i += 2) {
  if (!allowed.has(args[i]) || !args[i + 1] || options.has(args[i]))
    throw new Error(
      "Expected unique --out, --id, --response-file, --confirm, or --cursor-file options.",
    );
  options.set(args[i], args[i + 1]);
}
if (
  !options.has("--out") ||
  !["list", "resolve", "export", "erase-cloud-data"].includes(action)
)
  throw new Error(
    "Usage: node scripts/privacy-admin.mjs list|resolve|export|erase-cloud-data --out PRIVATE_FILE [options]",
  );
const url = new URL("https://catdo.workercat.com/api/admin/privacy-requests");
let body;
if (action === "list") {
  if (options.has("--cursor-file")) {
    const { nextCursor } = JSON.parse(
      await readFile(options.get("--cursor-file"), "utf8"),
    );
    if (!nextCursor?.createdAt || !nextCursor?.id)
      throw new Error("The cursor file has no next page.");
    url.searchParams.set("createdAt", nextCursor.createdAt);
    url.searchParams.set("id", nextCursor.id);
  }
} else {
  const id = options.get("--id");
  if (!id) throw new Error("A verified request --id is required.");
  if (action === "resolve") {
    if (!options.has("--response-file"))
      throw new Error("--response-file is required.");
    body = {
      id,
      status: "resolved",
      response: (await readFile(options.get("--response-file"), "utf8")).trim(),
    };
  } else if (action === "export") {
    body = { id, action };
  } else {
    if (options.get("--confirm") !== "erase-cloud-data")
      throw new Error(
        "Cloud erasure is permanent. Pass --confirm erase-cloud-data after reviewing the request.",
      );
    body = { id, action, confirm: "erase-cloud-data" };
  }
}
const output = resolve(options.get("--out"));
await mkdir(dirname(output), { recursive: true, mode: 0o700 });
// Reserve the private output before making any potentially irreversible request.
const file = await open(output, "wx", 0o600);
try {
  const token = (
    await readFile(
      `${homedir()}/.config/workercat/catdo-privacy-admin-token`,
      "utf8",
    )
  ).trim();
  if (token.length < 32)
    throw new Error("Missing or invalid private operator token.");
  const response = await fetch(url, {
    method: body ? "POST" : "GET",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
    redirect: "error",
    signal: AbortSignal.timeout(15_000),
  });
  const result = await response.json();
  await file.writeFile(`${JSON.stringify(result, null, 2)}\n`);
  if (!response.ok)
    throw new Error(
      `Operator request failed (${response.status}); inspect the private output file.`,
    );
  process.stdout.write(`Saved private result to ${output}\n`);
} finally {
  await file.close();
}
