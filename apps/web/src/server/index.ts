import { createClerkClient } from "@clerk/backend";
import { pendingSchema } from "./sync";
import type { CatDoAccount } from "./account";
import { readSyncJson, RequestBodyError } from "./request-body";
import { acceptanceSchema, termsRequired } from "./legal";
import { privacyInboxName, privacyRequestSchema } from "./privacy";
import { privacyAdmin } from "./privacy-admin";
export interface Env {
  ACCOUNTS: DurableObjectNamespace<CatDoAccount>;
  CLERK_SECRET_KEY: string;
  CLERK_PUBLISHABLE_KEY: string;
  CLERK_ISSUER: string;
  CLERK_DESKTOP_CLIENT_ID: string;
  APP_ORIGIN: string;
  PRIVACY_ADMIN_TOKEN?: string;
}
const json = (
  body: unknown,
  status = 200,
  headers: Record<string, string> = {},
) =>
  Response.json(body, {
    status,
    headers: {
      "Cache-Control": "no-store",
      "X-Content-Type-Options": "nosniff",
      ...(status === 503 ? { "Retry-After": "30" } : {}),
      ...headers,
    },
  });
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/"))
      return json({ error: "Not found" }, 404);
    const methods = new Map([
      ["/api/health", ["GET", "HEAD"]],
      ["/api/config", ["GET"]],
      ["/api/me", ["GET"]],
      ["/api/sync", ["GET", "POST"]],
      ["/api/legal", ["GET", "POST"]],
      ["/api/privacy-requests", ["GET", "POST"]],
      ["/api/admin/privacy-requests", ["GET", "POST"]],
    ]).get(url.pathname);
    if (!methods) return json({ error: "Not found" }, 404);
    if (!methods.includes(request.method))
      return json({ error: "Method not allowed" }, 405, {
        Allow: methods.join(", "),
      });
    // Liveness only: no auth calls, storage allocation, or account information.
    if (url.pathname === "/api/health") {
      const response = json({ status: "ok" });
      return request.method === "HEAD"
        ? new Response(null, {
            status: response.status,
            headers: response.headers,
          })
        : response;
    }
    if (url.pathname === "/api/config")
      return json({
        publishableKey: env.CLERK_PUBLISHABLE_KEY ?? "",
        issuer: env.CLERK_ISSUER ?? "",
        desktopClientId: env.CLERK_DESKTOP_CLIENT_ID ?? "",
      });
    if (url.pathname === "/api/admin/privacy-requests") {
      try {
        const result = await privacyAdmin(request, env);
        return json(result.body, result.status);
      } catch (error) {
        if (error instanceof RequestBodyError)
          return json({ error: error.message }, error.status);
        console.error("CatDo privacy inbox unavailable");
        return json(
          { error: "The privacy inbox is temporarily unavailable." },
          503,
        );
      }
    }
    if (!env.CLERK_SECRET_KEY || !env.CLERK_PUBLISHABLE_KEY)
      return json({ error: "Sign-in is being configured." }, 503);
    // API auth is bearer-only. A cross-site form/cookie cannot mutate task data.
    if (!request.headers.get("authorization")?.startsWith("Bearer "))
      return json({ error: "Sign in to sync." }, 401);
    const origin = request.headers.get("origin");
    let appOrigin: URL;
    try {
      appOrigin = new URL(env.APP_ORIGIN);
      if (
        !["http:", "https:"].includes(appOrigin.protocol) ||
        appOrigin.origin !== env.APP_ORIGIN
      )
        throw new Error("Invalid origin");
    } catch {
      console.error("CatDo configuration invalid", { setting: "APP_ORIGIN" });
      return json({ error: "Sign-in is being configured." }, 503);
    }
    const allowed = [
      env.APP_ORIGIN,
      ...(appOrigin.hostname === "127.0.0.1"
        ? ["http://127.0.0.1:5173", "http://127.0.0.1:4173"]
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
      const oauthState = await clerk.authenticateRequest(request, {
        acceptsToken: "oauth_token",
      });
      const oauth = oauthState.toAuth();
      const sessionState = oauth?.isAuthenticated
        ? null
        : await clerk.authenticateRequest(request, {
            acceptsToken: "session_token",
            authorizedParties: allowed,
          });
      let auth = oauth?.isAuthenticated ? oauth : sessionState?.toAuth();
      // Native Clerk calls have no browser Origin or azp claim. They still use
      // a signed bearer token; verify it again without browser-origin pinning,
      // then accept only the claimless native case.
      if (
        !auth?.isAuthenticated &&
        !origin &&
        sessionState?.reason === "token-invalid-authorized-parties"
      ) {
        const nativeState = await clerk.authenticateRequest(request, {
          acceptsToken: "session_token",
        });
        const native = nativeState.toAuth();
        if (
          native?.isAuthenticated &&
          native.tokenType === "session_token" &&
          native.sessionClaims.azp === undefined
        )
          auth = native;
      }
      if (!auth?.isAuthenticated || !("userId" in auth) || !auth.userId) {
        // Reason codes only: never log bearer tokens, session claims, or task data.
        console.warn("CatDo auth denied", {
          oauth: oauthState.reason,
          session: sessionState?.reason,
        });
        return json({ error: "Sign in again to sync." }, 401);
      }
      if (
        auth.tokenType === "oauth_token" &&
        (!env.CLERK_DESKTOP_CLIENT_ID ||
          auth.clientId !== env.CLERK_DESKTOP_CLIENT_ID ||
          !auth.scopes.includes("openid"))
      )
        return json({ error: "This client cannot sync CatDo." }, 403);
      const user = auth.userId;
      if (url.pathname === "/api/me") return json({ userId: user });
      if (url.pathname === "/api/legal") {
        if (request.method === "GET")
          return json(await env.ACCOUNTS.getByName(user).legal());
        const parsed = acceptanceSchema.safeParse(await readSyncJson(request));
        if (!parsed.success)
          return json(
            { error: "Confirm the current terms and age requirement." },
            400,
          );
        const result = await env.ACCOUNTS.getByName(user).acceptLegal(
          parsed.data,
        );
        return json(result.body, result.status);
      }
      if (url.pathname === "/api/privacy-requests") {
        if (request.method === "GET")
          return json(
            await env.ACCOUNTS.getByName(privacyInboxName).privacyRequests(
              user,
            ),
          );
        const parsed = privacyRequestSchema.safeParse(
          await readSyncJson(request),
        );
        if (!parsed.success)
          return json(
            {
              error: "Choose a request type and use at most 2,000 characters.",
            },
            400,
          );
        const result = await env.ACCOUNTS.getByName(
          privacyInboxName,
        ).submitPrivacyRequest(user, parsed.data);
        return json(result.body, result.status);
      }
      if (request.method === "GET")
        return json(await env.ACCOUNTS.getByName(user).snapshot());
      const input = await readSyncJson(request);
      let pending;
      try {
        pending = pendingSchema.parse(input);
      } catch {
        return json({ error: "Invalid sync request." }, 400);
      }
      const account = env.ACCOUNTS.getByName(user);
      if (!(await account.legal()).accepted) return json(termsRequired, 428);
      const result = await account.push(pending);
      return json(result.body, result.status);
    } catch (error) {
      // Never return Clerk diagnostics or request payloads containing credentials/tasks.
      if (error instanceof RequestBodyError)
        return json({ error: error.message }, error.status);
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
