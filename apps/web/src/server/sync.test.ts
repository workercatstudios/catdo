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
function account() {
  const db = new DatabaseSync(":memory:");
  const storage: AccountStorage = {
    sql: {
      exec(query, ...values) {
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
