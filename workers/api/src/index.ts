import { createClerkClient } from "@clerk/backend";
import { pendingSchema } from "./sync";
import { CatDoAccount } from "./account";
export { CatDoAccount };
export interface Env {
  ACCOUNTS: DurableObjectNamespace<CatDoAccount>;
  ASSETS: Fetcher;
  CLERK_SECRET_KEY: string;
  CLERK_PUBLISHABLE_KEY: string;
  CLERK_ISSUER: string;
  CLERK_DESKTOP_CLIENT_ID: string;
  APP_ORIGIN: string;
}
const json = (body: unknown, status = 200) =>
  Response.json(body, {
    status,
    headers: {
      "Cache-Control": "no-store",
      "X-Content-Type-Options": "nosniff",
    },
  });
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) return env.ASSETS.fetch(request);
    if (url.pathname === "/api/config" && request.method === "GET")
      return json({
        publishableKey: env.CLERK_PUBLISHABLE_KEY ?? "",
        issuer: env.CLERK_ISSUER ?? "",
        desktopClientId: env.CLERK_DESKTOP_CLIENT_ID ?? "",
      });
    if (!["/api/sync", "/api/me"].includes(url.pathname))
      return json({ error: "Not found" }, 404);
    if (!["GET", "POST"].includes(request.method))
      return json({ error: "Method not allowed" }, 405);
    if (!env.CLERK_SECRET_KEY || !env.CLERK_PUBLISHABLE_KEY)
      return json({ error: "Sign-in is being configured." }, 503);
    // API auth is bearer-only. A cross-site form/cookie cannot mutate task data.
    if (!request.headers.get("authorization")?.startsWith("Bearer "))
      return json({ error: "Sign in to sync." }, 401);
    const origin = request.headers.get("origin");
    const allowed = [
      env.APP_ORIGIN,
      ...(new URL(env.APP_ORIGIN).hostname === "127.0.0.1"
        ? ["http://127.0.0.1:5173"]
        : []),
    ];
    if (origin && !allowed.includes(origin)) {
      return json({ error: "Origin is not allowed." }, 403);
    }
    try {
      const clerk = createClerkClient({
        secretKey: env.CLERK_SECRET_KEY,
        publishableKey: env.CLERK_PUBLISHABLE_KEY,
      });
      // OAuth access JWTs have client_id, not a browser azp claim. Verify the
      // two token kinds separately so sessions still require an allowed origin.
      const oauth = (
        await clerk.authenticateRequest(request, {
          acceptsToken: "oauth_token",
        })
      ).toAuth();
      const auth = oauth?.isAuthenticated
        ? oauth
        : (
            await clerk.authenticateRequest(request, {
              acceptsToken: "session_token",
              authorizedParties: allowed,
            })
          ).toAuth();
      if (!auth?.isAuthenticated || !("userId" in auth) || !auth.userId)
        return json({ error: "Sign in again to sync." }, 401);
      if (
        auth.tokenType === "oauth_token" &&
        (!env.CLERK_DESKTOP_CLIENT_ID ||
          auth.clientId !== env.CLERK_DESKTOP_CLIENT_ID ||
          !auth.scopes.includes("openid"))
      )
        return json({ error: "This client cannot sync CatDo." }, 403);
      const user = auth.userId;
      if (url.pathname === "/api/me")
        return request.method === "GET"
          ? json({ userId: user })
          : json({ error: "Method not allowed" }, 405);
      if (request.method === "GET")
        return json(await env.ACCOUNTS.getByName(user).snapshot());
      if (!request.headers.get("content-type")?.startsWith("application/json"))
        return json({ error: "Expected JSON" }, 415);
      // Bound the actual stream, including requests without Content-Length.
      const reader = request.body?.getReader();
      if (!reader) return json({ error: "Missing changes" }, 400);
      const chunks: Uint8Array[] = [];
      let length = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        length += value.byteLength;
        if (length > 1900000) {
          await reader.cancel();
          return json({ error: "Changes are too large to sync." }, 413);
        }
        chunks.push(value);
      }
      const bytes = new Uint8Array(length);
      let offset = 0;
      for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.length;
      }
      let pending;
      try {
        pending = pendingSchema.parse(
          JSON.parse(new TextDecoder().decode(bytes)),
        );
      } catch {
        return json({ error: "Invalid sync request." }, 400);
      }
      const result = await env.ACCOUNTS.getByName(user).push(pending);
      return json(result.body, result.status);
    } catch (error) {
      // Never return Clerk diagnostics or request payloads containing credentials/tasks.
      if (
        error instanceof Error &&
        (error.name === "ZodError" || error.message.startsWith("Keep at least"))
      )
        return json({ error: "Invalid task data." }, 400);
      console.error(
        "CatDo request failed",
        error instanceof Error ? error.name : "UnknownError",
      );
      return json(
        {
          error:
            "Sync is temporarily unavailable. Your changes remain on this device.",
        },
        503,
      );
    }
  },
};
