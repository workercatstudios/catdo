import { Link } from "@tanstack/react-router";
import { ArrowRight } from "lucide-react";
import { SiteLayout, PageIntro } from "./layout";
import { guides } from "./content";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "../components/ui/pop-accordion";

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
        <Accordion
          className="article-body"
          multiple
          hiddenUntilFound
          defaultValue={[guide.sections[0][0]]}
        >
          {guide.sections.map(([title, text]) => (
            <AccordionItem key={title} value={title}>
              <AccordionTrigger>
                <h2>{title}</h2>
              </AccordionTrigger>
              <AccordionContent>
                <p>{text}</p>
              </AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>
        <Link to="/app" className="inline-link">
          Back to your tasks <ArrowRight size={17} />
        </Link>
      </main>
    </SiteLayout>
  );
}
