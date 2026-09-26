import { createFileRoute } from "@tanstack/react-router";
import { seo } from "../lib/seo";
import { SupportPage } from "../site/support";

export const Route = createFileRoute("/support")({
  head: () =>
    seo(
      "/support",
      "Help & support · CatDo",
      "Get help with CatDo sign-in, offline tasks, syncing, and downloads. Find guides, report a bug, or privately report a security issue.",
    ),
  component: SupportPage,
});
