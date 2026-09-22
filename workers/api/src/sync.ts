import {
  emptyData,
  validateData,
  dataSchema,
} from "../../../packages/domain/src/model";
import {
  merge,
  type Pending,
  type Snapshot,
} from "../../../packages/domain/src/sync";
import { z } from "zod";
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
    const pending = pendingSchema.parse(input);
    validateData(pending.base, true);
    validateData(pending.data);
    return this.storage.transactionSync(() => {
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
      const result = merge(pending.base, pending.data, current.data);
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
