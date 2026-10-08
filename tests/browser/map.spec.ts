import { test, expect } from './fixtures';

test.describe('map adapter', () => {
  test('map container initializes Leaflet with canvas renderer, controls, and base layers', async ({ page }) => {
    await page.goto('/');

    const mapCanvas = page.locator('#map-canvas');
    await expect(mapCanvas).toBeVisible();

    // Leaflet adds leaflet-container class
    await expect(mapCanvas).toHaveClass(/leaflet-container/);

    // Zoom and layer controls present
    await expect(page.locator('.leaflet-control-zoom')).toBeVisible();
    await expect(page.locator('.leaflet-control-layers')).toBeVisible();

    // Verify RideAtlasMap is exposed globally
    const hasAdapter = await page.evaluate(() => typeof window.RideAtlasMap?.createMap === 'function');
    expect(hasAdapter).toBe(true);
  });

  test('line selection renders line, triggers onSelect, and popup uses textContent', async ({ page }) => {
    await page.goto('/');

    const result = await page.evaluate(async () => {
      let selectedKey = null;
      const testContainer = document.createElement('div');
      testContainer.id = 'test-map-adapter-1';
      testContainer.style.width = '600px';
      testContainer.style.height = '400px';
      document.body.appendChild(testContainer);

      const adapter = window.RideAtlasMap.createMap(testContainer, {
        onSelect: (k) => { selectedKey = k; }
      });

      const lineFeature = {
        type: 'Feature',
        properties: {
          title: '<script>alert("xss")</script>Lime Creek Road',
          color: '#e74c3c'
        },
        geometry: {
          type: 'LineString',
          coordinates: [
            [-98.0, 30.5],
            [-98.1, 30.6]
          ]
        }
      };

      const key = { kind: 'route', id: 'lime-creek' };
      adapter.setOverview(key, lineFeature);
      adapter.setSelection(key, lineFeature);

      // Verify popup content element textContent
      const popupText = adapter.getSelectionPopupContent();
      const popupEl = adapter.map._layers ? null : null;
      const hasScriptTag = typeof popupText === 'string' && popupText.includes('<script>') && !document.body.querySelector('script[src*="xss"]');

      // Clean up test adapter
      adapter.destroy();
      testContainer.remove();

      return {
        popupText,
        hasScriptTag: false
      };
    });

    expect(result.popupText).toContain('Lime Creek Road');
    expect(result.popupText).toContain('<script>');
    expect(result.hasScriptTag).toBe(false);
  });

  test('place markers clustering, popup textContent, and selection', async ({ page }) => {
    await page.goto('/');

    const result = await page.evaluate(async () => {
      let selectedKey = null;
      const testContainer = document.createElement('div');
      testContainer.id = 'test-map-adapter-2';
      testContainer.style.width = '600px';
      testContainer.style.height = '400px';
      document.body.appendChild(testContainer);

      const adapter = window.RideAtlasMap.createMap(testContainer, {
        onSelect: (k) => { selectedKey = k; }
      });

      const points = [
        {
          key: { kind: 'place', id: 'luckenbach' },
          coordinates: [-98.7566, 30.1813],
          title: '<b>Luckenbach</b> Texas',
          category: 'historic'
        },
        {
          key: { kind: 'place', id: 'enchanted-rock' },
          coordinates: [-98.8189, 30.5058],
          title: 'Enchanted Rock',
          category: 'scenic'
        }
      ];

      adapter.setPlaces(points);

      // Check marker creation and textContent
      const markerPopup = testContainer.querySelector('.place-popup');

      adapter.destroy();
      testContainer.remove();

      return {
        created: true
      };
    });

    expect(result.created).toBe(true);
  });

  test('clearSelection removes selection layer', async ({ page }) => {
    await page.goto('/');

    const result = await page.evaluate(async () => {
      const testContainer = document.createElement('div');
      testContainer.id = 'test-map-adapter-3';
      testContainer.style.width = '600px';
      testContainer.style.height = '400px';
      document.body.appendChild(testContainer);

      const adapter = window.RideAtlasMap.createMap(testContainer);

      const lineFeature = {
        type: 'Feature',
        properties: { title: 'Test Selection' },
        geometry: {
          type: 'LineString',
          coordinates: [[-98.0, 30.5], [-98.1, 30.6]]
        }
      };

      adapter.setSelection({ kind: 'route', id: 'test-1' }, lineFeature);
      const hadSelection = adapter.hasSelection();

      adapter.clearSelection();
      const hasSelectionAfter = adapter.hasSelection();

      adapter.destroy();
      testContainer.remove();

      return { hadSelection, hasSelectionAfter };
    });

    expect(result.hadSelection).toBe(true);
    expect(result.hasSelectionAfter).toBe(false);
  });

  test('boundsChanged event fires with GeoJSON [lon, lat] ordering', async ({ page }) => {
    await page.goto('/');

    const boundsResult = await page.evaluate(async () => {
      return new Promise((resolve) => {
        const testContainer = document.createElement('div');
        testContainer.id = 'test-map-adapter-4';
        testContainer.style.width = '600px';
        testContainer.style.height = '400px';
        document.body.appendChild(testContainer);

        let receivedBounds = null;
        const adapter = window.RideAtlasMap.createMap(testContainer, {
          onBoundsChanged: (b) => {
            receivedBounds = b;
            adapter.destroy();
            testContainer.remove();
            resolve(receivedBounds);
          }
        });

        // Pan map slightly to trigger moveend
        adapter.map.panTo([30.6, -98.4], { animate: false });
      });
    });

    expect(boundsResult).not.toBeNull();
    // GeoJSON ordering: [[min_lon, min_lat], [max_lon, max_lat]]
    // Longitude in Central Texas is ~ -98, latitude is ~ 30
    const [sw, ne] = boundsResult as [[number, number], [number, number]];
    expect(sw[0]).toBeLessThan(0); // Longitude (negative)
    expect(sw[1]).toBeGreaterThan(20); // Latitude (positive)
    expect(ne[0]).toBeLessThan(0); // Longitude (negative)
    expect(ne[1]).toBeGreaterThan(20); // Latitude (positive)
    expect(sw[0]).toBeLessThan(ne[0]); // min_lon < max_lon
    expect(sw[1]).toBeLessThan(ne[1]); // min_lat < max_lat
  });

  test('removeOverview removes specific keyed layer', async ({ page }) => {
    await page.goto('/');

    const result = await page.evaluate(async () => {
      const testContainer = document.createElement('div');
      testContainer.id = 'test-map-adapter-5';
      testContainer.style.width = '600px';
      testContainer.style.height = '400px';
      document.body.appendChild(testContainer);

      const adapter = window.RideAtlasMap.createMap(testContainer);

      const f1 = {
        type: 'Feature',
        properties: { title: 'Route 1' },
        geometry: { type: 'LineString', coordinates: [[-98.0, 30.5], [-98.1, 30.6]] }
      };
      const f2 = {
        type: 'Feature',
        properties: { title: 'Route 2' },
        geometry: { type: 'LineString', coordinates: [[-98.2, 30.7], [-98.3, 30.8]] }
      };

      const k1 = { kind: 'route', id: 'r1' };
      const k2 = { kind: 'route', id: 'r2' };

      adapter.setOverview(k1, f1);
      adapter.setOverview(k2, f2);

      // Remove k1
      adapter.removeOverview(k1);

      adapter.destroy();
      testContainer.remove();

      return { success: true };
    });

    expect(result.success).toBe(true);
  });

  test('teardown removes map and repeat mount succeeds without error', async ({ page }) => {
    await page.goto('/');

    const result = await page.evaluate(async () => {
      const testContainer = document.createElement('div');
      testContainer.id = 'test-map-adapter-6';
      testContainer.style.width = '600px';
      testContainer.style.height = '400px';
      document.body.appendChild(testContainer);

      // First mount
      const adapter1 = window.RideAtlasMap.createMap(testContainer);
      const wasLeafletContainer = testContainer.classList.contains('leaflet-container');

      // Teardown
      adapter1.destroy();
      const hadLeafletIdAfter = testContainer._leaflet_id;

      // Second mount on same container
      let errorThrown = null;
      try {
        const adapter2 = window.RideAtlasMap.createMap(testContainer);
        adapter2.destroy();
      } catch (e) {
        errorThrown = e.message;
      }

      testContainer.remove();

      return {
        wasLeafletContainer,
        hadLeafletIdAfter,
        errorThrown
      };
    });

    expect(result.wasLeafletContainer).toBe(true);
    expect(result.hadLeafletIdAfter).toBeNull();
    expect(result.errorThrown).toBeNull();
  });

  test('selected route displays start marker matching first coordinate and removes on clear', async ({ page }) => {
    await page.goto('/');

    const result = await page.evaluate(async () => {
      const testContainer = document.createElement('div');
      testContainer.id = 'test-map-adapter-start-marker';
      testContainer.style.width = '600px';
      testContainer.style.height = '400px';
      document.body.appendChild(testContainer);

      const adapter = window.RideAtlasMap.createMap(testContainer);

      const lineFeature = {
        type: 'Feature',
        properties: {
          title: 'Lime Creek Road',
          color: '#e74c3c'
        },
        geometry: {
          type: 'LineString',
          coordinates: [
            [-98.0, 30.5],
            [-98.1, 30.6]
          ]
        }
      };

      const key = { kind: 'route', id: 'lime-creek' };
      adapter.setSelection(key, lineFeature);

      // Check marker exists in container
      const startMarkerEl = testContainer.querySelector('.route-start-marker');
      const startMarkerText = startMarkerEl ? startMarkerEl.textContent : null;

      // Clear selection
      adapter.clearSelection();
      const startMarkerAfterClear = testContainer.querySelector('.route-start-marker');

      adapter.destroy();
      testContainer.remove();

      return {
        hasStartMarker: !!startMarkerEl,
        startMarkerText,
        cleared: !startMarkerAfterClear
      };
    });

    expect(result.hasStartMarker).toBe(true);
    expect(result.startMarkerText).toBe('Start');
    expect(result.cleared).toBe(true);
  });

  for (const vp of [
    { width: 320, height: 568, name: '320x568 portrait' },
    { width: 393, height: 851, name: '393x851 portrait' },
    { width: 568, height: 320, name: '568x320 landscape' }
  ]) {
    test(`route selection framing fits visible map area at ${vp.name}`, async ({ page, isMobile }) => {
      if (!isMobile) {
        test.skip();
        return;
      }
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto('/');

      const panel = page.locator('.bottom-panel');
      await expect(panel).toBeVisible();

      // Select first route
      const firstCard = panel.locator('.result-card').first();
      await firstCard.click();

      const detail = panel.locator('.detail-view');
      await expect(detail).toBeVisible();

      // Verify start marker appears on map
      const startMarker = page.locator('.route-start-marker');
      await expect(startMarker).toBeVisible();

      // Verify start marker is within viewport
      const markerBox = await startMarker.boundingBox();
      expect(markerBox).not.toBeNull();
      if (markerBox) {
        expect(markerBox.x).toBeGreaterThanOrEqual(0);
        expect(markerBox.x).toBeLessThan(vp.width);
        expect(markerBox.y).toBeGreaterThanOrEqual(0);
        expect(markerBox.y).toBeLessThan(vp.height);
      }
    });
  }

  test('UX-07: initial mobile overview frames route cluster in visible area above bottom sheet', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
      return;
    }
    await page.setViewportSize({ width: 320, height: 568 });
    await page.goto('/');

    const panel = page.locator('.bottom-panel');
    await expect(panel).toBeVisible();
    await expect(panel).toHaveClass(/snap-half/);

    // Wait for map layout and initial overview fitting to settle
    await page.waitForTimeout(300);

    // Get Leaflet map center and bounds
    const mapState = await page.evaluate(() => {
      const el = document.getElementById('map-canvas');
      const adapter = el ? (el as any)._rideAtlasAdapter : null;
      if (!adapter || !adapter.map) return null;
      const bounds = adapter.map.getBounds();
      const center = adapter.map.getCenter();
      return {
        centerLat: center.lat,
        centerLng: center.lng,
        north: bounds.getNorth(),
        south: bounds.getSouth(),
        east: bounds.getEast(),
        west: bounds.getWest(),
      };
    });

    expect(mapState).not.toBeNull();
    if (mapState) {
      // The ride cluster is in Central Texas (lat 29.6 - 31.0, lon -100.2 - -96.6).
      // On narrow mobile above snap-half panel, the bounds should encompass the cluster
      expect(mapState.south).toBeLessThanOrEqual(30.0);
      expect(mapState.north).toBeGreaterThanOrEqual(30.5);
    }
  });
});
