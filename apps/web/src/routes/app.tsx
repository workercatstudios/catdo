import { createFileRoute } from "@tanstack/react-router";
import { AuthApp } from "../auth";
import { seo } from "../lib/seo";
export const Route = createFileRoute("/app")({
  ssr: false,
  head: () =>
    seo("/app", "Your tasks · CatDo", "Your personal task space.", true),
  component: AuthApp,
});
