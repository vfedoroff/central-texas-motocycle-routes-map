import { test, expect } from './fixtures';

test('opening view stays in Central Texas and explains route colors', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#map-canvas')).toHaveClass(/leaflet-container/);
  const view = await page.evaluate(() => {
    const map = document.querySelector('#map-canvas')._rideAtlasAdapter.map;
    return { zoom: map.getZoom(), center: map.getCenter() };
  });
  expect(view.zoom).toBeGreaterThanOrEqual(8);
  expect(view.center.lat).toBeGreaterThan(29);
  expect(view.center.lat).toBeLessThan(32);
  await expect(page.locator('#map-canvas .map-key')).toContainText('Colors identify rides');
  await expect(page.locator('#map-canvas .leaflet-top.leaflet-right .leaflet-control-attribution')).toBeVisible();
  const manualZoom = await page.evaluate(() => {
    const map = document.querySelector('#map-canvas')._rideAtlasAdapter.map;
    map.setZoom(6, { animate: false });
    return map.getZoom();
  });
  expect(manualZoom).toBe(6);
});

test('hover and selection identify a ride while other lines recede', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#map-canvas')).toHaveClass(/leaflet-container/);
  const result = await page.evaluate(() => {
    const container = document.createElement('div');
    container.style.cssText = 'width:600px;height:500px';
    document.body.appendChild(container);
    const adapter = window.RideAtlasMap.createMap(container);
    const key = { kind: 'route', id: 'readability' };
    const feature = { type: 'Feature', properties: { title: '<b>Test ride</b>', color: '#C03F84' }, geometry: { type: 'LineString', coordinates: [[-98,30.5],[-98.1,30.6]] } };
    adapter.setOverview(key, feature);
    let line;
    adapter.map.eachLayer(layer => { if (layer instanceof window.L.Polyline) line = layer; });
    const normal = line.options.opacity;
    line.fire('mouseover', {}, true);
    const hover = { weight: line.options.weight, label: container.querySelector('.route-name-label')?.textContent };
    line.fire('mouseout', {}, true);
    adapter.setSelection(key, feature);
    const selected = { opacity: line.options.opacity, label: container.querySelector('.route-name-label')?.textContent, markup: !!container.querySelector('.route-name-label b') };
    adapter.clearSelection();
    const restored = line.options.opacity;
    adapter.destroy(); container.remove();
    return { normal, hover, selected, restored };
  });
  expect(result.normal).toBeLessThan(0.7);
  expect(result.hover.weight).toBeGreaterThan(3);
  expect(result.hover.label).toBe('<b>Test ride</b>');
  expect(result.selected.opacity).toBeLessThan(result.normal);
  expect(result.selected.label).toBe('<b>Test ride</b>');
  expect(result.selected.markup).toBe(false);
  expect(result.restored).toBe(result.normal);
});

test('slow tiles show loading feedback', async ({ page }) => {
  await page.route('**/MapServer/tile/**', route => route.abort());
  await page.goto('/');
  await expect(page.locator('#map-canvas')).toHaveClass(/leaflet-container/);
  await expect(page.locator('.map-tile-status')).toBeVisible();
});
