import { DurableObject } from "cloudflare:workers";
import { AccountStore } from "./sync";
import type { Pending } from "../../../../packages/domain/src/sync";
import {
  PrivacyInbox,
  type PrivacyRequestInput,
  type PrivacyResolution,
} from "./privacy";
export class CatDoAccount extends DurableObject<Record<string, never>> {
  private store: AccountStore;
  private inbox: PrivacyInbox | undefined;
  constructor(ctx: DurableObjectState, env: Record<string, never>) {
    super(ctx, env);
    this.store = new AccountStore(ctx.storage);
    ctx.storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS event_tickets (ticket TEXT PRIMARY KEY, expires_at INTEGER NOT NULL)",
    );
  }
  snapshot() {
    return this.store.snapshot();
  }
  push(pending: Pending) {
    const before = this.store.snapshot().revision;
    const result = this.store.push(pending);
    const revision = "revision" in result.body ? result.body.revision : before;
    if (revision > before) {
      for (const socket of this.ctx.getWebSockets()) {
        try {
          socket.send(JSON.stringify({ revision }));
        } catch {
          socket.close();
        }
      }
    }
    return result;
  }
  issueEventTicket() {
    const ticket = crypto.randomUUID();
    const now = Date.now();
    this.ctx.storage.sql.exec(
      "DELETE FROM event_tickets WHERE expires_at < ?",
      now,
    );
    this.ctx.storage.sql.exec(
      "INSERT INTO event_tickets(ticket,expires_at) VALUES (?,?)",
      ticket,
      now + 30_000,
    );
    return { ticket };
  }
  async fetch(request: Request) {
    if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket")
      return new Response(null, { status: 426 });
    const ticket = new URL(request.url).searchParams.get("ticket");
    if (!ticket) return new Response(null, { status: 401 });
    const valid = this.ctx.storage.transactionSync(() => {
      const [row] = this.ctx.storage.sql.exec(
        "SELECT expires_at FROM event_tickets WHERE ticket=?",
        ticket,
      );
      if (!row) return false;
      this.ctx.storage.sql.exec(
        "DELETE FROM event_tickets WHERE ticket=?",
        ticket,
      );
      return Number(row.expires_at) >= Date.now();
    });
    if (!valid) return new Response(null, { status: 401 });
    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);
    this.ctx.acceptWebSocket(server);
    server.send(JSON.stringify({ revision: this.store.snapshot().revision }));
    return new Response(null, { status: 101, webSocket: client });
  }
  webSocketMessage(socket: WebSocket) {
    socket.close(1003, "Messages are not supported");
  }
  legal() {
    return this.store.legal();
  }
  acceptLegal(input: unknown) {
    return this.store.acceptLegal(input);
  }
  exportCloudData() {
    return this.store.exportCloudData();
  }
  eraseCloudData() {
    const result = this.store.eraseCloudData();
    for (const socket of this.ctx.getWebSockets()) {
      try {
        socket.send(JSON.stringify({ revision: 0 }));
      } catch {
        socket.close();
      }
    }
    return result;
  }
  private privacyInbox() {
    return (this.inbox ??= new PrivacyInbox(this.ctx.storage));
  }
  privacyRequests(userId: string) {
    return this.privacyInbox().list(userId);
  }
  submitPrivacyRequest(userId: string, input: PrivacyRequestInput) {
    return this.privacyInbox().submit(userId, input);
  }
  pendingPrivacyRequests(cursor?: { createdAt: string; id: string }) {
    return this.privacyInbox().pending(cursor);
  }
  async resolvePrivacyRequest(input: PrivacyResolution) {
    const result = this.privacyInbox().resolve(input);
    const cleanupAt = this.privacyInbox().nextCleanup();
    if (cleanupAt !== null) await this.ctx.storage.setAlarm(cleanupAt);
    return result;
  }
  privacyRequestForOperator(id: string) {
    return this.privacyInbox().requestForOperator(id);
  }
  async alarm() {
    this.privacyInbox().cleanup();
    const next = this.privacyInbox().nextCleanup();
    if (next !== null) await this.ctx.storage.setAlarm(next);
  }
}
