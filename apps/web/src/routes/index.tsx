import { createFileRoute } from "@tanstack/react-router";
import { Home } from "../site/home";
import { origin, seo } from "../lib/seo";
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
          "@graph": [
            {
              "@type": "WebSite",
              "@id": origin + "/#website",
              name: "CatDo",
              url: origin + "/",
              inLanguage: "en",
            },
            {
              "@type": "SoftwareApplication",
              "@id": origin + "/#application",
              name: "CatDo",
              url: origin + "/",
              image: origin + "/social.png",
              applicationCategory: "ProductivityApplication",
              operatingSystem: "Android, Linux, Web",
              offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
              description:
                "Organize tasks, projects, and everyday life with CatDo. Separate workspaces, due dates, recurring tasks, a calendar, and offline access. Free to use.",
            },
          ],
        }),
      },
    ],
  }),
  component: Home,
});
