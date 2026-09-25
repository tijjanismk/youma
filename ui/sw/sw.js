// Service worker de Youma (fiche 0020) — produit par vite.config.ts, ne pas modifier dans dist/.
// Coquille de l'application en cache : ouverture instantanée même si le Wi-Fi est faible.
// L'API (/api/) ne passe jamais par le cache : les données viennent toujours du poste central.
const VERSION = "__VERSION__";
const FICHIERS = __FICHIERS__;
const CACHE = `youma-${VERSION}`;

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(FICHIERS.map((f) => `/${f}`))));
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches
      .keys()
      .then((noms) => Promise.all(noms.filter((n) => n.startsWith("youma-") && n !== CACHE).map((n) => caches.delete(n))))
      .then(() => self.clients.claim()),
  );
});

// Nouvelle version : activée quand l'utilisateur le demande (pas au milieu d'une commande).
self.addEventListener("message", (e) => {
  if (e.data === "activer") self.skipWaiting();
});

self.addEventListener("fetch", (e) => {
  const req = e.request;
  const url = new URL(req.url);
  if (req.method !== "GET" || url.origin !== self.location.origin || url.pathname.startsWith("/api/")) return;
  if (req.mode === "navigate") {
    // Toutes les pages (personnel, menu client, suivi, livreur, propriétaire) partagent index.html.
    e.respondWith(caches.match("/index.html", { cacheName: CACHE }).then((r) => r || fetch(req)));
    return;
  }
  e.respondWith(
    caches.match(req, { cacheName: CACHE }).then(
      (r) =>
        r ||
        fetch(req).then((rep) => {
          // Fichiers versionnés (assets/…) : gardés pour la prochaine ouverture.
          if (rep.ok && url.pathname.startsWith("/assets/")) {
            const copie = rep.clone();
            caches.open(CACHE).then((c) => c.put(req, copie));
          }
          return rep;
        }),
    ),
  );
});
