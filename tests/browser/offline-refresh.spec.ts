import { test, expect } from './fixtures';

// Isolate adapter/UI behavior from service worker routing; real SW flow is checked manually.
test.use({ serviceWorkers: 'block' });
const id = 'lime-creek-road-lake-travis-loop';
const manifestUrl = `/data/offline/routes/${id}.json`;

test('legacy pack without version metadata requires refresh even for same-size content', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('searchbox', { name: 'Search catalog' })).toBeVisible();
  await page.evaluate(async ({ id, manifestUrl }) => {
    const manifest = await (await fetch(manifestUrl)).json();
    const cache = await caches.open(`ride-atlas-pack-route-${id}`);
    for (const resource of manifest.resources) {
      await cache.put(resource.url, await fetch(resource.url));
    }
  }, { id, manifestUrl });
  const status = await page.evaluate(async (id) => (window as any).CatalogPWA.getPackStatus(id), id);
  expect(status.status).toBe('needs_download');
  await page.evaluate(async (id) => (window as any).CatalogPWA.downloadPack(id, () => {}), id);
  expect((await page.evaluate(async (id) => (window as any).CatalogPWA.getPackStatus(id), id)).status).toBe('downloaded');
});

test('outdated legacy packs do not add offline controls to route details', async ({ page, isMobile }) => {
  await page.goto('/');
  await expect(page.getByRole('searchbox', { name: 'Search catalog' })).toBeVisible();
  await page.evaluate(async (id) => (window as any).CatalogPWA.downloadPack(id, () => {}), id);
  await page.route(manifestUrl, async route => {
    const response = await route.fetch();
    const manifest = await response.json();
    manifest.pack_version = 'changed-release';
    await route.fulfill({ json: manifest });
  });
  await page.getByRole('searchbox', { name: 'Search catalog' }).fill('lime');
  const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
  await panel.locator(`.result-card[data-id="${id}"]`).click();
  await expect(panel.locator('.detail-title')).toBeVisible();
  await expect(panel.locator('.offline-pack-section')).toHaveCount(0);
  await expect(panel.getByRole('button', { name: /offline pack/i })).toHaveCount(0);
  await panel.locator('.export-route-summary').click();
  await expect(panel.locator('.detail-gpx-download')).toBeVisible();
});
