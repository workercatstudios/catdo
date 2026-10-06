import { useEffect, useState, type ReactNode } from "react";
import { Button } from "./components/ui/pop-button";
import { Checkbox } from "./components/ui/pop-checkbox";
import {
  ageAnswered,
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

export function AgeEligibility({ onContinue }: { onContinue: () => void }) {
  const [confirmed, setConfirmed] = useState(false);
  return (
    <main className="auth-page">
      <a className="brand" href="/">
        <img src="/icon.png" alt="" />
        <span>
          <strong>CatDo</strong>
        </span>
      </a>
      <div className="legal-panel sign-in-intro">
        <h1>Sign in to CatDo</h1>
        <p className="muted">Your tasks, on every device.</p>
        <label className="legal-check age-check">
          <Checkbox
            checked={confirmed}
            onCheckedChange={(checked) => {
              setConfirmed(checked);
              // Let the tick land before the sign-in form takes its place.
              if (checked) setTimeout(onContinue, 280);
            }}
          />
          <span>I’m 13 or older and meet any age rules where I live</span>
        </label>
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
  // The device's one-time 13+ answer stands in for asking again.
  const [askAge] = useState(() => !ageAnswered());
  const [age, setAge] = useState(!askAge);
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
        <span>
          <strong>CatDo</strong>
        </span>
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
              {askAge && (
                <label className="legal-check">
                  <Checkbox
                    required
                    checked={age}
                    onCheckedChange={(checked) => setAge(checked)}
                  />
                  <span>
                    I’m at least {status.minimumAge} and meet any higher age or
                    parent or guardian permission requirements where I live.
                  </span>
                </label>
              )}
              <label className="legal-check">
                <Checkbox
                  required
                  checked={accepted}
                  onCheckedChange={(checked) => setAccepted(checked)}
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
              <Button type="submit" disabled={!age || !accepted || busy}>
                {busy ? "Saving…" : "Accept and continue"}
              </Button>
            </form>
          </>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {error && !status && (
          <Button
            variant="outline"
            onClick={() => setAttempt((value) => value + 1)}
          >
            Try again
          </Button>
        )}
        <div className="legal-actions">
          <Button variant="outline" onClick={onLocal}>
            Open saved tasks without sync
          </Button>
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => void exportCloud()}
          >
            Export cloud tasks
          </Button>
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
