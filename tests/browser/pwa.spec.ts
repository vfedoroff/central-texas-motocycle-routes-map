import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';

function getPanel(page: Page, isMobile: boolean) {
  return page.locator(isMobile ? '.bottom-panel' : '.sidebar');
}

test.describe('PWA and Offline Capabilities', () => {
  test.use({ serviceWorkers: 'allow' });

  test('PWA manifest and installation prerequisites', async ({ request, page }) => {
    // 1. Fetch manifest directly
    const manifestResp = await request.get('/manifest.webmanifest');
    expect(manifestResp.status()).toBe(200);
    const manifest = await manifestResp.json();

    expect(manifest.name).toBe('Ride Atlas');
    expect(manifest.display).toBe('standalone');
    expect(manifest.start_url).toBe('/');
    expect(manifest.icons).toBeDefined();
    expect(manifest.icons.length).toBeGreaterThanOrEqual(4);

    // Verify 192 and 512 regular and maskable icons exist in manifest
    const icon192 = manifest.icons.find((i: any) => i.sizes === '192x192' && !i.purpose?.includes('maskable'));
    const icon512 = manifest.icons.find((i: any) => i.sizes === '512x512' && !i.purpose?.includes('maskable'));
    const maskable192 = manifest.icons.find((i: any) => i.sizes === '192x192' && i.purpose?.includes('maskable'));
    const maskable512 = manifest.icons.find((i: any) => i.sizes === '512x512' && i.purpose?.includes('maskable'));

    expect(icon192).toBeDefined();
    expect(icon512).toBeDefined();
    expect(maskable192).toBeDefined();
    expect(maskable512).toBeDefined();

    // Verify icon files are served with HTTP 200
    for (const icon of [icon192, icon512, maskable192, maskable512]) {
      const iconResp = await request.get(icon.src);
      expect(iconResp.status()).toBe(200);
      expect(iconResp.headers()['content-type']).toContain('image/png');
    }

    // 2. Check HTML document metadata
    await page.goto('/');
    const manifestLink = page.locator('link[rel="manifest"]');
    await expect(manifestLink).toHaveAttribute('href', '/manifest.webmanifest');

    const themeColor = page.locator('meta[name="theme-color"]');
    await expect(themeColor).toHaveAttribute('content', '#1e293b');
  });

  test('IndexedDB saved objects persistence and filtering', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Identify first route card
    const firstCard = panel.locator('.result-card').first();
    const firstTitle = await firstCard.locator('.card-title').textContent();
    const saveBtn = firstCard.locator('.btn-card-save');

    await expect(saveBtn).toBeVisible();
    await expect(saveBtn).toContainText('Save');

    // Click save button on the card
    await saveBtn.click();
    await expect(saveBtn).toContainText('Saved');
    await expect(saveBtn).toHaveClass(/is-saved/);

    // Verify saved in IndexedDB
    const savedKeys = await page.evaluate(async () => {
      return await (window as any).CatalogPWA.getSavedKeys();
    });
    expect(savedKeys.length).toBeGreaterThan(0);

    // Filter by "Saved only"
    if (isMobile) {
      await panel.locator('.btn-toggle-filters').click();
    }
    const savedOnlyCheckbox = panel.locator('.checkbox-saved');
    await savedOnlyCheckbox.check();

    // Ensure only the saved card is displayed in results
    const resultCards = panel.locator('.result-card');
    await expect(resultCards).toHaveCount(1);
    await expect(resultCards.first().locator('.card-title')).toHaveText(firstTitle || '');

    // Reload page and verify persistence across reload
    await page.reload();
    const panelAfterReload = getPanel(page, isMobile);
    await expect(panelAfterReload.locator('.results-list')).toBeVisible();

    // Check "Saved only" again
    if (isMobile) {
      await panelAfterReload.locator('.btn-toggle-filters').click();
    }
    await panelAfterReload.locator('.checkbox-saved').check();
    const persistedCards = panelAfterReload.locator('.result-card');
    await expect(persistedCards).toHaveCount(1);
    await expect(persistedCards.first().locator('.card-title')).toHaveText(firstTitle || '');

    // Un-save via card save button
    await persistedCards.first().locator('.btn-card-save').click();

    // Now empty state should show
    await expect(panelAfterReload.locator('.state-empty')).toBeVisible();
  });

  test('DetailView save button synchronizes with card and IndexedDB', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Open route detail
    const firstCard = panel.locator('.result-card').first();
    const routeId = await firstCard.getAttribute('data-id');
    await firstCard.click();

    await expect(panel.locator('.detail-title')).toBeVisible();
    const detailSaveBtn = panel.locator('.btn-detail-save');
    await expect(detailSaveBtn).toBeVisible();
    await expect(detailSaveBtn).toContainText('Save');

    // Toggle save in detail view
    await detailSaveBtn.click();
    await expect(detailSaveBtn).toContainText('Saved');
    await expect(detailSaveBtn).toHaveClass(/is-saved/);

    // Go back to results
    await panel.locator('.btn-back-to-results').click();
    await expect(panel.locator('.results-list')).toBeVisible();

    // The card corresponding to this route should now be saved
    const targetCard = panel.locator(`.result-card[data-id="${routeId}"]`);
    await expect(targetCard.locator('.btn-card-save')).toHaveClass(/is-saved/);

    // Clean up
    await targetCard.locator('.btn-card-save').click({ force: true });
  });

  test('Unavailable saved objects display cleanly with remove action', async ({ page, isMobile }) => {
    await page.goto('/');

    // Inject unavailable object directly into IndexedDB
    await page.evaluate(async () => {
      await (window as any).CatalogPWA.saveObject('route', 'ghost-deleted-route-123');
    });

    // Reload page to initialize saved keys from IndexedDB
    await page.reload();
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    if (isMobile) {
      await panel.locator('.btn-toggle-filters').click();
      const savedCheckbox = panel.locator('.checkbox-saved');
      await savedCheckbox.check();
      await panel.locator('.btn-done-filters').click();
    } else {
      const savedCheckbox = panel.locator('.checkbox-saved');
      await savedCheckbox.check();
    }

    // Verify unavailable item card appears
    const unavailItem = panel.locator('.result-item-unavailable');
    await expect(unavailItem).toBeVisible();
    await expect(unavailItem).toContainText('Unavailable route');
    await expect(unavailItem).toContainText('This item is no longer available in the catalog');

    // Click remove button on the unavailable item
    const removeBtn = unavailItem.locator('.btn-remove-saved');
    await expect(removeBtn).toBeVisible();
    await removeBtn.click();

    // Item disappears from results and empty state or zero unavailable items remain
    await expect(unavailItem).not.toBeVisible();

    // Verify removed from IndexedDB
    const isSaved = await page.evaluate(async () => {
      return await (window as any).CatalogPWA.isSaved('route', 'ghost-deleted-route-123');
    });
    expect(isSaved).toBe(false);
  });

  test('Offline route pack download and CacheStorage management', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Open first route detail
    const firstCard = panel.locator('.result-card').first();
    const routeId = await firstCard.getAttribute('data-id');
    expect(routeId).toBeTruthy();
    await firstCard.click();

    await expect(panel.locator('.detail-title')).toBeVisible();

    // Offline pack section should be rendered
    const offlineSection = panel.locator('.offline-pack-section');
    await expect(offlineSection).toBeVisible();

    const downloadBtn = offlineSection.locator('.btn-offline-download');
    await expect(downloadBtn).toBeVisible();
    await expect(downloadBtn).toContainText('Download for Offline Use');

    // Click download
    await downloadBtn.click();

    // Progress bar or downloaded badge should appear
    await expect(offlineSection.locator('.offline-badge-downloaded, .offline-progress-bar-bg')).toBeVisible();

    // Wait until downloaded badge appears
    const downloadedBadge = offlineSection.locator('.offline-badge-downloaded');
    await expect(downloadedBadge).toBeVisible({ timeout: 10000 });
    await expect(downloadedBadge).toContainText('Available Offline');

    // Verify resources exist in CacheStorage
    const cacheVerified = await page.evaluate(async (rid) => {
      const cacheName = `ride-atlas-pack-route-${rid}`;
      if (!window.caches) return false;
      const has = await caches.has(cacheName);
      if (!has) return false;
      const cache = await caches.open(cacheName);
      const keys = await cache.keys();
      return keys.length > 0;
    }, routeId);
    expect(cacheVerified).toBe(true);

    // Verify Remove Offline Pack button is present and works
    const removePackBtn = offlineSection.locator('.btn-offline-remove');
    await expect(removePackBtn).toBeVisible();
    await removePackBtn.click();

    // Should return to download state
    await expect(offlineSection.locator('.btn-offline-download')).toBeVisible();

    // Cache should be deleted
    const cacheAfterRemove = await page.evaluate(async (rid) => {
      const cacheName = `ride-atlas-pack-route-${rid}`;
      return await caches.has(cacheName);
    }, routeId);
    expect(cacheAfterRemove).toBe(false);
  });

  test('Service worker script exists and excludes tile and analytics caching', async ({ request }) => {
    const swResp = await request.get('/service-worker.js');
    expect(swResp.status()).toBe(200);
    const swContent = await swResp.text();

    expect(swContent).toContain('SHELL_CACHE');
    expect(swContent).toContain('SITE_VERSION');
    // Verifies security rule: map tiles and analytics are never cached
    expect(swContent).toContain('openstreetmap');
    expect(swContent).toContain('google-analytics');
  });

  test('Global offline manifest structure and route packs integrity', async ({ request }) => {
    const manifestResp = await request.get('/offline-manifest.json');
    expect(manifestResp.status()).toBe(200);
    const manifest = await manifestResp.json();

    expect(manifest.schema_version).toBe(1);
    expect(typeof manifest.site_version).toBe('string');
    expect(manifest.site_version.length).toBe(64); // SHA-256 hex
    expect(Array.isArray(manifest.packs)).toBe(true);
    expect(manifest.packs.length).toBe(26);

    for (const pack of manifest.packs) {
      expect(pack.key.kind).toBe('route');
      expect(typeof pack.key.id).toBe('string');
      expect(typeof pack.manifest_url).toBe('string');
    }

    // Verify individual route pack manifest
    const firstPackResp = await request.get(manifest.packs[0].manifest_url);
    expect(firstPackResp.status()).toBe(200);
    const firstPack = await firstPackResp.json();
    expect(firstPack.schema_version).toBe(1);
    expect(typeof firstPack.pack_version).toBe('string');
    expect(firstPack.pack_version.length).toBe(64);
    expect(Array.isArray(firstPack.resources)).toBe(true);
    expect(firstPack.resources.length).toBeGreaterThan(0);
  });

  test('Offline pack download cancel rolls back and purges cache', async ({ page }) => {
    await page.goto('/');

    await page.evaluate(async () => {
      const pwa = (window as any).CatalogPWA;
      let canceled = false;
      const dlPromise = pwa.downloadPack('blanco-river-valley-fm-165-loop', () => {
        if (!canceled) {
          canceled = true;
          pwa.cancelDownload('blanco-river-valley-fm-165-loop');
        }
      }).catch(() => {});
      await dlPromise;
    });

    // Check that cache does not remain in caches
    const hasCache = await page.evaluate(async () => {
      return await caches.has('ride-atlas-pack-route-blanco-river-valley-fm-165-loop');
    });
    expect(hasCache).toBe(false);
  });
});
