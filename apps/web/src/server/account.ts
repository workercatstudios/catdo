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
  }
  snapshot() {
    return this.store.snapshot();
  }
  push(pending: Pending) {
    return this.store.push(pending);
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
    return this.store.eraseCloudData();
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
