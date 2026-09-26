import {
  emptyData,
  validateData,
  dataSchema,
  type Data,
} from "../../../../packages/domain/src/model";
import {
  merge,
  type Pending,
  type Snapshot,
} from "../../../../packages/domain/src/sync";
import { z } from "zod";
import { acceptanceSchema, legalPolicy } from "./legal";
export const pendingSchema = z.object({
  id: z.uuid(),
  base: dataSchema,
  data: dataSchema,
});
type SqlValue = string | number | null | ArrayBuffer;
export interface AccountStorage {
  sql: {
    exec(
      query: string,
      ...values: SqlValue[]
    ): Iterable<Record<string, SqlValue>>;
  };
  transactionSync<T>(callback: () => T): T;
}
/** The local starter workspace gets a new ID on every device. On a first
 * upload, join its contents to the account's existing Personal workspace. */
function joinStarterWorkspace(pending: Pending, remote: Data): Data {
  if (pending.base.workspaces.length) return pending.data;
  const local = pending.data.workspaces.filter(
    (w) => w.name === "Personal" && !w.archived,
  );
  const existing = remote.workspaces.filter(
    (w) => w.name === "Personal" && !w.archived,
  );
  if (local.length !== 1 || existing.length !== 1) return pending.data;
  const from = local[0].id;
  const to = existing[0].id;
  if (from === to) return pending.data;
  return {
    ...pending.data,
    workspaces: pending.data.workspaces.map((w) =>
      w.id === from ? { ...w, id: to } : w,
    ),
    projects: pending.data.projects.map((p) =>
      p.workspace_id === from ? { ...p, workspace_id: to } : p,
    ),
    tasks: pending.data.tasks.map((t) =>
      t.workspace_id === from ? { ...t, workspace_id: to } : t,
    ),
    history: pending.data.history.map((h) => ({
      ...h,
      task:
        h.task.workspace_id === from ? { ...h.task, workspace_id: to } : h.task,
    })),
  };
}
/** Each instance belongs to one authenticated Clerk user. Transactions contain
 * both the new snapshot and its receipt, so a lost response is safe to retry. */
export class AccountStore {
  constructor(private storage: AccountStorage) {
    storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS account (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL, body TEXT NOT NULL)",
    );
    storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS receipts (id TEXT PRIMARY KEY, revision INTEGER NOT NULL)",
    );
    storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS legal_acceptances (version TEXT PRIMARY KEY, accepted_at TEXT NOT NULL, terms_url TEXT NOT NULL, privacy_url TEXT NOT NULL, minimum_age INTEGER NOT NULL)",
    );
    storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS account_closure (id INTEGER PRIMARY KEY CHECK(id=1), closed_at TEXT NOT NULL)",
    );
  }
  exportCloudData() {
    return {
      cloud: this.snapshot(),
      acceptances: [
        ...this.storage.sql.exec(
          "SELECT version,accepted_at,terms_url,privacy_url,minimum_age FROM legal_acceptances ORDER BY accepted_at",
        ),
      ].map((row) => ({
        version: String(row.version),
        acceptedAt: String(row.accepted_at),
        termsUrl: String(row.terms_url),
        privacyUrl: String(row.privacy_url),
        minimumAge: Number(row.minimum_age),
      })),
    };
  }
  eraseCloudData() {
    return this.storage.transactionSync(() => {
      this.storage.sql.exec("DELETE FROM account");
      this.storage.sql.exec("DELETE FROM receipts");
      this.storage.sql.exec(
        "INSERT OR IGNORE INTO account_closure(id,closed_at) VALUES (1,?)",
        new Date().toISOString(),
      );
      return { erased: true, cloudDisabled: true };
    });
  }
  legal() {
    const [row] = this.storage.sql.exec(
      "SELECT accepted_at FROM legal_acceptances WHERE version=?",
      legalPolicy.version,
    );
    return {
      ...legalPolicy,
      accepted: Boolean(row),
      ...(row ? { acceptedAt: String(row.accepted_at) } : {}),
    };
  }
  acceptLegal(input: unknown) {
    if (!acceptanceSchema.safeParse(input).success)
      return {
        status: 400,
        body: { error: "Confirm the current terms and age requirement." },
      };
    return this.storage.transactionSync(() => {
      this.storage.sql.exec(
        "INSERT OR IGNORE INTO legal_acceptances(version,accepted_at,terms_url,privacy_url,minimum_age) VALUES (?,?,?,?,?)",
        legalPolicy.version,
        new Date().toISOString(),
        legalPolicy.termsUrl,
        legalPolicy.privacyUrl,
        legalPolicy.minimumAge,
      );
      return { status: 200, body: this.legal() };
    });
  }
  snapshot(): Snapshot {
    const [row] = this.storage.sql.exec(
      "SELECT revision,body FROM account WHERE id=1",
    );
    return row
      ? { revision: Number(row.revision), data: JSON.parse(String(row.body)) }
      : { revision: 0, data: emptyData() };
  }
  push(input: Pending) {
    let pending: Pending;
    try {
      pending = pendingSchema.parse(input);
      validateData(pending.base, true);
      validateData(pending.data);
    } catch {
      return { status: 400, body: { error: "Invalid task data." } };
    }
    return this.storage.transactionSync(() => {
      if (
        [...this.storage.sql.exec("SELECT id FROM account_closure WHERE id=1")]
          .length
      )
        return {
          status: 403,
          body: {
            code: "cloud_data_deleted",
            error:
              "Cloud storage for this account was closed after a deletion request. Your local tasks remain on this device. Visit https://catdo.workercat.com/privacy-requests for help.",
          },
        };
      const current = this.snapshot();
      if (
        [
          ...this.storage.sql.exec(
            "SELECT id FROM receipts WHERE id=?",
            pending.id,
          ),
        ].length
      )
        return { status: 200, body: current };
      const result = merge(
        pending.base,
        joinStarterWorkspace(pending, current.data),
        current.data,
      );
      if (result.conflicts.length)
        return {
          status: 409,
          body: { ...current, conflicts: result.conflicts },
        };
      try {
        validateData(result.data);
      } catch {
        return {
          status: 409,
          body: {
            ...current,
            conflicts: [
              {
                collection: "tasks",
                id: "structure",
                name: "A task or project was moved on another device",
              },
            ],
          },
        };
      }
      const body = JSON.stringify(result.data);
      if (new TextEncoder().encode(body).length > 900000)
        return {
          status: 413,
          body: {
            error:
              "This account has reached the current sync size limit. Your local changes are safe; export a backup and contact WorkerCat.",
          },
        };
      const revision = current.revision + 1;
      this.storage.sql.exec(
        "INSERT INTO account(id,revision,body) VALUES (1,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,body=excluded.body",
        revision,
        body,
      );
      this.storage.sql.exec(
        "INSERT INTO receipts(id,revision) VALUES (?,?)",
        pending.id,
        revision,
      );
      return { status: 200, body: { revision, data: result.data } };
    });
  }
}
