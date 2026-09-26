import { z } from "zod";
import type { AccountStorage } from "./sync";

export const privacyInboxName = "privacy:inbox";
export const privacyRequestSchema = z.strictObject({
  kind: z.enum(["access", "correction", "deletion", "other"]),
  message: z.string().trim().max(2000).optional(),
});
export const privacyResolutionSchema = z.strictObject({
  id: z.uuid(),
  status: z.literal("resolved"),
  response: z.string().trim().min(1).max(2000),
});
export const privacyActionSchema = z.union([
  privacyResolutionSchema,
  z.strictObject({ id: z.uuid(), action: z.literal("export") }),
  z.strictObject({
    id: z.uuid(),
    action: z.literal("erase-cloud-data"),
    confirm: z.literal("erase-cloud-data"),
  }),
]);
export type PrivacyRequestInput = z.infer<typeof privacyRequestSchema>;
export type PrivacyResolution = z.infer<typeof privacyResolutionSchema>;
export type PrivacyRequest = {
  id: string;
  kind: PrivacyRequestInput["kind"];
  message: string;
  status: "pending" | "resolved";
  createdAt: string;
  updatedAt: string;
  response?: string;
};
type Row = Record<string, string | number | null | ArrayBuffer>;
function fromRow(row: Row): PrivacyRequest {
  return {
    id: String(row.id),
    kind: row.kind as PrivacyRequest["kind"],
    message: String(row.message),
    status: row.status as PrivacyRequest["status"],
    createdAt: String(row.created_at),
    updatedAt: String(row.updated_at),
    ...(row.response ? { response: String(row.response) } : {}),
  };
}

/** One private inbox, addressed only by server-verified account identities. */
export class PrivacyInbox {
  constructor(private storage: AccountStorage) {
    storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS privacy_requests (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, kind TEXT NOT NULL, message TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, response TEXT)",
    );
    storage.sql.exec(
      "CREATE INDEX IF NOT EXISTS privacy_user ON privacy_requests(user_id,created_at)",
    );
    storage.sql.exec(
      "CREATE INDEX IF NOT EXISTS privacy_pending ON privacy_requests(status,created_at,id)",
    );
  }
  cleanup() {
    this.storage.sql.exec(
      "DELETE FROM privacy_requests WHERE status='resolved' AND updated_at <= ?",
      new Date(Date.now() - 90 * 24 * 60 * 60 * 1000).toISOString(),
    );
  }
  nextCleanup() {
    const [row] = this.storage.sql.exec(
      "SELECT MIN(updated_at) AS oldest FROM privacy_requests WHERE status='resolved'",
    );
    return row?.oldest
      ? Date.parse(String(row.oldest)) + 90 * 24 * 60 * 60 * 1000
      : null;
  }
  private pruneUser(userId: string) {
    this.storage.sql.exec(
      "DELETE FROM privacy_requests WHERE user_id=? AND status='resolved' AND id NOT IN (SELECT id FROM privacy_requests WHERE user_id=? AND status='resolved' ORDER BY updated_at DESC,id DESC LIMIT 20)",
      userId,
      userId,
    );
  }
  list(userId: string) {
    this.cleanup();
    return {
      requests: [
        ...this.storage.sql.exec(
          "SELECT * FROM privacy_requests WHERE user_id=? ORDER BY created_at DESC,id DESC LIMIT 24",
          userId,
        ),
      ].map(fromRow),
    };
  }
  submit(userId: string, input: PrivacyRequestInput) {
    const parsed = privacyRequestSchema.safeParse(input);
    if (!parsed.success)
      return { status: 400, body: { error: "Invalid privacy request." } };
    return this.storage.transactionSync(() => {
      this.cleanup();
      const [existing] = this.storage.sql.exec(
        "SELECT * FROM privacy_requests WHERE user_id=? AND kind=? AND status='pending'",
        userId,
        parsed.data.kind,
      );
      if (existing)
        return { status: 200, body: { request: fromRow(existing) } };
      const [count] = this.storage.sql.exec(
        "SELECT COUNT(*) AS total FROM privacy_requests WHERE status='pending'",
      );
      if (Number(count.total) >= 10000)
        return {
          status: 503,
          body: {
            error:
              "The privacy inbox is temporarily full. Please try again later.",
          },
        };
      // Keep at most 20 resolved requests plus one pending request per kind.
      this.pruneUser(userId);
      const now = new Date().toISOString();
      const request: PrivacyRequest = {
        id: crypto.randomUUID(),
        kind: parsed.data.kind,
        message: parsed.data.message ?? "",
        status: "pending",
        createdAt: now,
        updatedAt: now,
      };
      this.storage.sql.exec(
        "INSERT INTO privacy_requests(id,user_id,kind,message,status,created_at,updated_at) VALUES (?,?,?,?,'pending',?,?)",
        request.id,
        userId,
        request.kind,
        request.message,
        now,
        now,
      );
      return { status: 201, body: { request } };
    });
  }
  pending(cursor?: { createdAt: string; id: string }) {
    this.cleanup();
    const rows = [
      ...this.storage.sql.exec(
        "SELECT * FROM privacy_requests WHERE status='pending' AND (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at,id LIMIT 101",
        cursor?.createdAt ?? "",
        cursor?.createdAt ?? "",
        cursor?.id ?? "",
      ),
    ];
    const page = rows.slice(0, 100);
    const last = page.at(-1);
    return {
      requests: page.map((row) => ({
        ...fromRow(row),
        userId: String(row.user_id),
      })),
      nextCursor:
        rows.length > 100 && last
          ? { createdAt: String(last.created_at), id: String(last.id) }
          : null,
    };
  }
  requestForOperator(id: string) {
    const [row] = this.storage.sql.exec(
      "SELECT * FROM privacy_requests WHERE id=?",
      id,
    );
    return row ? { ...fromRow(row), userId: String(row.user_id) } : null;
  }
  resolve(input: PrivacyResolution) {
    const parsed = privacyResolutionSchema.safeParse(input);
    if (!parsed.success)
      return { status: 400, body: { error: "Invalid privacy response." } };
    return this.storage.transactionSync(() => {
      const [row] = this.storage.sql.exec(
        "SELECT * FROM privacy_requests WHERE id=?",
        parsed.data.id,
      );
      if (!row) return { status: 404, body: { error: "Request not found." } };
      // Resolving a request is an idempotent record; it never deletes task data.
      if (row.status === "resolved")
        return { status: 200, body: { request: fromRow(row) } };
      const now = new Date().toISOString();
      this.storage.sql.exec(
        "UPDATE privacy_requests SET status='resolved',response=?,updated_at=? WHERE id=?",
        parsed.data.response,
        now,
        parsed.data.id,
      );
      this.pruneUser(String(row.user_id));
      return {
        status: 200,
        body: {
          request: fromRow({
            ...row,
            status: "resolved",
            response: parsed.data.response,
            updated_at: now,
          }),
        },
      };
    });
  }
}
