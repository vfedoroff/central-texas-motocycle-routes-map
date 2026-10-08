import { test, expect } from './fixtures';
import { readFileSync } from 'node:fs';

const id = 'lime-creek-road-lake-travis-loop';
const title = 'Lime Creek Road & Lake Travis Loop';
const geometry = JSON.parse(readFileSync(`dist/data/overview/routes/${id}.geojson`, 'utf8')).geometry;

for (const offset of [0, 8]) {
  test(`route line opens details when clicked ${offset}px from its stroke`, async ({ page, isMobile }) => {
    await page.goto('/');
    await page.getByRole('searchbox', { name: 'Search catalog' }).fill(title);
    await expect(page.locator(isMobile ? '.bottom-panel' : '.sidebar').locator(`.result-card[data-id="${id}"]`)).toBeVisible();
    if (isMobile) {
      await page.getByRole('button', { name: 'Show map', exact: true }).click();
      // The bottom sheet animates for 300ms and invalidates the map size.
      await page.waitForTimeout(400);
    }
    const a = geometry.coordinates[0], b = geometry.coordinates[1];
    await page.evaluate(({ a, b }) => {
      const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
      map.setView([(a[1] + b[1]) / 2, (a[0] + b[0]) / 2], 14, { animate: false });
    }, { a, b });
    await expect.poll(() => page.evaluate(() => {
      const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
      let count = 0;
      map.eachLayer((layer: any) => { if (layer.feature?.geometry?.type === 'LineString' && layer.options.weight === 3.5) count++; });
      return count;
    })).toBeGreaterThan(0);
    // Leaflet schedules canvas painting after adding overview geometry.
    await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
    const point = await page.evaluate(({ a, b, offset }) => {
      const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
      const p = map.latLngToContainerPoint([a[1], a[0]]);
      const q = map.latLngToContainerPoint([b[1], b[0]]);
      const dx = q.x - p.x, dy = q.y - p.y, length = Math.hypot(dx, dy);
      const rect = map.getContainer().getBoundingClientRect();
      return { x: rect.left + (p.x + q.x) / 2 - dy / length * offset,
        y: rect.top + (p.y + q.y) / 2 + dx / length * offset };
    }, { a, b, offset });
    if (isMobile) await page.touchscreen.tap(point.x, point.y);
    else await page.mouse.click(point.x, point.y);
    const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
    await expect(panel.locator('.detail-title')).toHaveText(title);
  });
}
