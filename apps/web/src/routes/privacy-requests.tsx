import { createFileRoute } from "@tanstack/react-router";
import { AuthApp } from "../auth";
import { seo } from "../lib/seo";
export const Route = createFileRoute("/privacy-requests")({
  ssr: false,
  head: () =>
    seo(
      "/privacy-requests",
      "Privacy requests · CatDo",
      "Manage private requests about your CatDo account.",
      true,
    ),
  component: () => <AuthApp privacyRequests />,
});
