import { createFileRoute } from "@tanstack/react-router";
import { Home } from "../site/home";
import { seo } from "../lib/seo";
export const Route = createFileRoute("/")({
  head: () => ({
    ...seo(
      "/",
      "CatDo · A calm task manager for web, Android, and Linux",
      "Organize tasks, projects, and everyday life with CatDo. Separate workspaces, due dates, recurring tasks, a calendar, and offline access. Free to use.",
    ),
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify({
          "@context": "https://schema.org",
          "@type": "SoftwareApplication",
          name: "CatDo",
          url: "https://catdo.workercat.com",
          applicationCategory: "ProductivityApplication",
          operatingSystem: "Android, Linux, Web",
          offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
          description:
            "Organize tasks, projects, and everyday life with CatDo. Separate workspaces, due dates, recurring tasks, a calendar, and offline access. Free to use.",
        }),
      },
    ],
  }),
  component: Home,
});
