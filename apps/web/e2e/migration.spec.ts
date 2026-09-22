import { test, expect } from "@playwright/test";
import { createServer } from "node:http";
import { readFile, access } from "node:fs/promises";
import { resolve, extname } from "node:path";
import { fixtureData, owner } from "./fixtures";

// Exercise a real service-worker update on the same origin. The fixture retains
// the shipped worker's scope/cache behavior; no production credentials or data.
test("upgrades the old service worker without dropping offline account data", async ({
  browser,
}) => {
  let upgraded = false;
  const oldWorker = (await readFile("apps/web/e2e/legacy-worker.js", "utf8"))
    .replace("__VERSION__", "migration-fixture")
    .replace("__ASSETS__", JSON.stringify(["/app"]));
  const oldHTML =
    '<!doctype html><html><head><title>Legacy CatDo</title></head><body><h1>Legacy CatDo</h1><script>navigator.serviceWorker.register("/sw.js")</script></body></html>';
  const legacyRoot = resolve(".local/legacy-browser/apps/web/dist");
  const realLegacy = await access(legacyRoot + "/index.html").then(
    () => true,
    () => false,
  );
  const server = createServer(async (req, res) => {
    const path = new URL(req.url!, "http://localhost").pathname;
    try {
      if (path === "/api/config") {
        res.writeHead(503, { "Content-Type": "application/json" });
        res.end('{"error":"Offline migration fixture"}');
        return;
      }
      if (!upgraded) {
        if (realLegacy) {
          const file = resolve(
            legacyRoot,
            path.startsWith("/assets/") || ["/sw.js", "/cat.png"].includes(path)
              ? "." + path
              : "index.html",
          );
          if (!file.startsWith(legacyRoot + "/")) {
            res.writeHead(404);
            res.end();
            return;
          }
          const body = await readFile(file);
          const type: Record<string, string> = {
            ".js": "text/javascript",
            ".css": "text/css",
            ".html": "text/html",
            ".png": "image/png",
          };
          res.writeHead(200, {
            "Content-Type": type[extname(file)] || "application/octet-stream",
            "Cache-Control": "no-store",
          });
          res.end(body);
          return;
        }
        res.writeHead(200, {
          "Content-Type": path === "/sw.js" ? "text/javascript" : "text/html",
          "Cache-Control": "no-store",
        });
        res.end(path === "/sw.js" ? oldWorker : oldHTML);
        return;
      }
      const response = await fetch("http://127.0.0.1:4173" + req.url, {
        redirect: "manual",
      });
      const headers = Object.fromEntries(response.headers);
      delete headers["content-encoding"];
      delete headers["content-length"];
      res.writeHead(response.status, headers);
      res.end(Buffer.from(await response.arrayBuffer()));
    } catch {
      res.writeHead(500);
      res.end("Fixture request failed");
    }
  });
  await new Promise<void>((r) => server.listen(4176, "127.0.0.1", r));
  const context = await browser.newContext();
  const page = await context.newPage();
  try {
    await page.goto("http://127.0.0.1:4176/app");
    const data = fixtureData();
    data.tasks[0].title = "Saved before the rewrite";
    await page.evaluate(
      async ({ data, owner }) => {
        localStorage.setItem("catdo:last-user", owner);
        const db = await new Promise<IDBDatabase>((resolve) => {
          const req = indexedDB.open("catdo", 1);
          req.onupgradeneeded = () =>
            req.result.createObjectStore("accounts", { keyPath: "owner" });
          req.onsuccess = () => resolve(req.result);
        });
        await new Promise<void>((resolve) => {
          const tx = db.transaction("accounts", "readwrite");
          tx.objectStore("accounts").put({
            owner,
            data,
            base: { revision: 0, data: { ...data, tasks: [] } },
            pending: null,
            conflict: null,
          });
          tx.oncomplete = () => resolve();
        });
        db.close();
      },
      { data, owner },
    );
    await page.waitForFunction(() => !!navigator.serviceWorker.controller);
    upgraded = true;
    await page.evaluate(async () => {
      const registration = await navigator.serviceWorker.getRegistration();
      await registration!.update();
    });
    await page.waitForFunction(
      async () => !!(await navigator.serviceWorker.getRegistration())?.waiting,
    );
    await page.reload();
    await expect(page.getByText("A fresh CatDo is ready.")).toBeVisible();
    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: "Reload", exact: true }).click();
    await expect(page.getByText("A fresh CatDo is ready.")).toHaveCount(0);
    await page.getByRole("button", { name: "Open saved tasks" }).click();
    await expect(
      page.getByRole("button", { name: "Complete Saved before the rewrite" }),
    ).toBeVisible();
    await context.setOffline(true);
    await page.reload();
    await page.getByRole("button", { name: "Open saved tasks" }).click();
    await expect(
      page.getByRole("button", { name: "Complete Saved before the rewrite" }),
    ).toBeVisible();
    const cachesLeft = await page.evaluate(() => caches.keys());
    expect(
      cachesLeft.every(
        (key) =>
          !key.startsWith("catdo-shell-") ||
          key.startsWith("catdo-shell-start-"),
      ),
    ).toBe(true);
  } finally {
    await context.close();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
});
