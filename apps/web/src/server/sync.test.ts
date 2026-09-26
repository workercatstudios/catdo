import { DatabaseSync } from "node:sqlite";
import { it, expect } from "vitest";
import { AccountStore, type AccountStorage } from "./sync";
import {
  emptyData,
  initialData,
  newTask,
  saveTask,
} from "../../../../packages/domain/src/model";
import type { Pending } from "../../../../packages/domain/src/sync";
function account(failReceipts = false) {
  const db = new DatabaseSync(":memory:");
  const storage: AccountStorage = {
    sql: {
      exec(query, ...values) {
        if (failReceipts && query.startsWith("INSERT INTO receipts"))
          throw new Error("Simulated storage failure");
        return db
          .prepare(query)
          .all(...(values as (string | number | null)[])) as Record<
          string,
          string | number | null
        >[];
      },
    },
    transactionSync(fn) {
      db.exec("BEGIN");
      try {
        const value = fn();
        db.exec("COMMIT");
        return value;
      } catch (error) {
        db.exec("ROLLBACK");
        throw error;
      }
    },
  };
  return new AccountStore(storage);
}
function initial() {
  const data = initialData();
  saveTask(data, newTask(data.workspaces[0].id, "Task"));
  return { id: crypto.randomUUID(), base: emptyData(), data };
}
it("atomically accepts a change and recognizes a lost-response retry after further edits", () => {
  const store = account(),
    first = initial();
  expect(store.push(first).status).toBe(200);
  const next = structuredClone(first.data);
  next.tasks[0].title = "Another device";
  expect(
    store.push({ id: crypto.randomUUID(), base: first.data, data: next })
      .status,
  ).toBe(200);
  const retry = store.push(first);
  expect(retry.status).toBe(200);
  expect(store.snapshot().revision).toBe(2);
  expect(store.snapshot().data.tasks[0].title).toBe("Another device");
});

it("rejects invalid record relationships without changing storage or consuming the operation ID", () => {
  const store = account();
  const valid = initial();
  const invalid = structuredClone(valid);
  invalid.data.tasks[0].workspace_id = crypto.randomUUID();
  expect(store.push(invalid)).toEqual({
    status: 400,
    body: { error: "Invalid task data." },
  });
  expect(store.snapshot()).toEqual({ revision: 0, data: emptyData() });
  expect(store.push(valid).status).toBe(200);
});

it("rolls back the snapshot if saving its retry receipt fails", () => {
  const store = account(true);
  expect(() => store.push(initial())).toThrow("Simulated storage failure");
  expect(store.snapshot()).toEqual({ revision: 0, data: emptyData() });
});

it("enforces the account byte limit without losing the previous snapshot", () => {
  const store = account();
  const first = initial();
  store.push(first);
  const data = structuredClone(first.data);
  for (let i = 0; i < 16; i++) {
    const task = newTask(data.workspaces[0].id, `Task ${i}`);
    task.notes = "猫".repeat(20_000);
    data.tasks.push(task);
  }
  expect(
    store.push({ id: crypto.randomUUID(), base: first.data, data }).status,
  ).toBe(413);
  expect(store.snapshot()).toEqual({ revision: 1, data: first.data });
});
it("isolates account data and receipts", () => {
  const a = account(),
    b = account(),
    first = initial();
  a.push(first);
  expect(b.snapshot().data).toEqual(emptyData());
  expect(b.push(first).status).toBe(200);
  expect(b.snapshot().revision).toBe(1);
});
it("rejects conflicting stale writes without consuming their operation IDs", () => {
  const store = account(),
    first = initial();
  store.push(first);
  const a = structuredClone(first.data),
    b = structuredClone(first.data);
  a.tasks[0].title = "A";
  b.tasks[0].title = "B";
  store.push({ id: crypto.randomUUID(), base: first.data, data: a });
  const pending: Pending = {
    id: crypto.randomUUID(),
    base: first.data,
    data: b,
  };
  expect(store.push(pending).status).toBe(409);
  expect(store.snapshot().revision).toBe(2);
  expect(store.push({ ...pending, base: a }).status).toBe(200);
  expect(store.snapshot().data.tasks[0].title).toBe("B");
});
it("does not resurrect a remotely deleted task on a stale unrelated edit", () => {
  const store = account(),
    first = initial();
  store.push(first);
  const remote = structuredClone(first.data);
  remote.tasks = [];
  store.push({ id: crypto.randomUUID(), base: first.data, data: remote });
  const local = structuredClone(first.data);
  local.workspaces[0].name = "Renamed";
  expect(
    store.push({ id: crypto.randomUUID(), base: first.data, data: local })
      .status,
  ).toBe(200);
  expect(store.snapshot().data.tasks).toEqual([]);
});

it("joins a new device's Personal workspace to the existing account", () => {
  const store = account();
  const desktop = initial();
  expect(store.push(desktop).status).toBe(200);
  const phone = initial();
  phone.data.tasks[0].title = "Raid with the boys";
  phone.data.tasks[0].scheduled = "2026-09-23";
  const phoneWorkspace = phone.data.workspaces[0].id;
  phone.data.projects.push({
    id: crypto.randomUUID(),
    workspace_id: phoneWorkspace,
    name: "Plans",
    archived: false,
  });
  const result = store.push(phone);
  expect(result.status).toBe(200);
  const synced = store.snapshot().data;
  expect(synced.workspaces).toEqual(desktop.data.workspaces);
  expect(synced.tasks.map((t) => t.title).sort()).toEqual([
    "Raid with the boys",
    "Task",
  ]);
  expect(synced.tasks.map((t) => t.workspace_id)).toEqual([
    desktop.data.workspaces[0].id,
    desktop.data.workspaces[0].id,
  ]);
  expect(synced.projects[0].workspace_id).toBe(desktop.data.workspaces[0].id);
  expect(store.push(phone).status).toBe(200);
  expect(store.snapshot().data.tasks).toHaveLength(2);
});
