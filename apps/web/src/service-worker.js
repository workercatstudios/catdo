const CACHE = "catdo-shell-start-__VERSION__";
const ASSETS = __ASSETS__;
self.addEventListener("install", (event) =>
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(ASSETS))),
);
self.addEventListener("message", (event) => {
  if (event.data?.type === "ACTIVATE") self.skipWaiting();
});
self.addEventListener("activate", (event) =>
  event.waitUntil(
    (async () => {
      for (const key of await caches.keys())
        if (key.startsWith("catdo-shell-") && key !== CACHE)
          await caches.delete(key);
      await self.clients.claim();
    })(),
  ),
);
self.addEventListener("fetch", (event) => {
  const request = event.request,
    url = new URL(request.url);
  if (
    request.method !== "GET" ||
    url.origin !== self.location.origin ||
    url.pathname.startsWith("/api/")
  )
    return;
  const app = url.pathname === "/app" || url.pathname.startsWith("/app/");
  if (request.mode === "navigate" && app) {
    event.respondWith(
      fetch(request).catch(async () => {
        const cached = await caches.match("/app", {
          cacheName: CACHE,
          ignoreVary: true,
        });
        return cached || Response.error();
      }),
    );
    return;
  }
  if (!ASSETS.includes(url.pathname) || url.pathname === "/app") return;
  const cached = () =>
    caches.match(request, { cacheName: CACHE, ignoreVary: true });
  // Fingerprinted build files never change, so the cache wins. Public files
  // such as the icon keep their names, so the network wins while online.
  event.respondWith(
    url.pathname.startsWith("/assets/")
      ? cached().then((hit) => hit || fetch(request))
      : fetch(request).catch(() =>
          cached().then((hit) => hit || Response.error()),
        ),
  );
});
