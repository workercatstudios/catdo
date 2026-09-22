import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/changelog")({
  beforeLoad: () => {
    throw redirect({
      href: "https://github.com/workercatstudios/catdo/releases",
      statusCode: 301,
    });
  },
});
