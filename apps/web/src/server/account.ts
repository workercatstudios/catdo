import { DurableObject } from "cloudflare:workers";
import { AccountStore } from "./sync";
import type { Pending } from "../../../../packages/domain/src/sync";
export class CatDoAccount extends DurableObject<Record<string, never>> {
  private store: AccountStore;
  constructor(ctx: DurableObjectState, env: Record<string, never>) {
    super(ctx, env);
    this.store = new AccountStore(ctx.storage);
  }
  snapshot() {
    return this.store.snapshot();
  }
  push(pending: Pending) {
    return this.store.push(pending);
  }
}
