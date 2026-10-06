import { Link } from "@tanstack/react-router";
import { ArrowRight, Download } from "lucide-react";
import { SiteLayout } from "./layout";
import { ProductPreview } from "./product-preview";
import { Button } from "../components/ui/pop-button";
import { BounceText } from "../components/ui/bounce-text";
import { IdleCat } from "../components/ui/idle-cat";
import { Sparkles } from "../components/ui/sparkles";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "../components/ui/pop-accordion";
import { release } from "./content";

const questions = [
  {
    q: "Is CatDo free?",
    a: "Yes. All current features are free to use. We may add an optional supporter tier later. The hosted service can be used for personal or workplace tasks. The source is available under the PolyForm Noncommercial license.",
  },
  {
    q: "Where do I start?",
    a: "Put one task in Inbox. Add projects and dates when you need them.",
    link: ["getting-started", "Your first little plan ↗"],
  },
  {
    q: "Can I use it offline?",
    a: "Yes, after signing in and opening your tasks online. Saved tasks stay on your device, and edits sync when you reconnect. If you use “Open saved tasks,” sign in again when you’re back online to resume syncing.",
    link: ["sync-and-offline", "More about sync and offline use ↗"],
  },
  {
    q: "How do dates and recurring tasks work?",
    a: "A scheduled date is when you plan to do it. A due date is its deadline. Recurring tasks can follow a fixed schedule or repeat after you finish.",
    link: ["dates-and-repeats", "Dates, repeats, and reminders ↗"],
  },
  {
    q: "Are there keyboard shortcuts?",
    a: "Ctrl+K to search. Ctrl+Enter for a new task. Enter to save from quick add.",
    link: ["keyboard-shortcuts", "See the shortcuts ↗"],
  },
] as const;

export function Home() {
  return (
    <SiteLayout>
      <main id="main" className="home-page">
        <section className="home-hero" aria-labelledby="hero-title">
          <div className="hero-stage" aria-hidden="true">
            <span className="shape shape-sun" />
            <span className="shape shape-pill" />
            <span className="shape shape-ring" />
            <span className="shape shape-square" />
            <span className="shape shape-star">✦</span>
            <span className="shape shape-star shape-star-2">✦</span>
            <IdleCat className="hero-cat" size={150} mood="idle" />
          </div>
          <div className="hero-heading">
            <p className="eyebrow">
              A personal task manager. Web, Android, Linux & Windows.
            </p>
            <h1 id="hero-title">
              <BounceText text="A little more" trigger="mount" delay={120} />
              <br />
              <Sparkles count={5} size={22} className="hero-sparkles">
                <BounceText
                  text="organized."
                  trigger="mount"
                  delay={120 + 13 * 80}
                />
              </Sparkles>
            </h1>
          </div>
          <div className="hero-copy">
            <p className="hero-intro">
              Get it out of your head.
              <br />
              Make a little room for your day.
            </p>
            <div className="hero-actions">
              <Button size="lg" nativeButton={false} render={<a href="/app" />}>
                Get started <ArrowRight />
              </Button>
              <a href="#download" className="inline-link">
                Get it for Linux <Download size={16} />
              </a>
            </div>
            <p className="availability">Free to use. No subscription needed.</p>
          </div>
          <ProductPreview />
        </section>

        <section
          id="download"
          className="home-section download-section"
          aria-labelledby="download-title"
        >
          <div className="download-heading">
            <img src="/icon.png" width="80" height="80" alt="" loading="lazy" />
            <div>
              <p className="eyebrow">At home on Linux</p>
              <h2 id="download-title">One less browser tab.</h2>
            </div>
          </div>
          <div className="download-copy">
            <p>
              Native, offline, and close at hand. Reminders, a system tray, and
              updates in a click.
            </p>
            <div className="download-actions">
              <Button
                size="lg"
                nativeButton={false}
                render={<a href={release.appImage} />}
              >
                <Download /> Download AppImage
              </Button>
              <a href={release.archive}>Or get the tar.gz ↗</a>
            </div>
            <p className="download-requirements">
              Latest release · Linux x86_64 · glibc 2.39+
            </p>
            <Accordion className="install-details" hiddenUntilFound>
              <AccordionItem value="install">
                <AccordionTrigger>How to install</AccordionTrigger>
                <AccordionContent>
                  <p>
                    Download the AppImage, allow it to run as a program in your
                    file manager’s permissions, then open it. Sign in with your
                    WorkerCat account to sync your tasks.
                  </p>
                  <p>
                    If FUSE is unavailable, launch with{" "}
                    <code>--appimage-extract-and-run</code>. The tar.gz includes
                    an install script and instructions.
                  </p>
                  <a href={release.url}>Release notes and checksums ↗</a>
                </AccordionContent>
              </AccordionItem>
            </Accordion>
            <p className="platform-note">
              On Windows? <a href={release.windows}>Download the app.</a> On
              Android? <a href={release.android}>Download the APK.</a> On
              another device? <a href="/app">Use the web app.</a>
            </p>
          </div>
        </section>

        <section
          id="help"
          className="home-section questions-section"
          aria-labelledby="questions-title"
        >
          <div>
            <p className="eyebrow">A few things to know</p>
            <h2 id="questions-title">Keep it simple.</h2>
            <a className="inline-link" href="/support">
              Need a hand? <ArrowRight size={16} />
            </a>
          </div>
          <Accordion className="questions" hiddenUntilFound>
            {questions.map((item) => (
              <AccordionItem key={item.q} value={item.q}>
                <AccordionTrigger>{item.q}</AccordionTrigger>
                <AccordionContent>
                  <p>{item.a}</p>
                  {"link" in item && (
                    <Link to="/help/$slug" params={{ slug: item.link[0] }}>
                      {item.link[1]}
                    </Link>
                  )}
                </AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </section>
      </main>
    </SiteLayout>
  );
}
