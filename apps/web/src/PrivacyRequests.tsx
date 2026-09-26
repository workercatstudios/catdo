import { useEffect, useState, type ReactNode } from "react";
import { accountRequest, type GetToken } from "./lib/legal";
import { LegalLinks } from "./LegalAccess";

const kinds = {
  access: "Access my data",
  correction: "Correct my data",
  deletion: "Delete my data",
  other: "Another privacy question",
} as const;
type RequestKind = keyof typeof kinds;
type PrivacyRequest = {
  id: string;
  kind: RequestKind;
  message: string;
  status: "pending" | "resolved";
  createdAt: string;
  updatedAt: string;
  response?: string;
};
export function PrivacyRequests({
  getToken,
  signOut,
}: {
  getToken: GetToken;
  signOut: ReactNode;
}) {
  const [requests, setRequests] = useState<PrivacyRequest[]>([]);
  const [kind, setKind] = useState<RequestKind>("access");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let current = true;
    setError("");
    accountRequest<{ requests: PrivacyRequest[] }>(
      getToken,
      "/api/privacy-requests",
    )
      .then((value) => {
        if (current) {
          setRequests(value.requests);
          setLoaded(true);
        }
      })
      .catch(() => {
        if (current) setError("Couldn’t load your requests. Please try again.");
      });
    return () => {
      current = false;
    };
  }, [getToken, attempt]);
  async function submit() {
    if (busy) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const { request } = await accountRequest<{ request: PrivacyRequest }>(
        getToken,
        "/api/privacy-requests",
        { kind, message: message.trim() },
      );
      setRequests((previous) => [
        request,
        ...previous.filter((item) => item.id !== request.id),
      ]);
      setMessage("");
      setNotice(
        "Your private request is saved for review. Check this page for a response. Submitting a deletion request does not immediately delete your data.",
      );
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Couldn’t submit your request. Please try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  const pending = requests.some(
    (request) => request.kind === kind && request.status === "pending",
  );
  return (
    <main className="auth-page privacy-requests-page">
      <a className="brand" href="/">
        <img src="/icon.png" alt="" />
        CatDo
      </a>
      <div className="legal-panel">
        <h1>Privacy requests</h1>
        <p>
          Send WorkerCat a private request about your CatDo account. You don’t
          need to accept the terms. For data in another WorkerCat product, use
          that product’s account controls.
        </p>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <label>
            What do you need?
            <select
              value={kind}
              onChange={(event) => setKind(event.target.value as RequestKind)}
            >
              {Object.entries(kinds).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Details (optional)
            <textarea
              rows={4}
              maxLength={2000}
              value={message}
              onChange={(event) => setMessage(event.target.value)}
              aria-describedby="privacy-details-help"
            />
          </label>
          <p id="privacy-details-help" className="muted">
            Up to 2,000 characters. Don’t include passwords, sign-in codes, or
            information about other people unless it is needed for your request.
          </p>
          {pending && (
            <p role="status">
              You already have an open request of this kind. Check its status
              below.
            </p>
          )}
          <button
            className="primary"
            type="submit"
            disabled={busy || !loaded || pending}
          >
            {busy ? "Submitting…" : "Submit private request"}
          </button>
        </form>
        {error && (
          <>
            <p className="error" role="alert">
              {error}
            </p>
            {!loaded && (
              <button onClick={() => setAttempt((value) => value + 1)}>
                Try again
              </button>
            )}
          </>
        )}
        {notice && <p role="status">{notice}</p>}
        <section
          className="privacy-request-history"
          aria-labelledby="request-history"
        >
          <h2 id="request-history">Your requests</h2>
          {!loaded && !error && <p>Loading requests…</p>}
          {loaded && requests.length === 0 && (
            <p>You haven’t submitted a request yet.</p>
          )}
          {requests.map((request) => (
            <article key={request.id} className="privacy-request">
              <h3>{kinds[request.kind]}</h3>
              <p>
                {request.status === "pending" ? "Awaiting review" : "Resolved"}{" "}
                ·{" "}
                <time dateTime={request.createdAt}>
                  {new Date(request.createdAt).toLocaleDateString()}
                </time>
              </p>
              {request.message && (
                <p className="request-message">{request.message}</p>
              )}
              {request.response && (
                <p className="request-message">
                  <strong>Response: </strong>
                  {request.response}
                </p>
              )}
            </article>
          ))}
        </section>
        <div className="legal-actions">
          <a href="/app">Back to tasks</a>
          {signOut}
        </div>
        <LegalLinks />
      </div>
    </main>
  );
}
