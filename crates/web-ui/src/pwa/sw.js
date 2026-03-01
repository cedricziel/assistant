// -- Assistant PWA Service Worker (full offline) --
//
// Cache strategy:
//   - Static assets (CSS, vendored JS, icons): cache-first
//   - HTML pages: network-first, fall back to cache, then offline page
//   - Non-GET / SSE streams: network-only (pass through)

// Replaced at serve-time by the Rust handler:
//   Release builds → CARGO_PKG_VERSION (e.g. "0.1.21")
//   Debug  builds → CARGO_PKG_VERSION + startup timestamp (e.g. "0.1.21-dev.1740700000")
const CACHE_VERSION = "__APP_VERSION__";
const STATIC_CACHE = `assistant-static-${CACHE_VERSION}`;
const PAGE_CACHE = `assistant-pages-${CACHE_VERSION}`;

const STATIC_ASSETS = [
  "/pwa/manifest.webmanifest",
  "/pwa/icon.svg",
  "/pwa/icon-maskable.svg",
  "/pwa/offline",
  "__APP_CSS_URL__",
  "__HTMX_URL__",
  "__HTMX_SSE_URL__",
];

// -- Install: pre-cache app shell -------------------------------------------

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(STATIC_CACHE)
      .then((cache) => cache.addAll(STATIC_ASSETS))
      .then(() => self.skipWaiting()),
  );
});

// -- Activate: purge old caches ---------------------------------------------

self.addEventListener("activate", (event) => {
  const CURRENT = [STATIC_CACHE, PAGE_CACHE];
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter(
              (key) => key.startsWith("assistant-") && !CURRENT.includes(key),
            )
            .map((key) => caches.delete(key)),
        ),
      )
      .then(() => self.clients.claim()),
  );
});

// -- Fetch: route to appropriate strategy -----------------------------------

self.addEventListener("fetch", (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET requests (POST, DELETE, etc.)
  if (request.method !== "GET") return;

  // Skip SSE streaming endpoints — these must stay live
  if (url.pathname.includes("/stream")) return;

  // Skip htmx partial requests — let them go to network
  if (request.headers.get("HX-Request") === "true") return;

  // Static PWA assets: cache-first
  if (url.pathname.startsWith("/pwa/")) {
    event.respondWith(cacheFirst(request, STATIC_CACHE));
    return;
  }

  // Fingerprinted static assets (CSS): cache-first
  if (url.pathname.startsWith("/static/")) {
    event.respondWith(cacheFirst(request, STATIC_CACHE));
    return;
  }

  // Login/logout: network-only (auth flow must not be cached)
  if (url.pathname === "/login" || url.pathname === "/logout") return;

  // A2A protocol endpoints: network-only
  if (
    url.pathname.startsWith("/.well-known/") ||
    url.pathname.startsWith("/message/") ||
    url.pathname.startsWith("/tasks")
  ) {
    return;
  }

  // HTML pages: network-first with cache fallback
  const accept = request.headers.get("Accept") || "";
  if (accept.includes("text/html") || !url.pathname.includes(".")) {
    event.respondWith(networkFirst(request, PAGE_CACHE));
    return;
  }
});

// -- Strategies --------------------------------------------------------------

async function cacheFirst(request, cacheName) {
  const cached = await caches.match(request);
  if (cached) return cached;

  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(cacheName);
      cache.put(request, response.clone());
    }
    return response;
  } catch (_err) {
    return new Response("Offline", {
      status: 503,
      statusText: "Service Unavailable",
    });
  }
}

async function networkFirst(request, cacheName) {
  try {
    const response = await fetch(request);
    if (response.ok && response.status !== 401) {
      const cache = await caches.open(cacheName);
      cache.put(request, response.clone());
    }
    return response;
  } catch (_err) {
    // Network failed — try cache
    const cached = await caches.match(request);
    if (cached) return cached;

    // Nothing cached — return the offline page for navigation requests
    const offlinePage = await caches.match("/pwa/offline");
    if (offlinePage) return offlinePage;

    return new Response("Offline", {
      status: 503,
      statusText: "Service Unavailable",
    });
  }
}
