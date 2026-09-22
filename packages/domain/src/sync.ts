import { type Data, emptyData, validateData } from "./model";
export const collections = [
  "workspaces",
  "projects",
  "tasks",
  "history",
] as const;
export type Conflict = {
  collection: (typeof collections)[number];
  id: string;
  name: string;
};
export type Snapshot = { revision: number; data: Data };
export type Pending = { id: string; base: Data; data: Data };
export type SyncState = {
  owner: string;
  base: Snapshot;
  pending: Pending | null;
  data: Data;
};
export function equal(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (
    a === null ||
    b === null ||
    typeof a !== "object" ||
    typeof b !== "object"
  )
    return false;
  if (Array.isArray(a) || Array.isArray(b))
    return (
      Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((v, i) => equal(v, b[i]))
    );
  const x = a as Record<string, unknown>,
    y = b as Record<string, unknown>;
  return (
    Object.keys(x).length === Object.keys(y).length &&
    Object.keys(x).every((k) => equal(x[k], y[k]))
  );
}
/** Whole-record conflicts keep completion, recurrence and task moves indivisible.
 * Deletions remain absent when a stale client has not changed that record. */
export function merge(
  base: Data,
  local: Data,
  remote: Data,
  choice?: "local" | "remote",
): { data: Data; conflicts: Conflict[] } {
  const data = emptyData(),
    conflicts: Conflict[] = [];
  for (const collection of collections) {
    const b = new Map(base[collection].map((x) => [x.id, x]));
    const l = new Map(local[collection].map((x) => [x.id, x]));
    const r = new Map(remote[collection].map((x) => [x.id, x]));
    const result: unknown[] = [];
    for (const id of new Set([...r.keys(), ...l.keys(), ...b.keys()])) {
      const before = b.get(id),
        mine = l.get(id),
        theirs = r.get(id);
      let value: unknown;
      if (equal(mine, before)) value = theirs;
      else if (equal(theirs, before) || equal(mine, theirs)) value = mine;
      else {
        const record = (mine ?? theirs) as
          { name?: string; title?: string } | undefined;
        conflicts.push({
          collection,
          id,
          name: record?.title ?? record?.name ?? "Completed occurrence",
        });
        value = choice === "remote" ? theirs : mine;
      }
      if (value !== undefined) result.push(value);
    }
    (data[collection] as unknown[]) = result;
  }
  // A completion and its history are one change. Concurrent completion of the
  // same task must not create two history entries or advance it twice.
  const originalHistory = new Set(base.history.map((h) => h.id));
  const localAdded = local.history.filter((h) => !originalHistory.has(h.id));
  const remoteAdded = remote.history.filter((h) => !originalHistory.has(h.id));
  const conflictingTasks = new Set(
    conflicts.filter((c) => c.collection === "tasks").map((c) => c.id),
  );
  for (const mine of localAdded) {
    if (
      remoteAdded.some(
        (theirs) => theirs.task.id === mine.task.id && theirs.id !== mine.id,
      )
    ) {
      if (!conflictingTasks.has(mine.task.id))
        conflicts.push({
          collection: "tasks",
          id: mine.task.id,
          name: mine.task.title,
        });
      conflictingTasks.add(mine.task.id);
    }
  }
  const chosenHistory = new Set(
    (choice === "remote" ? remote : local).history.map((h) => h.id),
  );
  data.history = data.history.filter(
    (h) =>
      !conflictingTasks.has(h.task.id) ||
      originalHistory.has(h.id) ||
      chosenHistory.has(h.id),
  );
  return { data, conflicts };
}
export function reconcile(
  state: SyncState,
  remote: Snapshot,
  choice?: "local" | "remote",
) {
  const result = merge(state.base.data, state.data, remote.data, choice);
  if (result.conflicts.length && !choice)
    return { conflicts: result.conflicts };
  validateData(result.data);
  return {
    state: { ...state, base: remote, pending: null, data: result.data },
    conflicts: result.conflicts,
  };
}
