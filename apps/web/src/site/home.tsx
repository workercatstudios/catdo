import { Link } from "@tanstack/react-router";
import { ArrowRight, Download } from "lucide-react";
import { SiteLayout } from "./layout";
import { ProductPreview } from "./product-preview";
import { Button } from "../components/ui/button";
import { release, repository } from "./content";

export function Home() {
  return (
    <SiteLayout>
      <main id="main" className="home-page">
        <section className="home-hero" aria-labelledby="hero-title">
          <div className="hero-heading">
            <p className="eyebrow">A personal task manager. Web + Linux.</p>
            <h1 id="hero-title">
              A little more
              <br />
              organized.
            </h1>
          </div>
          <div className="hero-copy">
            <p className="hero-intro">
              Get it out of your head.
              <br />
              Make a little room for your day.
            </p>
            <div className="hero-actions">
              <Button asChild size="lg">
                <a href="/app">
                  Get started <ArrowRight />
                </a>
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
              <Button asChild size="lg">
                <a href={release.appImage}>
                  <Download /> Download AppImage
                </a>
              </Button>
              <a href={release.archive}>Or get the tar.gz ↗</a>
            </div>
            <p className="download-requirements">
              Version {release.version} · Linux x86_64 · glibc 2.39+
            </p>
            <details className="install-details">
              <summary>How to install</summary>
              <p>
                Download the AppImage, allow it to run as a program in your file
                manager’s permissions, then open it. Sign in with your WorkerCat
                account to sync your tasks.
              </p>
              <p>
                If FUSE is unavailable, launch with{" "}
                <code>--appimage-extract-and-run</code>. The tar.gz includes an
                install script and instructions.
              </p>
              <a href={release.url}>Release notes and checksums ↗</a>
            </details>
            <p className="platform-note">
              On another device? <a href="/app">Use the web app.</a> Android and
              Windows apps are planned.
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
            <a className="inline-link" href={`${repository}/issues`}>
              Need a hand? <ArrowRight size={16} />
            </a>
          </div>
          <div className="questions">
            <details>
              <summary>Is CatDo free?</summary>
              <p>
                Yes. All current features are free to use. We may add an
                optional supporter tier later. The source is available under the
                PolyForm Noncommercial license.
              </p>
            </details>
            <details>
              <summary>Where do I start?</summary>
              <p>
                Put one task in Inbox. Add projects and dates when you need
                them.
              </p>
              <Link to="/help/$slug" params={{ slug: "getting-started" }}>
                Your first little plan ↗
              </Link>
            </details>
            <details>
              <summary>Can I use it offline?</summary>
              <p>
                Yes, after signing in and opening your tasks online. Saved tasks
                stay on your device, and edits sync when you reconnect. If you
                use “Open saved tasks,” sign in again when you’re back online to
                resume syncing.
              </p>
              <Link to="/help/$slug" params={{ slug: "sync-and-offline" }}>
                More about sync and offline use ↗
              </Link>
            </details>
            <details>
              <summary>How do dates and recurring tasks work?</summary>
              <p>
                A scheduled date is when you plan to do it. A due date is its
                deadline. Recurring tasks can follow a fixed schedule or repeat
                after you finish.
              </p>
              <Link to="/help/$slug" params={{ slug: "dates-and-repeats" }}>
                Dates, repeats, and reminders ↗
              </Link>
            </details>
            <details>
              <summary>Are there keyboard shortcuts?</summary>
              <p>
                Ctrl+K to search. Ctrl+Enter for a new task. Enter to save from
                quick add.
              </p>
              <Link to="/help/$slug" params={{ slug: "keyboard-shortcuts" }}>
                See the shortcuts ↗
              </Link>
            </details>
          </div>
        </section>
      </main>
    </SiteLayout>
  );
}
