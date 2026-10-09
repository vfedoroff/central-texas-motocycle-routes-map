import { test, expect } from '@playwright/test';

function getPanel(page: any, isMobile: boolean) {
  return page.locator(isMobile ? '.bottom-panel' : '.sidebar');
}

test.describe('object selection and details', () => {
  test('valid object selection URL loads details and full geometry', async ({ page, isMobile }) => {
    let detailRequested = false;
    let geometryRequested = false;

    page.on('request', (req) => {
      const url = req.url();
      if (url.includes('/data/routes/the-three-twisted-sisters-circuit.json')) {
        detailRequested = true;
      }
      if (url.includes('/data/geometry/routes/the-three-twisted-sisters-circuit.geojson')) {
        geometryRequested = true;
      }
    });

    await page.goto('/?kind=route&id=the-three-twisted-sisters-circuit');
    const panel = getPanel(page, isMobile);

    await expect(panel.locator('.detail-view')).toBeVisible();
    await expect(panel.locator('.detail-title')).toContainText('The Three Twisted Sisters');
    await expect(panel.locator('.detail-summary')).toBeVisible();
    await expect(panel.locator('.detail-distance')).toContainText('mi');

    const sharePosition = await panel.getByRole('button', { name: 'Share ride', exact: true }).boundingBox();
    const summaryPosition = await panel.locator('.detail-summary').boundingBox();
    const distancePosition = await panel.locator('.detail-distance').boundingBox();
    expect(sharePosition!.y).toBeGreaterThan(summaryPosition!.y + summaryPosition!.height);
    expect(sharePosition!.y + sharePosition!.height).toBeLessThanOrEqual(distancePosition!.y);

    // Page link
    const pageLink = panel.locator('.detail-page-link');
    await expect(pageLink).toHaveCount(0);
    await expect(panel.getByRole('button', { name: 'Share ride', exact: true })).toBeVisible();

    // Export route disclosure
    const exportDisclosure = panel.locator('.export-route-disclosure');
    await expect(exportDisclosure).toBeVisible();
    await exportDisclosure.locator('.export-route-summary').click();

    // GPX download link
    const gpxLink = panel.locator('.detail-gpx-download');
    await expect(gpxLink).toBeVisible();
    await expect(gpxLink).toHaveAttribute('href', '/downloads/routes/the-three-twisted-sisters-circuit.gpx');
    await expect(gpxLink).toHaveAttribute('download', '');

    expect(detailRequested).toBe(true);
    expect(geometryRequested).toBe(true);
  });

  test('unknown object key shows message without blanking the map', async ({ page, isMobile }) => {
    await page.goto('/?kind=route&id=non-existent-fake-route');
    const panel = getPanel(page, isMobile);

    // Error message displayed
    await expect(panel.locator('.detail-error')).toBeVisible();
    await expect(panel.locator('.detail-error')).toContainText('Object not found');

    // QA-08: Not-found details do not allow saving nonexistent route
    await expect(panel.locator('.btn-detail-save')).toHaveCount(0);
    // QA-08: Error recovery button offers "Back to results" instead of "Retry"
    await expect(panel.locator('.btn-retry-detail')).toContainText('Back to results');

    // Map container remains intact and rendered
    const mapContainer = page.locator('#map-canvas');
    await expect(mapContainer).toBeVisible();
    await expect(mapContainer).toHaveClass(/leaflet-container/);

    // Dismiss returns to results list
    await panel.locator('.btn-back-to-results').click();
    await expect(panel.locator('.results-list')).toBeVisible();
  });

  test('QA-07: route navigation manages keyboard focus between cards and details', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);

    const firstCard = panel.locator('.result-card').first();
    const routeId = await firstCard.getAttribute('data-id');
    expect(routeId).toBeTruthy();

    await firstCard.click();
    await expect(panel.locator('.detail-view')).toBeVisible();

    // Focus moved to the back button inside the detail view
    const backBtn = panel.locator('.btn-back-to-results');
    await expect(backBtn).toBeFocused({ timeout: 5000 });

    // Click back to return to results
    await backBtn.click();
    await expect(panel.locator('.results-list')).toBeVisible();

    // Focus restored to the route card that was opened
    await expect(panel.locator(`.result-card[data-id="${routeId}"]`)).toBeFocused({ timeout: 5000 });
  });

  test('route focus follows open and return and has accessible Save names', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);

    const firstCard = panel.locator('.result-card').first();
    const routeId = await firstCard.getAttribute('data-id');
    const routeTitle = (await firstCard.locator('.card-title').textContent())?.trim();
    expect(routeId).toBeTruthy();
    expect(routeTitle).toBeTruthy();

    // Verify card is article with role=button and not a nested button
    const cardTag = await firstCard.evaluate((el) => el.tagName);
    expect(cardTag).toBe('ARTICLE');
    await expect(firstCard).toHaveAttribute('role', 'button');

    // Save button has accessible name matching route title
    const cardSaveBtn = firstCard.locator('.btn-card-save');
    await expect(cardSaveBtn).toHaveAttribute('aria-label', `Save ${routeTitle}`);

    // Clicking save toggles save state without selecting the card
    await cardSaveBtn.click();
    await expect(cardSaveBtn).toHaveAttribute('aria-label', `Remove ${routeTitle} from saved`);
    await expect(panel.locator('.detail-view')).toHaveCount(0);

    // Untoggle save
    await cardSaveBtn.click();
    await expect(cardSaveBtn).toHaveAttribute('aria-label', `Save ${routeTitle}`);

    // Keyboard activation (Enter key on focused card)
    await firstCard.focus();
    await page.keyboard.press('Enter');
    await expect(panel.locator('.detail-view')).toBeVisible();

    // Detail view save button has accessible name matching route title
    const detailSaveBtn = panel.locator('.btn-detail-save');
    await expect(detailSaveBtn).toHaveAttribute('aria-label', `Save ${routeTitle}`);

    // Back to results
    const backBtn = panel.locator('.btn-back-to-results');
    await expect(backBtn).toBeFocused({ timeout: 5000 });
    await backBtn.click();
    await expect(panel.locator('.results-list')).toBeVisible();

    // Focus returned to the originating visible card
    await expect(panel.locator(`.result-card[data-id="${routeId}"]`)).toBeFocused({ timeout: 5000 });
  });

  test('close behavior restores prior filters and pagination', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);

    // Apply a search query
    const searchInput = panel.locator('.input-search');
    await searchInput.fill('creek');
    await page.waitForTimeout(200);

    // Open first result
    const firstCard = panel.locator('.result-card').first();
    const cardTitle = await firstCard.locator('.card-title').textContent();
    await firstCard.click();

    // Verify detail is open
    await expect(panel.locator('.detail-view')).toBeVisible();
    await expect(panel.locator('.detail-title')).toContainText(cardTitle || '');

    // Close details
    await panel.locator('.btn-back-to-results').click();

    // Results list restored with prior query intact
    await expect(panel.locator('.results-list')).toBeVisible();
    await expect(searchInput).toHaveValue('creek');
  });

  test('rapid A -> B selection with A responding late ignores stale response', async ({ page, isMobile }) => {
    let routeADelayed = true;

    await page.route('**/data/routes/the-three-twisted-sisters-circuit.json', async (route) => {
      // Delay response for route A
      await new Promise((resolve) => setTimeout(resolve, 800));
      await route.continue();
    });

    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Click route A (Twisted Sisters)
    const cardA = panel.locator('.result-card[data-id="the-three-twisted-sisters-circuit"]');
    if (await cardA.count() > 0) {
      await cardA.click({ force: true });
    } else {
      await panel.locator('.result-card').first().click({ force: true });
    }

    // Immediately click back or switch: let's navigate to route B directly via URL
    // Or select Route B
    await page.evaluate(() => {
      window.history.pushState(null, '', '/?kind=route&id=lake-victor-burnet-backcountry-sweep');
      window.dispatchEvent(new PopStateEvent('popstate'));
    });

    // Wait for route B to load
    await expect(panel.locator('.detail-title')).toContainText('Lake Victor');

    // Wait past route A's delay (1000ms)
    await page.waitForTimeout(1000);

    // Ensure route B remains displayed and was not overwritten by route A
    await expect(panel.locator('.detail-title')).toContainText('Lake Victor');
    expect(page.url()).toContain('id=lake-victor-burnet-backcountry-sweep');
  });

  test('text-only route renders without blank photo, note, stops, or nearby sections', async ({ page, isMobile }) => {
    // Mock a text-only route detail
    await page.route('**/data/routes/text-only-fixture.json', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          schema_version: 1,
          key: { kind: 'route', id: 'text-only-fixture' },
          title: 'Pure Text Route',
          summary: 'A route with no photos, no notes, no stops.',
          body_html: '<p>Standard description text.</p>',
          tags: ['test'],
          sources: [],
          photos: [],
          related: [],
          stops: [],
          nearby_places: [],
          bounds: [-98.5, 30.5, -98.0, 31.0],
          category: 'Twisties & Canyons',
          color: '#e74c3c',
          distance_mi: 42.0,
          waypoints: ['A', 'B'],
          via: 'FM 1431',
          route_type: 'loop',
          navigation_links: []
        })
      });
    });
    await page.route('**/data/geometry/routes/text-only-fixture.geojson', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          type: 'Feature',
          properties: {},
          geometry: {
            type: 'LineString',
            coordinates: [[-98.5, 30.5], [-98.0, 31.0]]
          }
        })
      });
    });

    await page.goto('/?kind=route&id=text-only-fixture');
    const panel = getPanel(page, isMobile);

    await expect(panel.locator('.detail-title')).toHaveText('Pure Text Route');
    await expect(panel.locator('.detail-summary')).toBeVisible();

    // Verify sections without content are NOT in DOM (no blank sections)
    await expect(panel.locator('.detail-photos')).toHaveCount(0);
    await expect(panel.locator('.detail-notes')).toHaveCount(0);
    await expect(panel.locator('.detail-stops')).toHaveCount(0);
    await expect(panel.locator('.detail-nearby')).toHaveCount(0);
  });

  test('note-only route renders ride notes without blank photo or stops sections', async ({ page, isMobile }) => {
    await page.route('**/data/routes/note-only-fixture.json', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          schema_version: 1,
          key: { kind: 'route', id: 'note-only-fixture' },
          title: 'Notes Only Route',
          summary: 'A route with ride notes only.',
          body_html: '',
          tags: [],
          sources: [],
          photos: [],
          related: [],
          stops: [],
          nearby_places: [],
          bounds: [-98.5, 30.5, -98.0, 31.0],
          category: 'Twisties & Canyons',
          color: '#e74c3c',
          author_note: {
            visit_status: 'visited',
            recommendation: 'recommend',
            visited_on: '2026-05-15',
            text: 'Great road with clean pavement and scenic vistas.',
            updated_on: '2026-05-16'
          },
          navigation_links: []
        })
      });
    });
    await page.route('**/data/geometry/routes/note-only-fixture.geojson', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          type: 'Feature',
          properties: {},
          geometry: { type: 'LineString', coordinates: [[-98.5, 30.5], [-98.0, 31.0]] }
        })
      });
    });

    await page.goto('/?kind=route&id=note-only-fixture');
    const panel = getPanel(page, isMobile);

    await expect(panel.locator('.detail-title')).toHaveText('Notes Only Route');

    // Notes section is present and contains author review
    await expect(panel.locator('.detail-notes')).toBeVisible();
    await expect(panel.locator('.author-note-text')).toContainText('Great road with clean pavement');

    // Photos and stops sections are absent
    await expect(panel.locator('.detail-photos')).toHaveCount(0);
    await expect(panel.locator('.detail-stops')).toHaveCount(0);
    await expect(panel.locator('.detail-nearby')).toHaveCount(0);
  });

  test('stops-only route renders route stops and transitions to place details with back to route', async ({ page, isMobile }) => {
    await page.route('**/data/routes/stops-fixture.json', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          schema_version: 1,
          key: { kind: 'route', id: 'stops-fixture' },
          title: 'Stops Route',
          summary: 'A route with authored stops.',
          body_html: '',
          tags: [],
          sources: [],
          photos: [],
          related: [],
          stops: [
            {
              key: { kind: 'place', id: 'luckenbach-texas' },
              title: 'Luckenbach Texas',
              page_url: '/places/luckenbach-texas/',
              coordinates: [-98.757, 30.181],
              note: 'Historic post office & general store'
            }
          ],
          nearby_places: [],
          bounds: [-98.8, 30.1, -98.7, 30.2],
          navigation_links: []
        })
      });
    });
    await page.route('**/data/geometry/routes/stops-fixture.geojson', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          type: 'Feature',
          properties: {},
          geometry: { type: 'LineString', coordinates: [[-98.8, 30.1], [-98.7, 30.2]] }
        })
      });
    });

    await page.route('**/data/places/luckenbach-texas.json', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          schema_version: 1,
          key: { kind: 'place', id: 'luckenbach-texas' },
          title: 'Luckenbach Texas',
          summary: 'Iconic Hill Country settlement.',
          body_html: '<p>Town with population of 3.</p>',
          tags: ['historic'],
          sources: [],
          photos: [],
          related: [],
          stops: [],
          nearby_places: [],
          bounds: [-98.757, 30.181, -98.757, 30.181],
          coordinates: [-98.757, 30.181],
          place_category: 'landmark',
          navigation_links: []
        })
      });
    });

    await page.goto('/?kind=route&id=stops-fixture');
    const panel = getPanel(page, isMobile);

    await expect(panel.locator('.detail-title')).toHaveText('Stops Route');
    await expect(panel.locator('.detail-stops')).toBeVisible();
    await expect(panel.locator('.detail-nearby')).toHaveCount(0);

    // Click stop button
    const stopBtn = panel.locator('.stop-select-btn');
    await expect(stopBtn).toContainText('Luckenbach Texas');
    await stopBtn.click();

    // Place detail opened
    await expect(panel.locator('.detail-title')).toHaveText('Luckenbach Texas');
    expect(page.url()).toContain('id=luckenbach-texas');

    // "Back to route" button is visible and clicking it returns to the route
    const backToRouteBtn = panel.locator('.btn-back-to-route');
    await expect(backToRouteBtn).toBeVisible();
    await backToRouteBtn.click();

    // Route detail restored
    await expect(panel.locator('.detail-title')).toHaveText('Stops Route');
    expect(page.url()).toContain('id=stops-fixture');
  });

  test('nearby-only route renders near this route section with distance', async ({ page, isMobile }) => {
    await page.route('**/data/routes/nearby-fixture.json', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          schema_version: 1,
          key: { kind: 'route', id: 'nearby-fixture' },
          title: 'Nearby Route',
          summary: 'A route with nearby POIs.',
          body_html: '',
          tags: [],
          sources: [],
          photos: [],
          related: [],
          stops: [],
          nearby_places: [
            {
              key: { kind: 'place', id: 'hamilton-pool' },
              title: 'Hamilton Pool Preserve',
              page_url: '/places/hamilton-pool/',
              coordinates: [-98.127, 30.342],
              distance_m: 3218.688 // ~2.0 miles
            }
          ],
          bounds: [-98.2, 30.3, -98.1, 30.4],
          navigation_links: []
        })
      });
    });
    await page.route('**/data/geometry/routes/nearby-fixture.geojson', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          type: 'Feature',
          properties: {},
          geometry: { type: 'LineString', coordinates: [[-98.2, 30.3], [-98.1, 30.4]] }
        })
      });
    });

    await page.goto('/?kind=route&id=nearby-fixture');
    const panel = getPanel(page, isMobile);

    await expect(panel.locator('.detail-title')).toHaveText('Nearby Route');
    await expect(panel.locator('.detail-stops')).toHaveCount(0);

    const nearbySection = panel.locator('.detail-nearby');
    await expect(nearbySection).toBeVisible();
    await expect(nearbySection).toContainText('Hamilton Pool Preserve');
    await expect(nearbySection).toContainText('2.0 mi');
  });

  test('navigation actions, explainers, and export disclosure layout', async ({ page, isMobile }) => {
    await page.goto('/?kind=route&id=blanco-river-valley-fm-165-loop');
    const panel = getPanel(page, isMobile);

    await expect(panel.locator('.detail-title')).toBeVisible();

    // Navigation section appears ahead of exports
    const navSection = panel.locator('.detail-nav-links');
    await expect(navSection).toBeVisible();

    // Route navigation includes an origin and destination, rather than only the start.
    const routeLinks = navSection.locator('.detail-maps-link');
    await expect(routeLinks).toHaveCount(1);
    await expect(routeLinks).toHaveText('Open ride in Google Maps');
    await expect(navSection.locator('.nav-route-note')).toHaveText('Google Maps calculates the route, so it may differ from the ride shown here.');
    const href = await routeLinks.first().getAttribute('href');
    const routeUrl = new URL(href!);
    expect(routeUrl.searchParams.has('origin')).toBeTruthy();
    expect(routeUrl.searchParams.has('destination')).toBeTruthy();
    await expect(navSection.locator('.nav-start-explainer')).toHaveCount(0);

    // Export route disclosure is present and contains GPX / GeoJSON
    const exportDisclosure = panel.locator('.export-route-disclosure');
    await expect(exportDisclosure).toBeVisible();
    await exportDisclosure.locator('.export-route-summary').click();
    await expect(exportDisclosure.locator('.detail-gpx-download')).toBeVisible();
    await expect(exportDisclosure.locator('.detail-geojson-download')).toBeVisible();

    // GPX exports remain available without a separate offline-pack control.
    await expect(panel.locator('.offline-pack-section')).toHaveCount(0);
    await expect(panel.getByRole('button', { name: 'Download for Offline Use', exact: true })).toHaveCount(0);
  });

  test('copied route link restores selected ride', async ({ context, page, isMobile }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await page.addInitScript(() => Object.defineProperty(navigator, 'share', { value: undefined, configurable: true }));

    await page.goto('/');
    const panel = getPanel(page, isMobile);

    // Apply a search query first
    const searchInput = panel.locator('.input-search');
    await searchInput.fill('twisted sisters');
    await expect(panel.locator('.results-list .result-item')).toHaveCount(1);

    // Open route
    const card = panel.locator('.result-card').first();
    await card.click();
    await expect(panel.locator('.detail-view')).toBeVisible();
    await expect(panel.locator('.detail-title')).toContainText('The Three Twisted Sisters');

    // Click Copy route link
    const copyBtn = panel.locator('.btn-copy-route-link');
    await expect(copyBtn).toBeVisible();
    await copyBtn.click();

    // Live region announces success
    const statusMsg = panel.locator('div[role="status"][aria-live="polite"]');
    await expect(statusMsg).toHaveText('Link copied');

    // Read clipboard content
    const copiedText = await page.evaluate(() => navigator.clipboard.readText());
    expect(copiedText).toContain('/routes/the-three-twisted-sisters-circuit/');
    // Ensure sender search query is excluded
    expect(copiedText).not.toContain('twisted+sisters');
    expect(copiedText).not.toContain('query=');

    // Open copied URL in page
    await page.goto(copiedText);
    await expect(page).toHaveURL(/kind=route&id=the-three-twisted-sisters-circuit/);
    const newPanel = getPanel(page, isMobile);
    await expect(newPanel.locator('.detail-view')).toBeVisible();
    await expect(newPanel.locator('.detail-title')).toContainText('The Three Twisted Sisters');

    // Back to results works
    const backBtn = newPanel.locator('.btn-back-to-results');
    await expect(backBtn).toBeVisible();
    await backBtn.click();
    await expect(newPanel.locator('.results-list')).toBeVisible();
  });

  test('Share ride uses native sharing and leaves cancellation quiet', async ({ page, isMobile }) => {
    await page.addInitScript(() => {
      Object.defineProperty(navigator, 'share', { configurable: true, value: async (data: ShareData) => {
        (window as any).__sharedRide = data;
        throw new DOMException('Cancelled', 'AbortError');
      }});
    });
    await page.goto('/?kind=route&id=triangle');
    const panel = getPanel(page, isMobile);
    await panel.getByRole('button', { name: 'Share ride', exact: true }).click();
    await expect.poll(() => page.evaluate(() => (window as any).__sharedRide?.url)).toContain('/routes/triangle/');
    await expect(panel.locator('.manual-copy-container')).toHaveCount(0);
    await expect(panel.locator('.share-status')).toBeEmpty();
  });

  test('shared page retains preview without JavaScript', async ({ browser }) => {
    const context = await browser.newContext({ javaScriptEnabled: false });
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:8000/routes/triangle/');
    const preview = page.locator('.route-preview');
    await expect(preview).toBeVisible();
    await expect.poll(() => preview.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(1200);
    await expect(page.locator('meta[name="twitter:card"]')).toHaveAttribute('content', 'summary_large_image');
    await context.close();
  });

  test('copy route link fallback when clipboard write fails', async ({ page, isMobile }) => {
    await page.addInitScript(() => Object.defineProperty(navigator, 'share', { value: undefined, configurable: true }));
    await page.goto('/?kind=route&id=the-three-twisted-sisters-circuit');
    const panel = getPanel(page, isMobile);

    await expect(panel.locator('.detail-view')).toBeVisible();

    // Mock clipboard writeText failure
    await page.evaluate(() => {
      if (navigator.clipboard) {
        navigator.clipboard.writeText = () => Promise.reject(new Error('Permission denied'));
      }
    });

    const copyBtn = panel.locator('.btn-copy-route-link');
    await expect(copyBtn).toBeVisible();
    await copyBtn.click();

    // Fallback manual copy container appears
    const manualContainer = panel.locator('.manual-copy-container');
    await expect(manualContainer).toBeVisible();
    await expect(manualContainer.locator('.manual-copy-label')).toHaveText('Copy this link manually:');
    const manualInput = manualContainer.locator('.input-manual-copy');
    await expect(manualInput).toBeVisible();
    await expect(manualInput).toHaveValue(/\/routes\/the-three-twisted-sisters-circuit\//);
  });
});
