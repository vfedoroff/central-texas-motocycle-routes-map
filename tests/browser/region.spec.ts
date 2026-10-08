import { test, expect } from './fixtures';
import { readFileSync } from 'node:fs';
import type { Page } from '@playwright/test';

function getPanel(page: Page, isMobile: boolean) {
  return page.locator(isMobile ? '.bottom-panel' : '.sidebar');
}

test.describe('region loading', () => {
  test('first overview requests match the actual initial viewport', async ({ page }) => {
    const requested = new Set<string>();
    await page.route('**/data/overview/**', async route => {
      requested.add(new URL(route.request().url()).pathname);
      await route.continue();
    });
    await page.goto('/');
    await expect(page.locator('#map-canvas')).toBeVisible();
    const bounds = await page.evaluate(() => {
      const b = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map.getBounds();
      return [b.getWest(), b.getSouth(), b.getEast(), b.getNorth()];
    });
    const catalog = JSON.parse(readFileSync('dist/catalog-index.json', 'utf8'));
    const expected = catalog.objects.filter((o: any) => o.key.kind === 'route' && o.overview_url &&
      o.bounds[0] <= bounds[2] && o.bounds[2] >= bounds[0] &&
      o.bounds[1] <= bounds[3] && o.bounds[3] >= bounds[1]).map((o: any) => o.overview_url).sort();
    expect(expected.length).toBeGreaterThan(0);
    await expect.poll(() => Array.from(requested).sort()).toEqual(expected);
    await expect(page.locator('.btn-search-area:visible')).toHaveCount(0);
  });

  test('bounds not applied to list before clicking search this area', async ({ page, isMobile }) => {
    await page.goto('/');

    const panel = getPanel(page, isMobile);
    // Initial results list shows all 26 routes
    await expect(panel.locator('.results-list')).toBeVisible();
    await expect(panel.locator('.result-item').first()).toBeVisible();
    const initialCount = await panel.locator('.result-item').count();
    expect(initialCount).toBeGreaterThan(15);

    // Pan map to small area to trigger pending bounds
    await page.evaluate(() => {
      const container = document.getElementById('map-canvas');
      if (container && (container as any)._rideAtlasAdapter) {
        (container as any)._rideAtlasAdapter.map.setView([30.0, -97.0], 12);
      }
    });

    // "Search this area" button appears
    const searchAreaBtn = panel.locator('.btn-search-area');
    await expect(searchAreaBtn).toBeVisible();

    // Results count should remain unchanged before clicking the button
    const countBeforeClick = await panel.locator('.result-item').count();
    expect(countBeforeClick).toBe(initialCount);

    // Click "Search this area"
    await searchAreaBtn.click();

    // Results should now be filtered to that area (less than full catalog)
    await expect.poll(async () => await panel.locator('.result-item').count()).toBeLessThan(initialCount);
  });

  test('overview requests cap at four concurrent in flight and full geometry is absent', async ({ page }) => {
    let activeOverviewRequests = 0;
    let maxConcurrentSeen = 0;
    const requestedUrls: string[] = [];

    await page.route('**/data/overview/**', async (route) => {
      activeOverviewRequests++;
      if (activeOverviewRequests > maxConcurrentSeen) {
        maxConcurrentSeen = activeOverviewRequests;
      }
      requestedUrls.push(route.request().url());

      // Artificial delay to observe concurrency
      await new Promise((resolve) => setTimeout(resolve, 80));
      activeOverviewRequests--;
      await route.continue();
    });

    page.on('request', (req) => {
      requestedUrls.push(req.url());
    });

    await page.goto('/');

    // Wait for initial batch of overviews to settle
    await page.waitForTimeout(600);

    // Verify 4-request concurrency cap
    expect(maxConcurrentSeen).toBeLessThanOrEqual(4);

    // Verify no full geometry was requested during normal region browsing
    const hasFullGeometry = requestedUrls.some((u) => u.includes('/data/geometry/'));
    expect(hasFullGeometry).toBe(false);
  });

  test('failed geometry shows nonblocking warning and recovers on retry', async ({ page, isMobile }) => {
    let failOverview = true;
    await page.route('**/data/overview/**', async (route) => {
      if (failOverview) {
        await route.fulfill({ status: 500, body: 'Internal Server Error' });
      } else {
        await route.continue();
      }
    });

    await page.goto('/');

    // Warning banner appears
    const warningBanner = page.locator('.geometry-warning-banner');
    await expect(warningBanner).toBeVisible({ timeout: 5000 });
    await expect(warningBanner).toContainText('Failed to load geometry');

    // List is still usable in active panel
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Allow overviews to succeed and click Retry
    failOverview = false;
    const retryBtn = warningBanner.locator('.btn-retry-geometry');
    await retryBtn.click();

    // Warning banner disappears
    await expect(warningBanner).not.toBeVisible({ timeout: 5000 });
  });

  test('visible overview layers stay capped at 200 and leave the map when offscreen', async ({ page }) => {
    await page.route('**/catalog-index.json', async route => {
      const response = await route.fetch();
      const catalog = await response.json();
      const template = catalog.objects[0];
      catalog.objects = Array.from({ length: 205 }, (_, i) => ({
        ...template, key: { ...template.key, id: `region-fixture-${i}` }, title: `Region fixture ${i}`,
      }));
      await route.fulfill({ response, json: catalog });
    });
    await page.goto('/');
    const countLines = () => page.evaluate(() => {
      let count = 0;
      (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map.eachLayer((layer: any) => {
        if (layer.feature?.geometry?.type === 'LineString') count++;
      });
      return count;
    });
    await expect(page.getByText('Showing 200 roads; zoom in for more')).toBeVisible();
    await expect.poll(countLines, { timeout: 15000 }).toBe(200);
    await page.evaluate(() => {
      (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map.setView([0, 0], 12);
    });
    await expect.poll(countLines).toBe(0);
    await expect(page.getByText('Showing 200 roads; zoom in for more')).not.toBeVisible();
  });

  test('current map geometry loads after four stale requests release their slots', async ({ page, isMobile }) => {
    const releases: Array<() => void> = [];
    let intercepted = 0;
    await page.route('**/data/overview/**', async route => {
      if (++intercepted <= 4) {
        await new Promise<void>(resolve => releases.push(resolve));
      }
      await route.continue();
    });
    await page.goto('/');
    await expect.poll(() => releases.length).toBe(4);
    const panel = getPanel(page, isMobile);
    await panel.locator('.input-search').fill('austin llano');
    await expect(panel.locator('.result-item')).toHaveCount(1);
    releases.forEach(release => release());
    await expect.poll(() => page.evaluate(() => {
      const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
      const geometries: unknown[] = [];
      map.eachLayer((layer: any) => {
        if (layer.feature?.geometry?.type === 'LineString') geometries.push(layer.feature.geometry);
      });
      return geometries;
    })).toEqual([JSON.parse(readFileSync('dist/data/overview/routes/austin-to-llano-hill-country-blast.geojson', 'utf8')).geometry]);
  });

  test('stale response from previous generation is ignored', async ({ page, isMobile }) => {
    let slowRequestUrl: string | null = null;
    let resolveSlowRequest: (() => void) | null = null;

    await page.route('**/data/overview/**', async (route) => {
      // Delay first route overview until after filter changes
      if (!slowRequestUrl) {
        slowRequestUrl = route.request().url();
        await new Promise<void>((resolve) => {
          resolveSlowRequest = resolve;
        });
      }
      await route.continue();
    });

    await page.goto('/');

    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Search for a specific query to change filter generation
    const searchInput = panel.locator('.input-search');
    await searchInput.fill('twisted sisters');
    await expect(panel.locator('.results-list .result-item')).toHaveCount(1);
    await expect(panel.locator('.card-title')).toContainText('Twisted Sisters');

    // Now let the delayed request finish
    if (resolveSlowRequest) {
      resolveSlowRequest();
    }
    await page.waitForTimeout(300);

    // Results continue to reflect twisted sisters and are not overwritten by stale data
    await expect(panel.locator('.results-list .result-item')).toHaveCount(1);
    const results = panel.locator('.card-title').first();
    await expect(results).toContainText('Twisted Sisters');
  });
});
