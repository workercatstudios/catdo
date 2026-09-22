// Only cache the application shell and same-origin public assets. Never cache
// task API responses, auth traffic, or credentials; tasks live in account-scoped IDB.
const CACHE = "catdo-shell-__VERSION__";
self.addEventListener("install", (event) =>
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(__ASSETS__))),
);
self.addEventListener("activate", (event) =>
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key.startsWith("catdo-shell-") && key !== CACHE)
            .map((key) => caches.delete(key)),
        ),
      )
      .then(() => self.clients.claim()),
  ),
);
self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (
    url.origin !== self.location.origin ||
    event.request.method !== "GET" ||
    url.pathname.startsWith("/api/")
  )
    return;
  if (event.request.mode === "navigate") {
    event.respondWith(
      fetch(event.request)
        .then((response) => {
          if (response.ok && url.pathname.startsWith("/app")) {
            const clone = response.clone();
            event.waitUntil(
              caches.open(CACHE).then((c) => c.put("/app", clone)),
            );
          }
          return response;
        })
        .catch(() => caches.match("/app")),
    );
    return;
  }
  if (url.pathname.startsWith("/assets/") || url.pathname === "/cat.png")
    event.respondWith(
      caches.match(event.request).then(
        (cached) =>
          cached ||
          fetch(event.request).then((response) => {
            if (response.ok) {
              const clone = response.clone();
              event.waitUntil(
                caches.open(CACHE).then((c) => c.put(event.request, clone)),
              );
            }
            return response;
          }),
      ),
    );
});
