import handler from "@tanstack/react-start/server-entry";
import { androidDownload } from "./server/download";
export { CatDoAccount } from "./server/account";
export default {
  async fetch(request: Request) {
    const path = new URL(request.url).pathname;
    let response: Response;
    try {
      response =
        path === "/download/android"
          ? await androidDownload(request)
          : await handler.fetch(request);
    } catch (error) {
      // Never expose exception messages, URLs, credentials, or task data.
      console.error(
        "CatDo handler failed",
        error instanceof Error ? error.name : "UnknownError",
      );
      response = new Response(
        "CatDo is temporarily unavailable. Please try again.",
        {
          status: 503,
        },
      );
    }
    const headers = new Headers(response.headers);
    if (response.status === 404) headers.set("X-Robots-Tag", "noindex");
    headers.set("X-Content-Type-Options", "nosniff");
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin");
    headers.set("X-Frame-Options", "DENY");
    headers.set(
      "Content-Security-Policy",
      "base-uri 'self'; object-src 'none'; frame-ancestors 'none'",
    );
    headers.set(
      "Permissions-Policy",
      "camera=(), microphone=(), geolocation=()",
    );
    if (response.status === 503 && !headers.has("Retry-After"))
      headers.set("Retry-After", "30");
    if (
      path === "/app" ||
      path.startsWith("/app/") ||
      path === "/privacy-requests" ||
      path.startsWith("/api/")
    ) {
      headers.set("Cache-Control", "no-store");
      headers.set("X-Robots-Tag", "noindex, nofollow");
    }
    if (
      response.status >= 400 ||
      headers.has("set-cookie") ||
      request.headers.has("authorization") ||
      request.headers.has("cookie")
    ) {
      headers.set("Cache-Control", "no-store");
    } else if (
      !headers.has("Cache-Control") &&
      request.method === "GET" &&
      response.status === 200
    ) {
      headers.set("Cache-Control", "public, max-age=0, s-maxage=300");
    }
    return new Response(request.method === "HEAD" ? null : response.body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    });
  },
};
