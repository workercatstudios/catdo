import {
  type Data,
  emptyData,
  initialData,
  validateData,
} from "../../../packages/domain/src/model";
import {
  type Conflict,
  type Pending,
  type Snapshot,
  merge,
  equal,
} from "../../../packages/domain/src/sync";
export type LocalState = {
  owner: string;
  data: Data;
  base: Snapshot;
  pending: Pending | null;
  conflict: Snapshot | null;
};
const database = new Promise<IDBDatabase>((resolve, reject) => {
  const req = indexedDB.open("catdo", 1);
  req.onupgradeneeded = () =>
    req.result.createObjectStore("accounts", { keyPath: "owner" });
  req.onsuccess = () => resolve(req.result);
  req.onerror = () => reject(req.error);
});
async function read(owner: string): Promise<LocalState | undefined> {
  const db = await database;
  return new Promise((resolve, reject) => {
    const req = db.transaction("accounts").objectStore("accounts").get(owner);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}
async function write(state: LocalState) {
  const db = await database;
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction("accounts", "readwrite");
    tx.objectStore("accounts").put(state);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error ?? Error("Local save was interrupted."));
  });
}
export class TaskStore {
  state: LocalState | null = null;
  status = "Opening your tasks…";
  conflicts: Conflict[] = [];
  private listeners = new Set<() => void>();
  private channel: BroadcastChannel;
  private stopped = false;
  private syncing = false;
  private timer: ReturnType<typeof setInterval> | null = null;
  constructor(
    readonly owner: string,
    private token: () => Promise<string | null>,
  ) {
    this.channel = new BroadcastChannel(`catdo:${owner}`);
    this.channel.onmessage = () => {
      void this.reload();
    };
  }
  subscribe = (fn: () => void) => {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  };
  private emit() {
    for (const fn of this.listeners) fn();
  }
  private async reload() {
    this.state = (await read(this.owner)) ?? null;
    this.updateConflicts();
    this.emit();
  }
  private updateConflicts() {
    this.conflicts = this.state?.conflict
      ? merge(this.state.base.data, this.state.data, this.state.conflict.data)
          .conflicts
      : [];
  }
  private lock<T>(fn: () => Promise<T>) {
    return navigator.locks.request(`catdo-data:${this.owner}`, fn);
  }
  async open() {
    try {
      await this.lock(async () => {
        let state = await read(this.owner);
        if (!state) {
          let remote: Snapshot = { revision: 0, data: emptyData() };
          try {
            remote = (await this.request("GET")) as Snapshot;
          } catch {
            /* A new account can start offline. */
          }
          state = {
            owner: this.owner,
            base: remote,
            data: remote.data.workspaces.length ? remote.data : initialData(),
            pending: null,
            conflict: null,
          };
          await write(state);
        }
        this.state = state;
      });
      if (this.stopped) return;
      this.status = "Saved on this device";
      this.updateConflicts();
      this.emit();
      this.timer = setInterval(() => {
        void this.sync();
      }, 10000);
      void this.sync();
    } catch (e) {
      this.status = `Could not open local storage: ${message(e)}`;
      this.emit();
    }
  }
  stop() {
    this.stopped = true;
    if (this.timer) clearInterval(this.timer);
    this.channel.close();
  }
  async change(fn: (data: Data) => void) {
    await this.lock(async () => {
      const state = await read(this.owner);
      if (!state) throw Error("Tasks are still opening.");
      const data = structuredClone(state.data);
      fn(data);
      validateData(data);
      const next = { ...state, data };
      await write(next);
      this.state = next;
      this.status = "Saved on this device · waiting to sync";
      this.updateConflicts();
      this.emit();
      this.channel.postMessage("change");
    });
    void this.sync();
  }
  private async request(method: "GET" | "POST", pending?: Pending) {
    const token = await this.token();
    if (!token) throw Error("Sign in to sync. Your changes are saved here.");
    const response = await fetch("/api/sync", {
      method,
      headers: {
        Authorization: `Bearer ${token}`,
        ...(pending ? { "Content-Type": "application/json" } : {}),
      },
      body: pending ? JSON.stringify(pending) : undefined,
      signal: AbortSignal.timeout(20000),
    });
    const body = (await response.json()) as Snapshot & { error?: string };
    if (response.status === 409) return { ...body, conflict: true };
    if (!response.ok) throw Error(body.error ?? "Could not sync.");
    return body as Snapshot & { conflict?: boolean };
  }
  async sync() {
    if (this.syncing || this.stopped) return;
    this.syncing = true;
    try {
      await navigator.locks.request(
        `catdo-sync:${this.owner}`,
        { ifAvailable: true },
        async (lock) => {
          if (!lock) return;
          let pending: Pending | null = null;
          await this.lock(async () => {
            const state = await read(this.owner);
            if (!state || state.conflict) return;
            if (!state.pending && !equal(state.base.data, state.data)) {
              state.pending = {
                id: crypto.randomUUID(),
                base: state.base.data,
                data: state.data,
              };
              await write(state);
            }
            pending = state.pending;
            this.state = state;
          });
          if (!this.state || this.state.conflict) {
            this.status = "Sync needs your attention";
            this.emit();
            return;
          }
          this.status = "Syncing…";
          this.emit();
          const remote = await this.request(
            pending ? "POST" : "GET",
            pending ?? undefined,
          );
          if (this.stopped) return;
          await this.lock(async () => {
            const state = await read(this.owner);
            if (!state) return;
            if (pending && state.pending?.id !== pending.id) return;
            if (!pending && state.pending) return;
            const base =
              pending && !remote.conflict ? pending.data : state.base.data;
            const result = merge(base, state.data, remote.data);
            let valid = true;
            try {
              validateData(result.data);
            } catch {
              valid = false;
            }
            if (result.conflicts.length || !valid || remote.conflict) {
              state.base = { ...state.base, data: base };
              state.conflict = { revision: remote.revision, data: remote.data };
              state.pending = null;
              this.status = "Sync needs your attention";
            } else {
              state.data = result.data;
              state.base = { revision: remote.revision, data: remote.data };
              state.pending = null;
              this.status = equal(state.data, state.base.data)
                ? "All changes synced"
                : "Saved on this device · waiting to sync";
            }
            await write(state);
            this.state = state;
            this.updateConflicts();
            this.channel.postMessage("change");
            this.emit();
          });
        },
      );
    } catch (e) {
      this.status = navigator.onLine
        ? message(e)
        : "Offline · changes saved on this device";
      this.emit();
    } finally {
      this.syncing = false;
    }
  }
  get structuralConflict() {
    if (!this.state?.conflict) return false;
    try {
      validateData(
        merge(
          this.state.base.data,
          this.state.data,
          this.state.conflict.data,
          "local",
        ).data,
      );
      validateData(
        merge(
          this.state.base.data,
          this.state.data,
          this.state.conflict.data,
          "remote",
        ).data,
      );
      return false;
    } catch {
      return true;
    }
  }
  async resolve(choice: "local" | "remote") {
    await this.lock(async () => {
      const state = await read(this.owner);
      if (!state?.conflict) return;
      const result = merge(
        state.base.data,
        state.data,
        state.conflict.data,
        choice,
      );
      try {
        validateData(
          merge(state.base.data, state.data, state.conflict.data, "local").data,
        );
        validateData(
          merge(state.base.data, state.data, state.conflict.data, "remote")
            .data,
        );
      } catch {
        result.data = choice === "local" ? state.data : state.conflict.data;
      }
      validateData(result.data);
      state.data = result.data;
      state.base = state.conflict;
      state.conflict = null;
      state.pending = null;
      await write(state);
      this.state = state;
      this.updateConflicts();
      this.emit();
      this.channel.postMessage("change");
    });
    void this.sync();
  }
  async export() {
    const state = await read(this.owner);
    if (!state) return;
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(state.data, null, 2)], {
        type: "application/json",
      }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = "catdo-tasks.json";
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
}
export const message = (error: unknown) =>
  error instanceof Error
    ? error.message
    : "Something went wrong. Your saved tasks are still here.";
