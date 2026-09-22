import { createFileRoute, notFound } from "@tanstack/react-router";
import { guides } from "../site/content";
import { HelpArticle } from "../site/pages";
import { seo } from "../lib/seo";
export const Route = createFileRoute("/help/$slug")({
  loader: ({ params }) => {
    const guide = guides.find((g) => g.slug === params.slug);
    if (!guide) throw notFound();
    return guide;
  },
  head: ({ loaderData: g }) =>
    g
      ? seo("/help/" + g.slug, g.title + " · CatDo help", g.description)
      : { meta: [{ name: "robots", content: "noindex" }] },
  component: Article,
});
function Article() {
  const { slug } = Route.useParams();
  return <HelpArticle slug={slug} />;
}
