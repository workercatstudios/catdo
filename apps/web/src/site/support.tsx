import { Link } from "@tanstack/react-router";
import { SiteLayout, PageIntro } from "./layout";
import { guides, repository } from "./content";

export function SupportPage() {
  return (
    <SiteLayout>
      <main id="main" className="public-page article">
        <a href="/" className="back-link">
          ← Back to CatDo
        </a>
        <PageIntro
          eyebrow="Help & support"
          title="A little help, when you need it."
        >
          Start with a guide, or tell us what went wrong.
        </PageIntro>
        <div className="article-body information-body">
          <section aria-labelledby="guides">
            <h2 id="guides">Guides</h2>
            <ul>
              {guides.map((guide) => (
                <li key={guide.slug}>
                  <Link to="/help/$slug" params={{ slug: guide.slug }}>
                    {guide.searchTitle}
                  </Link>
                </li>
              ))}
            </ul>
          </section>
          <section aria-labelledby="sync-help">
            <h2 id="sync-help">If tasks look missing or sync stops</h2>
            <p>
              Check the CatDo account and workspace you’re using, then look in
              Completed and archived projects. Export your tasks from Settings
              before clearing storage or reinstalling. Unsynced changes may
              exist only on this device.
            </p>
            <p>
              If sign-in is temporarily unavailable, Open saved tasks lets you
              continue with data already stored on the device. Sign in again
              when connected to resume syncing.
            </p>
          </section>
          <section aria-labelledby="report-bug">
            <h2 id="report-bug">Report a bug or ask a question</h2>
            <p>
              <a href={`${repository}/issues`}>Open a GitHub issue</a> for
              general support, bugs, and feature requests. Include your
              platform, app version or browser, what you expected, and steps to
              reproduce it.
            </p>
            <p>
              GitHub issues are public. Do not post private tasks, account
              details, tokens, or data exports. Review screenshots for personal
              information before sharing.
            </p>
          </section>
          <section aria-labelledby="security-report">
            <h2 id="security-report">Report a security problem privately</h2>
            <p>
              For a suspected vulnerability, use{" "}
              <a href={`${repository}/security/advisories/new`}>
                GitHub’s private vulnerability report
              </a>
              . Include the affected version and reproduction steps. Please do
              not open a public issue for a security problem.
            </p>
          </section>
          <section aria-labelledby="downloads">
            <h2 id="downloads">Downloads and release notes</h2>
            <p>
              <a href={`${repository}/releases`}>GitHub Releases</a> has the
              Android APK, Linux downloads, checksums, and release notes.
              Windows is planned. CatDo is in active development; native apps
              and web features can differ.
            </p>
          </section>
        </div>
      </main>
    </SiteLayout>
  );
}
