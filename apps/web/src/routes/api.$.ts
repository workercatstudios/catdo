import { createFileRoute } from "@tanstack/react-router";
import { env } from "cloudflare:workers";
import api, { type Env } from "../server/index";
const handle = ({ request }: { request: Request }) =>
  api.fetch(request, env as unknown as Env);
export const Route = createFileRoute("/api/$")({
  server: {
    handlers: {
      GET: handle,
      POST: handle,
      PUT: handle,
      PATCH: handle,
      DELETE: handle,
      OPTIONS: handle,
      HEAD: handle,
    },
  },
});
