import { test, expect } from './fixtures';
import { readFileSync } from 'node:fs';

const id = 'lime-creek-road-lake-travis-loop';
const title = 'Lime Creek Road & Lake Travis Loop';
const geometry = JSON.parse(readFileSync(`dist/data/overview/routes/${id}.geojson`, 'utf8')).geometry;

test('route under the location marker remains selectable after clearing filters', async ({ context, page, isMobile }) => {
  const location = { latitude: 30.5788, longitude: -97.8531, accuracy: 100 };
  await context.grantPermissions(['geolocation']);
  await context.setGeolocation(location);
  await page.goto('/');
  const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
  await expect(panel.locator('.results-list')).toBeVisible();
  if (isMobile) await panel.locator('.btn-toggle-filters').click();
  await panel.locator('.btn-near-me').click();
  await expect(panel.locator('.btn-near-me')).toHaveClass(/is-active/);
  await panel.locator('.cat-pill', { hasText: 'Twisties & Canyons' }).click();
  await panel.locator('.btn-clear-filters').first().click();
  await expect(panel.locator('.btn-near-me')).not.toHaveClass(/is-active/);
  if (isMobile) {
    await panel.locator('.btn-toggle-filters').click();
    await page.getByRole('button', { name: 'Show map', exact: true }).click();
    // The sheet schedules map resizing through 500ms after its transition.
    await page.waitForTimeout(550);
  }
  await page.evaluate(({ latitude, longitude }) => {
    const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
    map.setView([latitude, longitude], 11, { animate: false });
  }, location);
  await expect.poll(() => page.evaluate(() => {
    const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
    let found = false;
    map.eachLayer((layer: any) => {
      if (layer.feature?.properties?.title === 'Leander to Luckenbach Loop') found = true;
    });
    return found;
  })).toBe(true);
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
  const marker = page.locator('.user-pulse-marker');
  await expect(marker).toBeVisible();
  // Wait for both location zooming and the mobile sheet resize to settle.
  await expect.poll(() => page.evaluate(({ latitude, longitude }) => {
    const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
    const point = map.latLngToContainerPoint([latitude, longitude]);
    const rect = map.getContainer().getBoundingClientRect();
    const pin = document.querySelector('.user-pulse-marker')!.getBoundingClientRect();
    return Math.hypot(pin.left + pin.width / 2 - rect.left - point.x,
      pin.top + pin.height / 2 - rect.top - point.y);
  }, location)).toBeLessThan(1);
  const box = await marker.boundingBox();
  if (!box) throw new Error('Location marker is missing');
  if (isMobile) await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
  else await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  // Several routes share this crossing; any selected route must open details.
  await expect(panel.locator('.detail-title')).toBeVisible();
  await expect(page).toHaveURL(/kind=route&id=/);
});

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

test('selecting a nearby route from Places hides other route lines', async ({ page, isMobile }) => {
  await page.goto('/');
  const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
  await panel.getByRole('tab', { name: 'Places', exact: true }).click();
  await panel.getByRole('searchbox', { name: 'Search catalog' }).fill('spur');
  if (isMobile) await panel.getByRole('button', { name: /Filters/ }).click();
  await panel.getByRole('checkbox', { name: 'Show nearby routes' }).check();
  if (isMobile) {
    await panel.getByRole('button', { name: /Filters/ }).click();
    await page.getByRole('button', { name: 'Show map', exact: true }).click();
    await page.waitForTimeout(550);
  }
  const lineCount = () => page.evaluate(() => {
    let count = 0;
    (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map.eachLayer((layer: any) => {
      if (layer.feature?.geometry?.type === 'LineString' && layer.options.weight === 3.5) count++;
    });
    return count;
  });
  await expect.poll(lineCount).toBeGreaterThan(0);
  await expect.poll(() => page.evaluate(() => {
    let found = false;
    (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map.eachLayer((layer: any) => {
      if (layer.feature?.properties?.title === 'Triangle' && layer.options.weight === 3.5) found = true;
    });
    return found;
  })).toBe(true);
  const point = await page.evaluate(() => {
    const map = (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map;
    let coords: number[] | undefined;
    map.eachLayer((layer: any) => {
      if (layer.feature?.properties?.title === 'Triangle' && layer.options.weight === 3.5) {
        const points = layer.feature.geometry.coordinates;
        coords = points[Math.floor(points.length / 4)];
      }
    });
    if (!coords) throw new Error('Triangle overview missing');
    map.setView([coords[1], coords[0]], 14, { animate: false });
    const p = map.latLngToContainerPoint([coords[1], coords[0]]);
    const rect = map.getContainer().getBoundingClientRect();
    return { x: rect.left + p.x, y: rect.top + p.y };
  });
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
  if (isMobile) await page.touchscreen.tap(point.x, point.y);
  else await page.mouse.click(point.x, point.y);
  await expect(panel.locator('.detail-title')).toHaveText('Triangle');
  await expect.poll(lineCount).toBe(0);
});
