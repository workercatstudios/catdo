import { useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { ClerkProvider, SignIn, useClerk, useAuth } from "@clerk/react";
import { App } from "./App";
import "./style.css";
const offlineToken = async () => null;
function SignedApp() {
  const { signOut } = useClerk();
  const { isLoaded, isSignedIn, userId, getToken } = useAuth();
  const token = useCallback(() => getToken(), [getToken]);
  useEffect(() => {
    if (isLoaded && isSignedIn && userId)
      localStorage.setItem("catdo:last-user", userId);
    else if (isLoaded && !isSignedIn && navigator.onLine)
      localStorage.removeItem("catdo:last-user");
  }, [isLoaded, isSignedIn, userId]);
  if (!isLoaded) return <OfflineGate />;
  if (!isSignedIn)
    return (
      <div className="auth-page">
        <a className="brand" href="/">
          <img src="/cat.png" alt="" />
          CatDo
        </a>
        <SignIn
          routing="hash"
          forceRedirectUrl="/app"
          signUpForceRedirectUrl="/app"
        />
      </div>
    );
  return (
    <App
      owner={userId!}
      getToken={token}
      account={
        <button
          className="text-button"
          onClick={() => {
            localStorage.removeItem("catdo:last-user");
            void signOut({ redirectUrl: "/" });
          }}
        >
          Sign out
        </button>
      }
    />
  );
}
function OfflineGate({ unavailable = false }: { unavailable?: boolean }) {
  const [open, setOpen] = useState(false),
    [offline, setOffline] = useState(!navigator.onLine);
  const owner = localStorage.getItem("catdo:last-user");
  useEffect(() => {
    const changed = () => setOffline(!navigator.onLine);
    window.addEventListener("online", changed);
    window.addEventListener("offline", changed);
    return () => {
      window.removeEventListener("online", changed);
      window.removeEventListener("offline", changed);
    };
  }, []);
  return open && owner ? (
    <App
      owner={owner}
      getToken={offlineToken}
      account={<button onClick={() => location.reload()}>Reconnect</button>}
    />
  ) : (
    <main className="loading">
      <img src="/cat.png" alt="" />
      <h1>
        {offline
          ? "You’re offline."
          : unavailable
            ? "Connection unavailable."
            : "Opening CatDo…"}
      </h1>
      {owner ? (
        <>
          <p>Your saved tasks are available on this device.</p>
          <button className="primary" onClick={() => setOpen(true)}>
            Open saved tasks
          </button>
        </>
      ) : (
        <>
          <p>
            {offline || unavailable
              ? "Connect once to sign in and save your tasks here."
              : "Getting your workspace ready."}
          </p>
          {unavailable && (
            <button onClick={() => location.reload()}>Try again</button>
          )}
        </>
      )}
    </main>
  );
}
function Landing() {
  return (
    <div className="landing">
      <header>
        <a className="brand" href="/">
          <img src="/cat.png" alt="" />
          <strong>CatDo</strong>
          <span>by WorkerCat</span>
        </a>
        <a className="button" href="/app">
          Open CatDo ↗
        </a>
      </header>
      <main>
        <p className="eyebrow">A place for what needs doing.</p>
        <h1>
          A little more
          <br />
          organized.
        </h1>
        <p className="intro">
          Your tasks, your projects, your own pace.
          <br />
          Make room for work — and everything else.
        </p>
        <a className="button primary" href="/app">
          Get started <span>→</span>
        </a>
        <p className="availability">
          Available on the web and Linux. Made by WorkerCat.
        </p>
        <div className="landing-example" aria-label="Example task list">
          <div>
            <span className="muted">Personal /</span> Today
            <span className="example-label">A day, with a little room.</span>
          </div>
          <h2>One thing at a time.</h2>
          <p>
            <span className="example-check" />
            Plan the next small step <small>Today</small>
          </p>
          <p>
            <span className="example-check" />
            Make time for a walk <small>Every day</small>
          </p>
          <p>
            <span className="example-check" />
            Pick up where you left off <small>Personal</small>
          </p>
          <img src="/cat.png" alt="" />
        </div>
        <div className="landing-details">
          <section>
            <h2>Separate spaces.</h2>
            <p>
              Keep work and personal tasks in their own workspaces. Projects
              give each a home.
            </p>
          </section>
          <section>
            <h2>A view of your day.</h2>
            <p>
              Plan with scheduled dates, keep deadlines in sight, and make room
              in your calendar.
            </p>
          </section>
          <section>
            <h2>Keep going offline.</h2>
            <p>
              Changes stay on your device and sync when you reconnect. Repeating
              tasks come back on schedule.
            </p>
          </section>
        </div>
      </main>
      <footer>
        <a href="https://workercat.com">WorkerCat</a>
        <span>Small tools. Room to think.</span>
        <a href="/app">Open CatDo</a>
      </footer>
    </div>
  );
}
async function boot() {
  const root = createRoot(document.getElementById("root")!);
  if (!location.pathname.startsWith("/app")) {
    root.render(<Landing />);
    return;
  }
  if (!navigator.onLine) {
    root.render(<OfflineGate />);
    return;
  }
  try {
    const response = await fetch("/api/config");
    if (!response.ok) throw Error();
    const config = (await response.json()) as { publishableKey: string };
    if (!config.publishableKey) throw Error();
    root.render(
      <ClerkProvider
        publishableKey={config.publishableKey}
        afterSignOutUrl="/"
        localization={{
          signIn: {
            start: {
              title: "Sign in to CatDo",
              subtitle: "Use your WorkerCat account",
            },
          },
          signUp: {
            start: {
              title: "Create your WorkerCat account",
              subtitle: "Continue to CatDo",
            },
          },
        }}
        appearance={{
          variables: {
            colorPrimary: "#426756",
            borderRadius: "8px",
            fontFamily: "Inter, system-ui, sans-serif",
          },
        }}
      >
        <SignedApp />
      </ClerkProvider>,
    );
  } catch {
    root.render(<OfflineGate unavailable />);
  }
}
void boot();
if ("serviceWorker" in navigator)
  void navigator.serviceWorker.register("/sw.js");
