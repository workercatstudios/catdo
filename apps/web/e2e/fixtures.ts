import type { Page } from "@playwright/test";
import {
  newTask,
  initialData,
  localDay,
} from "../../../packages/domain/src/model";
export const owner = "catdo-browser-fixture";
export function fixtureData() {
  const data = initialData();
  const workspace = data.workspaces[0].id;
  data.projects.push({
    id: "e9659e30-a353-48e6-b1f3-337ac6881e5e",
    workspace_id: workspace,
    name: "A little project",
    archived: false,
  });
  data.tasks.push(
    newTask(
      workspace,
      "Plan the next small step",
      "e9659e30-a353-48e6-b1f3-337ac6881e5e",
      localDay(),
    ),
    newTask(workspace, "Make time for a walk", null, localDay()),
  );
  return data;
}
export async function seed(page: Page, data = fixtureData()) {
  await page.goto("/");
  await page.evaluate(
    async ({ data, owner }) => {
      localStorage.setItem("catdo:last-user", owner);
      localStorage.setItem("catdo:theme", "light");
      const db = await new Promise<IDBDatabase>((resolve, reject) => {
        const req = indexedDB.open("catdo", 1);
        req.onupgradeneeded = () =>
          req.result.createObjectStore("accounts", { keyPath: "owner" });
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
      });
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction("accounts", "readwrite");
        tx.objectStore("accounts").put({
          owner,
          data,
          base: { revision: 0, data: structuredClone(data) },
          pending: null,
          conflict: null,
        });
        tx.oncomplete = () => resolve();
        tx.onerror = () => reject(tx.error);
      });
      db.close();
    },
    { data, owner },
  );
  await page.waitForFunction(() => !!navigator.serviceWorker.controller);
  return data;
}
export async function openOffline(page: Page) {
  await page.context().setOffline(true);
  await page.goto("/app");
  await page.getByRole("button", { name: "Open saved tasks" }).click();
  await page
    .getByRole("heading", { name: "Today", exact: true, level: 1 })
    .waitFor();
}
