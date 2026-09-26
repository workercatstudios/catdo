import { createFileRoute } from "@tanstack/react-router";
import { origin } from "../lib/seo";
export const Route = createFileRoute("/robots.txt")({
  server: {
    handlers: {
      GET: () =>
        new Response(
          `User-agent: *\nAllow: /\nDisallow: /api/\nSitemap: ${origin}/sitemap.xml\n`,
          { headers: { "Content-Type": "text/plain; charset=utf-8" } },
        ),
    },
  },
});
