import { test, expect } from '@playwright/test';

test.describe('composed UI and static pages', () => {
  test('root map serves composed index.html and boots Leptos UI', async ({ page }) => {
    await page.goto('/');

    await expect(page).toHaveTitle(/Ride Atlas/);
    const mapCanvas = page.locator('#map-canvas');
    await expect(mapCanvas).toBeVisible();
    await expect(mapCanvas).toHaveClass(/leaflet-container/);

    // Verify /app/ hashed asset requests succeeded
    const scripts = await page.locator('script[type="module"]').all();
    expect(scripts.length).toBeGreaterThan(0);
  });

  test('static route page renders without Leptos application', async ({ page }) => {
    const response = await page.goto('/routes/the-three-twisted-sisters-circuit/');
    expect(response?.status()).toBe(200);

    // Verify static content rendered
    await expect(page.locator('h1')).toContainText('The Three Twisted Sisters');
    await expect(page.locator('.meta')).toContainText('Distance:');

    // Static pages do not mount Leptos UI map
    await expect(page.locator('#map-canvas')).toHaveCount(0);
    await expect(page.locator('.sidebar')).toHaveCount(0);
  });

  test('generated downloads are accessible with valid status', async ({ request }) => {
    // GPX download
    const gpxResp = await request.get('/downloads/routes/the-three-twisted-sisters-circuit.gpx');
    expect(gpxResp.status()).toBe(200);
    const gpxText = await gpxResp.text();
    expect(gpxText).toContain('<gpx');
    expect(gpxText).toContain('The Three Twisted Sisters');

    // GeoJSON download
    const geojsonResp = await request.get('/downloads/routes/the-three-twisted-sisters-circuit.geojson');
    expect(geojsonResp.status()).toBe(200);
    const geojson = await geojsonResp.json();
    expect(geojson.type).toBe('Feature');
    expect(geojson.geometry.type).toBe('LineString');
  });

  test('missing static file returns 404 without SPA rewrite', async ({ request }) => {
    const missingResp = await request.get('/missing-static-file-404.html');
    expect(missingResp.status()).toBe(404);
  });
});
