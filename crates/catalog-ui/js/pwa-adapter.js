// PWA, IndexedDB Storage, and Offline Packs Adapter for Ride Atlas

(function () {
  const DB_NAME = "ride-atlas-db";
  const DB_VERSION = 1;
  const STORE_SAVED = "saved_objects";
  const MAX_TOTAL_PACKS_BYTES = 100 * 1024 * 1024; // 100 MB cap

  let dbPromise = null;
  let activeDownloads = new Map(); // routeId -> { controller, bytes, totalBytes }
  let swRegistration = null;
  let updateAvailable = false;
  let onUpdateCallback = null;

  function getDB() {
    if (!dbPromise) {
      dbPromise = new Promise((resolve, reject) => {
        if (!window.indexedDB) {
          return resolve(null);
        }
        const req = indexedDB.open(DB_NAME, DB_VERSION);
        req.onupgradeneeded = (e) => {
          const db = e.target.result;
          if (!db.objectStoreNames.contains(STORE_SAVED)) {
            db.createObjectStore(STORE_SAVED, { keyPath: "key" });
          }
        };
        req.onsuccess = (e) => resolve(e.target.result);
        req.onerror = (e) => reject(e.target.error);
      });
    }
    return dbPromise;
  }

  // --- IndexedDB Saved Objects ---
  async function getSavedKeys() {
    const db = await getDB();
    if (!db) return [];
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_SAVED, "readonly");
      const store = tx.objectStore(STORE_SAVED);
      const req = store.getAllKeys();
      req.onsuccess = () => resolve(req.result || []);
      req.onerror = () => reject(req.error);
    });
  }

  async function isSaved(kind, id) {
    const db = await getDB();
    if (!db) return false;
    const key = `${kind}:${id}`;
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_SAVED, "readonly");
      const store = tx.objectStore(STORE_SAVED);
      const req = store.get(key);
      req.onsuccess = () => resolve(!!req.result);
      req.onerror = () => reject(req.error);
    });
  }

  async function saveObject(kind, id) {
    const db = await getDB();
    if (!db) return;
    const key = `${kind}:${id}`;
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_SAVED, "readwrite");
      const store = tx.objectStore(STORE_SAVED);
      const req = store.put({ key, kind, id, saved_at: Date.now() });
      req.onsuccess = () => resolve(true);
      req.onerror = () => reject(req.error);
    });
  }

  async function removeSaved(kind, id) {
    const db = await getDB();
    if (!db) return;
    const key = `${kind}:${id}`;
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_SAVED, "readwrite");
      const store = tx.objectStore(STORE_SAVED);
      const req = store.delete(key);
      req.onsuccess = () => resolve(true);
      req.onerror = () => reject(req.error);
    });
  }

  // --- Offline Route Packs ---
  function getPackCacheName(routeId) {
    return `ride-atlas-pack-route-${routeId}`;
  }

  async function getTotalPacksBytes() {
    if (!window.caches) return 0;
    const cacheNames = await caches.keys();
    let total = 0;
    for (const name of cacheNames) {
      if (name.startsWith("ride-atlas-pack-route-")) {
        const cache = await caches.open(name);
        const requests = await cache.keys();
        for (const req of requests) {
          if (req.url.includes("/__pack_version__") || req.url.includes("/data/offline/")) {
            continue;
          }
          const res = await cache.match(req);
          if (res) {
            const blob = await res.clone().blob();
            total += blob.size;
          }
        }
      }
    }
    return total;
  }

  async function getPackStatus(routeId) {
    if (activeDownloads.has(routeId)) {
      const dl = activeDownloads.get(routeId);
      const percent = dl.totalBytes > 0 ? Math.round((dl.bytes / dl.totalBytes) * 100) : 0;
      return {
        status: "downloading",
        downloadedBytes: dl.bytes,
        totalBytes: dl.totalBytes,
        percent,
      };
    }

    if (!window.caches) {
      return { status: "not_downloaded", downloadedBytes: 0, totalBytes: 0, percent: 0 };
    }

    const cacheName = getPackCacheName(routeId);
    const hasCache = await caches.has(cacheName);
    if (!hasCache) {
      return { status: "not_downloaded", downloadedBytes: 0, totalBytes: 0, percent: 0 };
    }

    // Check if all required resources are actually present and up to date
    try {
      const cache = await caches.open(cacheName);
      const manifestUrl = `/data/offline/routes/${routeId}.json`;

      let serverManifest = null;
      try {
        const manifestRes = await fetch(manifestUrl, { cache: "no-store" });
        if (manifestRes && manifestRes.ok) {
          serverManifest = await manifestRes.json();
        }
      } catch (_) {
        serverManifest = null;
      }

      if (serverManifest) {
        const serverResources = serverManifest.resources || [];
        const serverTotalBytes = serverResources.reduce((acc, r) => acc + (r.size_bytes || 0), 0);

        // Check installed pack version
        const cachedManifestRes = await cache.match(manifestUrl);
        const cachedVerRes = await cache.match("/__pack_version__");
        let installedVersion = null;
        if (cachedVerRes) {
          installedVersion = await cachedVerRes.text();
        } else if (cachedManifestRes) {
          const cm = await cachedManifestRes.json();
          installedVersion = cm.pack_version;
        }

        // Legacy packs have no reliable version evidence. Require an explicit refresh
        // online rather than treating same-size resources as current.
        if (!installedVersion || installedVersion !== serverManifest.pack_version) {
          return { status: "needs_download", downloadedBytes: 0, totalBytes: serverTotalBytes, percent: 0 };
        }

        // Verify all required resources and check if any resource size changed
        for (const res of serverResources) {
          const match = await cache.match(res.url);
          if (!match) {
            return { status: "needs_download", downloadedBytes: 0, totalBytes: serverTotalBytes, percent: 0 };
          }
          if (res.size_bytes) {
            const blob = await match.blob();
            if (blob.size !== res.size_bytes) {
              return { status: "needs_download", downloadedBytes: 0, totalBytes: serverTotalBytes, percent: 0 };
            }
          }
        }

        return { status: "downloaded", downloadedBytes: serverTotalBytes, totalBytes: serverTotalBytes, percent: 100 };
      }

      // Offline fallback: server unreachable, verify cached resources
      const cachedManifestRes = await cache.match(manifestUrl);
      if (cachedManifestRes) {
        const cachedManifest = await cachedManifestRes.json();
        const resources = cachedManifest.resources || [];
        const totalBytes = resources.reduce((acc, r) => acc + (r.size_bytes || 0), 0);
        for (const res of resources) {
          const match = await cache.match(res.url);
          if (!match) {
            return { status: "needs_download", downloadedBytes: 0, totalBytes, percent: 0 };
          }
        }
        return { status: "downloaded", downloadedBytes: totalBytes, totalBytes, percent: 100 };
      }

      const keys = await cache.keys();
      if (keys.length > 0) {
        return { status: "downloaded", downloadedBytes: 0, totalBytes: 0, percent: 100 };
      }
      return { status: "not_downloaded", downloadedBytes: 0, totalBytes: 0, percent: 0 };
    } catch (_) {
      return { status: "downloaded", downloadedBytes: 0, totalBytes: 0, percent: 100 };
    }
  }

  async function downloadPack(routeId, onProgress) {
    if (activeDownloads.has(routeId)) return;

    if (!window.caches) {
      throw new Error("CacheStorage is not supported in this browser.");
    }

    // Fetch pack manifest
    const manifestUrl = `/data/offline/routes/${routeId}.json`;
    const manifestRes = await fetch(manifestUrl, { cache: "no-store" });
    if (!manifestRes.ok) {
      throw new Error(`Failed to load pack manifest for route: ${routeId}`);
    }
    const pack = await manifestRes.json();
    const resources = pack.resources || [];

    const packSizeBytes = resources.reduce((acc, r) => acc + (r.size_bytes || 0), 0);
    const currentTotalBytes = await getTotalPacksBytes();

    const cacheName = getPackCacheName(routeId);
    let currentRouteBytes = 0;
    const hasOldCache = await caches.has(cacheName);
    if (hasOldCache) {
      const oldCache = await caches.open(cacheName);
      const oldRequests = await oldCache.keys();
      for (const req of oldRequests) {
        if (req.url.includes("/__pack_version__") || req.url.includes("/data/offline/")) continue;
        const res = await oldCache.match(req);
        if (res) {
          const b = await res.clone().blob();
          currentRouteBytes += b.size;
        }
      }
    }

    if ((currentTotalBytes - currentRouteBytes) + packSizeBytes > MAX_TOTAL_PACKS_BYTES) {
      throw new Error("Managed offline packs cap (100 MB) reached. Please remove an existing pack before downloading.");
    }

    const controller = new AbortController();
    const state = {
      controller,
      bytes: 0,
      totalBytes: packSizeBytes,
    };
    activeDownloads.set(routeId, state);

    const cache = await caches.open(cacheName);

    try {
      // Concurrency limit <= 4
      const CONCURRENCY = 4;
      let currentIndex = 0;

      async function worker() {
        while (currentIndex < resources.length) {
          if (controller.signal.aborted) {
            throw new Error("Download aborted");
          }
          const index = currentIndex++;
          const res = resources[index];

          const response = await fetch(res.url, { signal: controller.signal });
          if (!response.ok) {
            throw new Error(`Failed to fetch resource: ${res.url} (status ${response.status})`);
          }

          // Read stream or blob for accurate byte tracking
          const blob = await response.clone().blob();
          await cache.put(res.url, response);

          state.bytes += blob.size;
          const percent = state.totalBytes > 0 ? Math.min(100, Math.round((state.bytes / state.totalBytes) * 100)) : 100;
          if (onProgress) {
            onProgress({
              routeId,
              bytes: state.bytes,
              totalBytes: state.totalBytes,
              percent,
            });
          }
        }
      }

      const workers = [];
      for (let i = 0; i < Math.min(CONCURRENCY, resources.length); i++) {
        workers.push(worker());
      }
      await Promise.all(workers);

      // Store the manifest and installed pack_version in the pack cache
      await cache.put(
        manifestUrl,
        new Response(JSON.stringify(pack), {
          headers: { "Content-Type": "application/json" }
        })
      );
      await cache.put(
        "/__pack_version__",
        new Response(pack.pack_version || "", {
          headers: { "Content-Type": "text/plain" }
        })
      );

      activeDownloads.delete(routeId);
      return { status: "downloaded", totalBytes: state.totalBytes };
    } catch (err) {
      activeDownloads.delete(routeId);
      // Roll back incomplete pack
      if (window.caches) {
        await caches.delete(cacheName);
      }
      throw err;
    }
  }

  async function cancelDownload(routeId) {
    if (activeDownloads.has(routeId)) {
      const dl = activeDownloads.get(routeId);
      dl.controller.abort();
      activeDownloads.delete(routeId);
      if (window.caches) {
        await caches.delete(getPackCacheName(routeId));
      }
    }
  }

  async function removePack(routeId) {
    cancelDownload(routeId);
    if (window.caches) {
      await caches.delete(getPackCacheName(routeId));
    }
  }

  // --- Service Worker Lifecycle ---
  function initServiceWorker(onUpdate) {
    onUpdateCallback = onUpdate;
    const isLocalhost =
      location.hostname === "localhost" ||
      location.hostname === "127.0.0.1" ||
      location.hostname === "[::1]";

    if (
      (location.protocol === "https:" || isLocalhost) &&
      "serviceWorker" in navigator
    ) {
      navigator.serviceWorker
        .register("/service-worker.js")
        .then((reg) => {
          swRegistration = reg;
          if (!reg) return;

          // Check if an updated worker is already waiting
          if (reg.waiting) {
            updateAvailable = true;
            if (onUpdateCallback) onUpdateCallback();
          }

          reg.addEventListener("updatefound", () => {
            const newWorker = reg.installing;
            if (!newWorker) return;
            newWorker.addEventListener("statechange", () => {
              if (
                newWorker.state === "installed" &&
                navigator.serviceWorker.controller
              ) {
                updateAvailable = true;
                if (onUpdateCallback) onUpdateCallback();
              }
            });
          });
        })
        .catch((err) => {
          console.warn("[PWA] Service worker registration failed:", err);
        });

      let hadController = Boolean(navigator.serviceWorker.controller);
      let refreshing = false;
      navigator.serviceWorker.addEventListener("controllerchange", () => {
        if (hadController && !refreshing) {
          refreshing = true;
          window.location.reload();
        }
      });
    }
  }

  function applyUpdate() {
    if (swRegistration && swRegistration.waiting) {
      swRegistration.waiting.postMessage({ type: "SKIP_WAITING" });
    }
  }

  // Expose global CatalogPWA adapter
  window.CatalogPWA = {
    getSavedKeys,
    isSaved,
    saveObject,
    removeSaved,
    getPackStatus,
    downloadPack,
    cancelDownload,
    removePack,
    getTotalPacksBytes,
    initServiceWorker,
    applyUpdate,
    isUpdateAvailable: () => updateAvailable,
  };
})();
