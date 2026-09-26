import { useCallback, useEffect, useState } from "react";
import { ClerkProvider, SignIn, useClerk, useAuth } from "@clerk/react";
import { App } from "./App";
import { AgeEligibility, LegalAccess, LegalLinks } from "./LegalAccess";
import { PrivacyRequests } from "./PrivacyRequests";
const offlineToken = async () => null;
function SignedApp({ privacyRequests = false }: { privacyRequests?: boolean }) {
  const [local, setLocal] = useState(false);
  const { signOut } = useClerk();
  const { isLoaded, isSignedIn, userId, getToken } = useAuth();
  const token = useCallback(() => getToken(), [getToken]);
  useEffect(() => {
    if (isLoaded && isSignedIn && userId)
      localStorage.setItem("catdo:last-user", userId);
    else if (isLoaded && !isSignedIn && navigator.onLine)
      localStorage.removeItem("catdo:last-user");
  }, [isLoaded, isSignedIn, userId]);
  if (!isLoaded)
    return privacyRequests ? (
      <main className="loading">
        <h1>Opening your account…</h1>
        <a href="/app">Back to tasks</a>
      </main>
    ) : (
      <OfflineGate />
    );
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
          {...(privacyRequests ? { signUpUrl: "/app", withSignUp: false } : {})}
        />
        <LegalLinks />
      </div>
    );
  const signOutButton = (
    <button
      className="text-button"
      onClick={() => {
        localStorage.removeItem("catdo:last-user");
        sessionStorage.removeItem("catdo:age-confirmed");
        void signOut({ redirectUrl: "/" });
      }}
    >
      Sign out
    </button>
  );
  if (privacyRequests)
    return (
      <PrivacyRequests key={userId} getToken={token} signOut={signOutButton} />
    );
  const app = (
    <App
      key={userId}
      owner={userId!}
      getToken={local ? offlineToken : token}
      account={
        local ? (
          <button onClick={() => setLocal(false)}>Review terms to sync</button>
        ) : (
          signOutButton
        )
      }
    />
  );
  return local ? (
    app
  ) : (
    <LegalAccess
      key={userId}
      getToken={token}
      onLocal={() => setLocal(true)}
      signOut={signOutButton}
    >
      {app}
    </LegalAccess>
  );
}

function OfflineGate({
  unavailable = false,
  openInitially = false,
}: {
  unavailable?: boolean;
  openInitially?: boolean;
}) {
  const [open, setOpen] = useState(openInitially),
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

export function AuthApp({
  privacyRequests = false,
}: {
  privacyRequests?: boolean;
}) {
  const [ageConfirmed, setAgeConfirmed] = useState(
    () => sessionStorage.getItem("catdo:age-confirmed") === "yes",
  );
  const [local, setLocal] = useState(false);
  const [config, setConfig] = useState<{ publishableKey: string } | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  useEffect(() => {
    if ((!ageConfirmed && !privacyRequests) || local) return;
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
  }, [ageConfirmed, privacyRequests, local]);
  if (local) return <OfflineGate openInitially />;
  if (!privacyRequests && !ageConfirmed && navigator.onLine)
    return (
      <AgeEligibility
        onContinue={() => {
          sessionStorage.setItem("catdo:age-confirmed", "yes");
          setAgeConfirmed(true);
        }}
        onSavedTasks={
          localStorage.getItem("catdo:last-user")
            ? () => setLocal(true)
            : undefined
        }
      />
    );
  if (!config)
    return privacyRequests ? (
      <main className="loading">
        <h1>
          {unavailable ? "Connection unavailable." : "Opening your account…"}
        </h1>
        <p>Connect to sign in and manage your private requests.</p>
        {unavailable && (
          <button onClick={() => location.reload()}>Try again</button>
        )}
        <a href="/app">Back to tasks</a>
      </main>
    ) : (
      <OfflineGate unavailable={unavailable} />
    );
  return (
    <ClerkProvider
      publishableKey={config.publishableKey}
      afterSignOutUrl="/"
      localization={{
        signIn: {
          start: {
            title: "Sign in to CatDo",
            subtitle: "Use your CatDo account",
          },
        },
        signUp: {
          start: {
            title: "Create your CatDo account",
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
      <SignedApp privacyRequests={privacyRequests} />
    </ClerkProvider>
  );
}
