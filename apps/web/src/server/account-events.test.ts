import { expect, it, vi } from "vitest";

vi.mock("cloudflare:workers", () => ({
  DurableObject: class {
    ctx: unknown;
    constructor(ctx: unknown) {
      this.ctx = ctx;
    }
  },
}));
vi.mock("./sync", () => ({
  AccountStore: class {
    revision = 0;
    seen = new Set<string>();
    snapshot() {
      return { revision: this.revision };
    }
    push(pending: { id: string }) {
      if (!this.seen.has(pending.id)) {
        this.seen.add(pending.id);
        this.revision++;
      }
      return { status: 200, body: this.snapshot() };
    }
  },
}));

import { CatDoAccount } from "./account";

it("notifies connected devices once for a committed revision", () => {
  const send = vi.fn();
  const ctx = {
    storage: { sql: { exec: vi.fn() } },
    getWebSockets: () => [{ send, close: vi.fn() }],
  };
  const account = new CatDoAccount(ctx as never, {});
  account.push({ id: "operation-a" } as never);
  account.push({ id: "operation-a" } as never);
  expect(send).toHaveBeenCalledTimes(1);
  expect(send).toHaveBeenCalledWith(JSON.stringify({ revision: 1 }));
});

it("consumes a short-lived event ticket before accepting a socket", async () => {
  const tickets = new Map<string, number>();
  const storage = {
    sql: {
      exec(query: string, ...values: (string | number)[]) {
        if (query.startsWith("INSERT INTO event_tickets"))
          tickets.set(String(values[0]), Number(values[1]));
        if (query.startsWith("SELECT expires_at"))
          return tickets.has(String(values[0]))
            ? [{ expires_at: tickets.get(String(values[0])) }]
            : [];
        if (query.startsWith("DELETE FROM event_tickets WHERE ticket"))
          tickets.delete(String(values[0]));
        return [];
      },
    },
    transactionSync<T>(fn: () => T) {
      return fn();
    },
  };
  const acceptWebSocket = vi.fn();
  const ctx = { storage, acceptWebSocket };
  const send = vi.fn();
  vi.stubGlobal(
    "WebSocketPair",
    class {
      0 = {};
      1 = { send };
    },
  );
  vi.stubGlobal(
    "Response",
    class {
      status: number;
      constructor(_body: unknown, init: { status: number }) {
        this.status = init.status;
      }
    },
  );
  try {
    const account = new CatDoAccount(ctx as never, {});
    const { ticket } = account.issueEventTicket();
    const request = new Request(
      `https://catdo.example/api/sync/events?ticket=${ticket}`,
      {
        headers: { Upgrade: "websocket" },
      },
    );
    expect((await account.fetch(request)).status).toBe(101);
    expect((await account.fetch(request)).status).toBe(401);
    expect(acceptWebSocket).toHaveBeenCalledTimes(1);
    expect(send).toHaveBeenCalledWith(JSON.stringify({ revision: 0 }));
  } finally {
    vi.unstubAllGlobals();
  }
});
