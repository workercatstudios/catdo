import { z } from "zod";
import type { Env } from "./index";
import { readSyncJson } from "./request-body";
import { privacyInboxName, privacyActionSchema } from "./privacy";

/** Separate operator secret; user sessions can never grant inbox access. */
async function authorized(request: Request, secret: string | undefined) {
  if (!secret || secret.length < 32 || request.headers.has("origin"))
    return false;
  const supplied = request.headers.get("authorization") ?? "";
  if (supplied.length > 1024) return false;
  const encoder = new TextEncoder();
  const [actual, expected] = await Promise.all([
    crypto.subtle.digest("SHA-256", encoder.encode(supplied)),
    crypto.subtle.digest("SHA-256", encoder.encode(`Bearer ${secret}`)),
  ]);
  const a = new Uint8Array(actual),
    b = new Uint8Array(expected);
  let difference = 0;
  for (let i = 0; i < a.length; i++) difference |= a[i] ^ b[i];
  return difference === 0;
}

export async function privacyAdmin(request: Request, env: Env) {
  if (!(await authorized(request, env.PRIVACY_ADMIN_TOKEN)))
    return { status: 401, body: { error: "Operator authorization required." } };
  const inbox = env.ACCOUNTS.getByName(privacyInboxName);
  if (request.method === "GET") {
    const url = new URL(request.url);
    const createdAt = url.searchParams.get("createdAt");
    const id = url.searchParams.get("id");
    const parsed = z
      .strictObject({ createdAt: z.iso.datetime(), id: z.uuid() })
      .safeParse({ createdAt, id });
    if ((createdAt !== null || id !== null) && !parsed.success)
      return { status: 400, body: { error: "Invalid inbox cursor." } };
    return {
      status: 200,
      body: await inbox.pendingPrivacyRequests(
        parsed.success ? parsed.data : undefined,
      ),
    };
  }
  const parsed = privacyActionSchema.safeParse(await readSyncJson(request));
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid privacy response." } };
  if ("action" in parsed.data) {
    const item = await inbox.privacyRequestForOperator(parsed.data.id);
    if (!item) return { status: 404, body: { error: "Request not found." } };
    if (item.status !== "pending")
      return {
        status: 409,
        body: { error: "This request is already resolved." },
      };
    const expectedKind =
      parsed.data.action === "export" ? "access" : "deletion";
    if (item.kind !== expectedKind)
      return {
        status: 409,
        body: { error: "The request does not authorize this action." },
      };
    const account = env.ACCOUNTS.getByName(item.userId);
    if (parsed.data.action === "export") {
      const data = await account.exportCloudData();
      const requests = await inbox.privacyRequests(item.userId);
      return {
        status: 200,
        body: { userId: item.userId, ...data, ...requests },
      };
    }
    return { status: 200, body: await account.eraseCloudData() };
  }
  return inbox.resolvePrivacyRequest(parsed.data);
}
