import { createFileRoute } from "@tanstack/react-router";
import { seo } from "../lib/seo";
import { PrivacyPage } from "../site/privacy";

export const Route = createFileRoute("/privacy")({
  head: () =>
    seo(
      "/privacy",
      "Privacy & your data · CatDo",
      "How CatDo stores your tasks, uses sign-in and hosting providers, and handles browser storage, Android diagnostics, and data export.",
    ),
  component: PrivacyPage,
});
