import { useEffect, useState, type ReactNode } from "react";
import {
  accountRequest,
  downloadJson,
  privacyUrl,
  termsUrl,
  type GetToken,
  type LegalStatus,
} from "./lib/legal";

export function LegalLinks() {
  return (
    <nav className="auth-links" aria-label="CatDo information">
      <a href={termsUrl} target="_blank" rel="noreferrer">
        Terms of service
      </a>
      <a href={privacyUrl} target="_blank" rel="noreferrer">
        Privacy policy
      </a>
      <a href="/privacy">Privacy & your data</a>
      <a href="/privacy-requests">Privacy requests</a>
      <a href="/support">Help & support</a>
    </nav>
  );
}

export function AgeEligibility({
  onContinue,
  onSavedTasks,
}: {
  onContinue: () => void;
  onSavedTasks?: () => void;
}) {
  const [confirmed, setConfirmed] = useState(false);
  return (
    <main className="auth-page">
      <a className="brand" href="/">
        <img src="/icon.png" alt="" />
        CatDo
      </a>
      <div className="legal-panel">
        <h1>Before you sign in</h1>
        <p>
          CatDo accounts are for people aged 13 and older. If your country
          requires a higher age or a parent’s or guardian’s permission, those
          requirements apply too.
        </p>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (confirmed) onContinue();
          }}
        >
          <label className="legal-check">
            <input
              type="checkbox"
              required
              checked={confirmed}
              onChange={(event) => setConfirmed(event.target.checked)}
            />
            <span>
              I’m at least 13 and meet the age and permission requirements where
              I live.
            </span>
          </label>
          <button className="primary" type="submit" disabled={!confirmed}>
            Continue to sign in
          </button>
        </form>
        {onSavedTasks && (
          <button onClick={onSavedTasks}>Open saved tasks</button>
        )}
        <p className="muted">
          You can request help with data in an existing account without
          accepting the terms.
        </p>
        <LegalLinks />
      </div>
    </main>
  );
}

export function LegalAccess({
  getToken,
  onLocal,
  signOut,
  children,
}: {
  getToken: GetToken;
  onLocal: () => void;
  signOut: ReactNode;
  children: ReactNode;
}) {
  const [status, setStatus] = useState<LegalStatus | null>(null);
  const [error, setError] = useState("");
  const [age, setAge] = useState(false);
  const [accepted, setAccepted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let current = true;
    setError("");
    accountRequest<LegalStatus>(getToken, "/api/legal")
      .then((value) => {
        if (current) setStatus(value);
      })
      .catch(() => {
        if (current)
          setError(
            "Couldn’t check your terms acceptance. Your saved tasks are still available.",
          );
      });
    return () => {
      current = false;
    };
  }, [getToken, attempt]);
  async function accept() {
    if (!status || !age || !accepted || busy) return;
    setBusy(true);
    setError("");
    try {
      await accountRequest(getToken, "/api/legal", {
        version: status.version,
        accepted: true,
        ageConfirmed: true,
      });
      // Read the durable receipt before allowing sync.
      setStatus(await accountRequest<LegalStatus>(getToken, "/api/legal"));
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Couldn’t save your acceptance. Please try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function exportCloud() {
    setBusy(true);
    setError("");
    try {
      const snapshot = await accountRequest<{ data: unknown }>(
        getToken,
        "/api/sync",
      );
      downloadJson(snapshot.data, "catdo-cloud-tasks.json");
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Couldn’t export your cloud tasks. Please try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  if (status?.accepted) return children;
  return (
    <main className="auth-page">
      <a className="brand" href="/">
        <img src="/icon.png" alt="" />
        CatDo
      </a>
      <div className="legal-panel">
        <h1>
          {status
            ? "Review the terms for cloud sync"
            : "Checking your account…"}
        </h1>
        {status && (
          <>
            <p>
              CatDo is available for personal and workplace use. Before syncing,
              read WorkerCat’s{" "}
              <a href={status.termsUrl} target="_blank" rel="noreferrer">
                terms of service
              </a>{" "}
              and{" "}
              <a href={status.privacyUrl} target="_blank" rel="noreferrer">
                privacy policy
              </a>{" "}
              (version {status.version}).
            </p>
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void accept();
              }}
            >
              <label className="legal-check">
                <input
                  type="checkbox"
                  required
                  checked={age}
                  onChange={(event) => setAge(event.target.checked)}
                />
                <span>
                  I’m at least {status.minimumAge} and meet any higher age or
                  parent or guardian permission requirements where I live.
                </span>
              </label>
              <label className="legal-check">
                <input
                  type="checkbox"
                  required
                  checked={accepted}
                  onChange={(event) => setAccepted(event.target.checked)}
                />
                <span>
                  I agree to the terms of service, version {status.version}.
                </span>
              </label>
              <p className="muted">
                We record the terms version, your age confirmation, and the
                acceptance time for your CatDo account. The privacy policy
                explains how we use your data.
              </p>
              <button
                className="primary"
                type="submit"
                disabled={!age || !accepted || busy}
              >
                {busy ? "Saving…" : "Accept and continue"}
              </button>
            </form>
          </>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {error && !status && (
          <button onClick={() => setAttempt((value) => value + 1)}>
            Try again
          </button>
        )}
        <div className="legal-actions">
          <button onClick={onLocal}>Open saved tasks without sync</button>
          <button disabled={busy} onClick={() => void exportCloud()}>
            Export cloud tasks
          </button>
          {signOut}
        </div>
        <p className="muted">
          You can keep using and exporting saved tasks without accepting. Cloud
          export and private requests remain available.
        </p>
        <LegalLinks />
      </div>
    </main>
  );
}
