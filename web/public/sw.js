const CACHE = "sss-v1"

self.addEventListener("install", () => self.skipWaiting())

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  )
})

self.addEventListener("fetch", (event) => {
  const req = event.request
  if (req.method !== "GET") return
  const url = new URL(req.url)
  if (url.origin !== location.origin || url.pathname.startsWith("/api/")) return
  // Hashed build files never change; pages are refreshed from the network when it is there.
  if (url.pathname.startsWith("/assets/") && /-[\w-]{8,}\./.test(url.pathname)) {
    event.respondWith(
      caches.match(req).then(
        (hit) =>
          hit ||
          fetch(req).then((res) => {
            if (res.ok) {
              const copy = res.clone()
              caches.open(CACHE).then((c) => c.put(req, copy))
            }
            return res
          }),
      ),
    )
    return
  }
  event.respondWith(
    fetch(req)
      .then((res) => {
        if (res.ok) {
          const copy = res.clone()
          caches.open(CACHE).then((c) => c.put(req, copy))
        }
        return res
      })
      .catch(() =>
        caches.match(req).then((hit) => {
          if (hit) return hit
          // Editor sections (/ru/vault/rooms) are the same page as the language root.
          const lang = url.pathname.split("/")[1]
          if (req.mode === "navigate" && (lang === "ru" || lang === "en")) {
            return caches.match(new URL(`/${lang}/`, location.origin)).then((root) => root || Response.error())
          }
          return Response.error()
        }),
      ),
  )
})
