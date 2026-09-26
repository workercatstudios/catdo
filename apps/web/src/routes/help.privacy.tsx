import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/help/privacy")({
  beforeLoad: () => {
    throw redirect({ to: "/privacy", statusCode: 301 });
  },
});
