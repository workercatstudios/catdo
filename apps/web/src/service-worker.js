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
  if (ASSETS.includes(url.pathname) && url.pathname !== "/app")
    event.respondWith(
      caches
        .match(request, { cacheName: CACHE, ignoreVary: true })
        .then((hit) => hit || fetch(request)),
    );
});
