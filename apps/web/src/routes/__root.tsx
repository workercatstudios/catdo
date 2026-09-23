import {
  HeadContent,
  Scripts,
  createRootRoute,
  Outlet,
  Link,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect, useState, type ReactNode } from "react";
import { themeScript, ThemeListener } from "../lib/theme";
import stylesheet from "../style.css?url";
export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { name: "theme-color", content: "#fdfcfb" },
    ],
    links: [
      { rel: "stylesheet", href: stylesheet },
      { rel: "icon", type: "image/png", href: "/icon.png" },
    ],
  }),
  shellComponent: Document,
  component: Outlet,
  notFoundComponent: () => (
    <main className="not-found">
      <img src="/icon.png" width="64" height="64" alt="" />
      <p className="eyebrow">404 · Nothing here</p>
      <h1>This page wandered off.</h1>
      <p>Let’s get you back to familiar ground.</p>
      <Link to="/">Back to CatDo →</Link>
    </main>
  ),
  errorComponent: () => (
    <main className="not-found">
      <h1>Something interrupted CatDo.</h1>
      <p>Your saved tasks are still on this device.</p>
      <a href="/app">Try opening your tasks again</a>
    </main>
  ),
});
function Document({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
        <HeadContent />
      </head>
      <body>
        <ThemeListener />
        {children}
        <Updates />
        <Scripts />
      </body>
    </html>
  );
}
function Updates() {
  const [waiting, setWaiting] = useState<ServiceWorker | null>(null);
  const inApp = useRouterState({
    select: (state) =>
      state.location.pathname === "/app" ||
      state.location.pathname.startsWith("/app/"),
  });
  useEffect(() => {
    const pointer = () => {
      document.documentElement.dataset.input = "pointer";
    };
    const keyboard = () => {
      document.documentElement.dataset.input = "keyboard";
    };
    window.addEventListener("pointerdown", pointer);
    window.addEventListener("keydown", keyboard);
    if (import.meta.env.PROD && "serviceWorker" in navigator)
      void navigator.serviceWorker
        .register("/sw.js")
        .then((r) => {
          if (r.waiting && navigator.serviceWorker.controller)
            setWaiting(r.waiting);
          r.addEventListener("updatefound", () => {
            const next = r.installing;
            next?.addEventListener("statechange", () => {
              if (
                next.state === "installed" &&
                navigator.serviceWorker.controller
              )
                setWaiting(next);
            });
          });
        })
        .catch(() => {});
    return () => {
      window.removeEventListener("pointerdown", pointer);
      window.removeEventListener("keydown", keyboard);
    };
  }, []);
  return waiting && inApp ? (
    <div className="update-notice" role="status">
      A fresh CatDo is ready.{" "}
      <button
        onClick={() => {
          if (!confirm("Reload CatDo? Save any open task first.")) return;
          navigator.serviceWorker.addEventListener(
            "controllerchange",
            () => location.reload(),
            { once: true },
          );
          waiting.postMessage({ type: "ACTIVATE" });
        }}
      >
        Reload
      </button>
    </div>
  ) : null;
}
