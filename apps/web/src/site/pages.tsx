import { Link } from "@tanstack/react-router";
import { ArrowRight } from "lucide-react";
import { SiteLayout, PageIntro } from "./layout";
import { guides } from "./content";

export function HelpArticle({ slug }: { slug: string }) {
  const guide = guides.find((g) => g.slug === slug)!;
  return (
    <SiteLayout>
      <main id="main" className="public-page article">
        <a href="/#help" className="back-link">
          ← Back to CatDo
        </a>
        <PageIntro eyebrow="CatDo guide" title={guide.title}>
          {guide.description}
        </PageIntro>
        <div className="article-body">
          {guide.sections.map(([title, text], index) => (
            <details key={title} open={index === 0}>
              <summary>
                <h2>{title}</h2>
              </summary>
              <p>{text}</p>
            </details>
          ))}
        </div>
        <Link to="/app" className="inline-link">
          Back to your tasks <ArrowRight size={17} />
        </Link>
      </main>
    </SiteLayout>
  );
}
