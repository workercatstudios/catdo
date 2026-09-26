import { privacyUrl, termsUrl } from "../lib/legal";
import { SiteLayout, PageIntro } from "./layout";

export function PrivacyPage() {
  return (
    <SiteLayout>
      <main id="main" className="public-page article">
        <a href="/" className="back-link">
          ← Back to CatDo
        </a>
        <PageIntro eyebrow="Privacy & your data" title="Where your tasks live.">
          What CatDo saves, which services help it work, and what you can
          control.
        </PageIntro>
        <p className="document-date">Updated September 26, 2026</p>
        <div className="article-body information-body">
          <p>
            This page explains CatDo’s data controls. The{" "}
            <a href={privacyUrl}>WorkerCat privacy policy</a> and{" "}
            <a href={termsUrl}>terms of service</a> apply to CatDo. Accounts are
            for people aged 13 and older, subject to higher local age or
            permission requirements.
          </p>
          <section aria-labelledby="task-data">
            <h2 id="task-data">Your tasks and account</h2>
            <p>
              CatDo is a WorkerCat task manager. It saves the information you
              enter: workspace and project names, tasks, notes, dates, subtasks,
              reminders, and completion history.
            </p>
            <p>
              Clerk handles sign-in for your CatDo account. CatDo uses your
              verified account identifier to keep your synced tasks separate
              from other accounts. The Linux app can also be used locally
              without signing in.
            </p>
          </section>
          <section aria-labelledby="device-storage">
            <h2 id="device-storage">Cookies and storage on your device</h2>
            <p>
              The web app stores tasks in your browser for offline use. It
              remembers your appearance preference and the last account used,
              and caches app files so they can open without a connection. Clerk
              uses browser cookies and other storage to support sign-in and
              sessions.
            </p>
            <p>
              The CatDo web application does not include advertising pixels or a
              product analytics SDK. That does not mean visiting the site is
              anonymous: hosting and sign-in providers process connection and
              request information. Fonts are served with the site.
            </p>
            <p>
              The Linux and Android apps also save tasks locally. Signing out
              does not erase saved tasks. On a shared device, export anything
              you need before clearing CatDo’s site or app data. Clearing local
              storage can permanently remove changes that have not synced.
            </p>
          </section>
          <section aria-labelledby="providers">
            <h2 id="providers">Cloud sync and service providers</h2>
            <p>
              When you sign in and sync, your task data is sent over HTTPS to
              CatDo on Cloudflare and stored separately for your account. CatDo
              is not end-to-end encrypted: the service processes your tasks to
              store and sync them.
            </p>
            <ul>
              <li>
                <a href="https://clerk.com/legal/privacy">Clerk</a> provides
                account sign-in and session management.
              </li>
              <li>
                <a href="https://www.cloudflare.com/privacypolicy/">
                  Cloudflare
                </a>{" "}
                hosts the website, API, and synced task storage, and processes
                request diagnostics.
              </li>
              <li>
                <a href="https://firebase.google.com/support/privacy">
                  Google Firebase
                </a>{" "}
                provides Android crash reporting and notification delivery.
              </li>
              <li>
                <a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement">
                  GitHub
                </a>{" "}
                hosts app downloads, source code, and public support issues.
                Following those links connects you to GitHub.
              </li>
            </ul>
          </section>
          <section aria-labelledby="diagnostics">
            <h2 id="diagnostics">Diagnostics and Android notifications</h2>
            <p>
              Cloudflare processes request metadata and server diagnostics to
              help operate CatDo. CatDo’s sync error logging avoids task
              payloads and sign-in tokens.
            </p>
            <p>
              The Android app uses Firebase Crashlytics for crash reports and
              limited failure events, collected automatically. Reports can
              include app and device details, installation identifiers, and
              error traces. CatDo does not explicitly attach your tasks, account
              ID, or sign-in tokens as report fields.
            </p>
            <p>
              Android notification registration is enabled when you opt in
              through the app. Firebase processes installation information to
              deliver notifications. You can turn notifications off in the app.
              Crash reporting is separate from that notification setting. There
              is currently no in-app switch to turn off crash reporting.
            </p>
          </section>
          <section aria-labelledby="export-delete">
            <h2 id="export-delete">Exporting and removing data</h2>
            <p>
              In the web app, open Settings and choose Export all tasks to save
              a JSON copy of your workspaces, projects, tasks, and completion
              history. Keep exports somewhere private. You can edit or delete
              individual tasks in CatDo; online changes sync to your account.
            </p>
            <p>
              Signing out, clearing browser storage, or uninstalling an app does
              not itself delete the cloud copy. Cloud task data stays until you
              delete it or request removal; we do not automatically expire it
              for inactivity. CatDo does not currently provide a self-service
              button to erase all cloud account data. Deleting a task from the
              current list is also different from removing past completion
              records, other device copies, or provider backups.
            </p>
            <p>
              You can <a href="/privacy-requests">submit a private request</a>{" "}
              to access, correct, or delete your CatDo data, or ask a privacy
              question. Check that page for a response. You can sign in to
              manage requests and export cloud tasks without accepting updated
              terms. A request requires review and does not immediately erase
              data.
            </p>
          </section>
        </div>
        <a href="/support" className="inline-link">
          Help with CatDo →
        </a>
      </main>
    </SiteLayout>
  );
}
