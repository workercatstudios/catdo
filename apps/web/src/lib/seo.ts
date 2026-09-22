export const origin = "https://catdo.workercat.com";
export function seo(
  path: string,
  title: string,
  description: string,
  noindex = false,
) {
  return {
    meta: [
      // Public ownership tag for the site's Search Console property.
      ...(path === "/"
        ? [
            {
              name: "google-site-verification",
              content: "zqaJhx9MQyE3FRg17xkbXhAV3aZ1-IGMv9jBFJwaR34",
            },
          ]
        : []),
      { title },
      { name: "description", content: description },
      { property: "og:title", content: title },
      { property: "og:description", content: description },
      { property: "og:type", content: "website" },
      { property: "og:site_name", content: "CatDo" },
      { property: "og:url", content: origin + path },
      { property: "og:image", content: origin + "/social.png" },
      { property: "og:image:width", content: "1200" },
      { property: "og:image:height", content: "630" },
      { property: "og:image:alt", content: "CatDo. A little more organized." },
      { name: "twitter:card", content: "summary_large_image" },
      ...(noindex ? [{ name: "robots", content: "noindex, nofollow" }] : []),
    ],
    links: [{ rel: "canonical", href: origin + path }],
  };
}
