import { useCallback, useEffect, useState } from "react";
import { ClerkProvider, SignIn, useClerk, useAuth } from "@clerk/react";
import { App } from "./App";
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
          <img src="/icon.png" alt="" />
          CatDo
        </a>
        <SignIn
          routing="hash"
          forceRedirectUrl={location.pathname + location.search}
          signUpForceRedirectUrl={location.pathname + location.search}
        />
      </div>
    );
  return (
    <App
      key={userId}
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
      key={owner}
      owner={owner}
      getToken={offlineToken}
      account={<button onClick={() => location.reload()}>Reconnect</button>}
    />
  ) : (
    <main className="loading">
      <img src="/icon.png" alt="" />
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

export function AuthApp() {
  const [config, setConfig] = useState<{ publishableKey: string } | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  useEffect(() => {
    if (!navigator.onLine) {
      setUnavailable(true);
      return;
    }
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);
    fetch("/api/config", { signal: controller.signal })
      .then(async (r) => {
        if (!r.ok) throw Error();
        const value = (await r.json()) as { publishableKey: string };
        if (!value.publishableKey) throw Error();
        setConfig(value);
      })
      .catch(() => setUnavailable(true))
      .finally(() => clearTimeout(timeout));
    return () => {
      controller.abort();
      clearTimeout(timeout);
    };
  }, []);
  if (!config) return <OfflineGate unavailable={unavailable} />;
  return (
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
          colorPrimary: "#3e624e",
          borderRadius: "8px",
          fontFamily: "Inter Variable, system-ui, sans-serif",
        },
      }}
    >
      <SignedApp />
    </ClerkProvider>
  );
}
