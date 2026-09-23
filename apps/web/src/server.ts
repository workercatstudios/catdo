import handler from "@tanstack/react-start/server-entry";
import { androidDownload } from "./server/download";
export { CatDoAccount } from "./server/account";
export default {
  async fetch(request: Request) {
    const path = new URL(request.url).pathname;
    if (path === "/download/android") return androidDownload(request);
    const response = await handler.fetch(request);
    const headers = new Headers(response.headers);
    if (response.status === 404) headers.set("X-Robots-Tag", "noindex");
    headers.set("X-Content-Type-Options", "nosniff");
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin");
    headers.set("X-Frame-Options", "DENY");
    if (
      path === "/app" ||
      path.startsWith("/app/") ||
      path.startsWith("/api/")
    ) {
      headers.set("Cache-Control", "no-store");
      headers.set("X-Robots-Tag", "noindex, nofollow");
    } else if (
      request.method === "GET" &&
      response.status === 200 &&
      !headers.has("set-cookie")
    ) {
      headers.set("Cache-Control", "public, max-age=0, s-maxage=300");
    }
    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    });
  },
};
