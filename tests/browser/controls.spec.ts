import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';

function getPanel(page: Page, isMobile: boolean) {
  return page.locator(isMobile ? '.bottom-panel' : '.sidebar');
}

async function openFiltersIfMobile(panel: ReturnType<typeof getPanel>, isMobile: boolean) {
  if (isMobile) {
    const filtersBtn = panel.locator('.btn-toggle-filters');
    const isExpanded = await filtersBtn.getAttribute('aria-expanded');
    if (isExpanded !== 'true') {
      await filtersBtn.click();
    }
  }
}

test.describe('catalog controls', () => {
  test('bootstrap loads index only without full geometry requests', async ({ page, isMobile }) => {
    const requestedUrls: string[] = [];
    page.on('request', (req) => {
      requestedUrls.push(req.url());
    });

    await page.goto('/');

    const panel = getPanel(page, isMobile);
    // Wait for catalog results to appear
    await expect(panel.locator('.results-list')).toBeVisible();

    // Verify index was loaded
    const hasIndexRequest = requestedUrls.some((u) => u.includes('/catalog-index.json'));
    expect(hasIndexRequest).toBe(true);

    // Verify no full geometry or detail requests on unselected load
    const hasGeometryRequest = requestedUrls.some((u) => u.includes('/data/geometry/'));
    expect(hasGeometryRequest).toBe(false);
  });

  test('index failure shows retry and recovers on click', async ({ page, isMobile }) => {
    let failIndex = true;
    await page.route('**/catalog-index.json', async (route) => {
      if (failIndex) {
        await route.fulfill({ status: 500, body: 'Internal Server Error' });
      } else {
        await route.continue();
      }
    });

    await page.goto('/');

    const panel = getPanel(page, isMobile);
    // Verify error state and retry button
    const retryBtn = panel.locator('.btn-retry');
    await expect(retryBtn).toBeVisible();
    await expect(panel.locator('.state-error')).toContainText('Failed to load catalog');

    // Allow index to succeed and click retry
    failIndex = false;
    await retryBtn.scrollIntoViewIfNeeded();
    await retryBtn.click();

    // Verify results recover
    await expect(panel.locator('.results-list')).toBeVisible();
  });

  test('query search debounces and filters results', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    const searchInput = panel.locator('.input-search');
    await searchInput.fill('twisted sisters');

    // Wait for debounce and filtered list
    await expect(panel.locator('.results-list .result-item')).toHaveCount(1);
    await expect(panel.locator('.card-title')).toContainText('Twisted Sisters');

    // Clear filters button resets list
    await openFiltersIfMobile(panel, isMobile);
    await panel.locator('.btn-clear-filters').first().click();
    await expect(searchInput).toHaveValue('');
    const count = await panel.locator('.results-list .result-item').count();
    expect(count).toBeGreaterThan(1);
  });

  test('category and mileage filters with inline validation errors', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    await openFiltersIfMobile(panel, isMobile);

    // Toggle category pill
    const twistiesPill = panel.locator('.cat-pill', { hasText: 'Twisties & Canyons' });
    await expect(twistiesPill).toHaveAttribute('aria-pressed', 'false');
    await twistiesPill.click();
    await expect(twistiesPill).toHaveClass(/cat-active/);
    await expect(twistiesPill).toHaveAttribute('aria-pressed', 'true');

    // Mileage inputs
    const minInput = panel.locator('.input-mileage').first();
    const maxInput = panel.locator('.input-mileage').nth(1);

    // Enter inverted mileage (min 100, max 20)
    await minInput.fill('100');
    await minInput.dispatchEvent('change');
    await maxInput.fill('20');
    await maxInput.dispatchEvent('change');

    // Expect inline error
    const inlineError = panel.locator('.inline-error');
    await expect(inlineError).toBeVisible();
    await expect(inlineError).toContainText('cannot exceed');

    // Correct mileage
    await minInput.fill('10');
    await minInput.dispatchEvent('change');
    await maxInput.fill('150');
    await maxInput.dispatchEvent('change');
    await expect(inlineError).toBeHidden();
  });

  test('tab switching retains query and remembers route filters', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    const searchInput = panel.locator('.input-search');
    await searchInput.fill('creek');

    // Switch to Places tab
    const placesTab = panel.locator('.tab-btn', { hasText: 'Places' });
    await placesTab.click();
    await expect(placesTab).toHaveClass(/tab-active/);
    await expect(placesTab).toHaveAttribute('aria-selected', 'true');
    await expect(panel.locator('.tab-btn', { hasText: 'Routes' })).toHaveAttribute('aria-selected', 'false');

    // Query is retained
    await expect(searchInput).toHaveValue('creek');

    // Switch back to Routes tab
    const routesTab = panel.locator('.tab-btn', { hasText: 'Routes' });
    await routesTab.click();
    await expect(routesTab).toHaveClass(/tab-active/);
    await expect(routesTab).toHaveAttribute('aria-selected', 'true');
    await expect(placesTab).toHaveAttribute('aria-selected', 'false');
    await expect(searchInput).toHaveValue('creek');
  });

  test('url selection deep link selects object on load', async ({ page, isMobile }) => {
    await page.goto('/?kind=route&id=the-three-twisted-sisters-circuit');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.detail-view')).toBeVisible();
    await expect(panel.locator('.detail-title')).toContainText('Twisted Sisters');

    // Dismissing details restores results list
    await panel.locator('.btn-back-to-results').click();
    await expect(panel.locator('.results-list')).toBeVisible();
  });

  test('mobile bottom sheet toggle between map and results', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
    }

    await page.goto('/');
    const bottomPanel = page.locator('.bottom-panel');
    await expect(bottomPanel).toBeVisible();

    // Default half state
    await expect(bottomPanel).toHaveClass(/snap-half/);

    const toggleBtn = page.locator('.btn-toggle-view');
    await expect(toggleBtn).toBeVisible();
    await expect(toggleBtn).toHaveText('Show map');
    await expect(toggleBtn).toHaveAttribute('aria-expanded', 'true');

    // Toggle to collapse panel and show map
    await toggleBtn.click();
    await expect(bottomPanel).toHaveClass(/snap-collapsed/);
    await expect(toggleBtn).toHaveText('Show results');
    await expect(toggleBtn).toHaveAttribute('aria-expanded', 'false');

    // Toggle back to results
    await toggleBtn.click();
    await expect(bottomPanel).toHaveClass(/snap-half/);
    await expect(toggleBtn).toHaveText('Show map');
    await expect(toggleBtn).toHaveAttribute('aria-expanded', 'true');

    // Privacy button opens modal and has focusable close
    const privacyBtn = page.locator('.btn-mobile-privacy');
    await expect(privacyBtn).toBeVisible();
    await privacyBtn.click();
    const modal = page.locator('.privacy-dialog');
    await expect(modal).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(modal).not.toBeVisible();
  });

  test('request GPS permission, locate map, and sort routes near me', async ({ context, page, isMobile }) => {
    // Mock user GPS in Austin near Lime Creek Road (approx 30.43, -97.89)
    await context.grantPermissions(['geolocation']);
    await context.setGeolocation({ latitude: 30.43, longitude: -97.89 });

    await page.goto('/');

    const consentBtn = page.locator('.consent-btn-allow');
    if (await consentBtn.isVisible()) {
      await consentBtn.click();
    }

    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    await openFiltersIfMobile(panel, isMobile);

    // Verify Find routes near me button exists
    const nearMeBtn = panel.locator('.btn-near-me');
    await expect(nearMeBtn).toBeVisible();
    await expect(nearMeBtn).toContainText('Find routes near me');

    // Click near me button
    await nearMeBtn.click();

    // Verify near me button becomes active
    await expect(nearMeBtn).toHaveClass(/is-active/);
    await expect(nearMeBtn).toContainText('Near me (Active)');

    // Verify distance badges appear on results cards
    const proximityBadge = panel.locator('.badge-proximity').first();
    await expect(proximityBadge).toBeVisible();
    await expect(proximityBadge).toContainText('mi away');

    // Lime Creek Road should be first because user is right by Lake Travis
    const firstTitle = await panel.locator('.card-title').first().textContent();
    expect(firstTitle).toContain('Lime Creek');

    // Verify user pulse marker appears on the map
    await expect(page.locator('.user-pulse-marker')).toBeVisible();

    // Toggle off proximity sorting
    await nearMeBtn.click();
    await expect(nearMeBtn).not.toHaveClass(/is-active/);
  });

  test('map locate control centers map on user GPS coordinates', async ({ context, page }) => {
    await context.grantPermissions(['geolocation']);
    await context.setGeolocation({ latitude: 30.2672, longitude: -97.7431 });

    await page.goto('/');

    const consentBtn = page.locator('.consent-btn-allow');
    if (await consentBtn.isVisible()) {
      await consentBtn.click();
    }

    const mapLocateBtn = page.locator('.leaflet-control-locate');
    await expect(mapLocateBtn).toBeVisible();

    await mapLocateBtn.click({ force: true });
    await expect(page.locator('.user-pulse-marker')).toBeVisible();
  });

  test('UX-01: mobile default Half view reveals result count and first route card without scrolling', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
      return;
    }
    for (const viewport of [null, { width: 320, height: 568 }, { width: 568, height: 320 }]) {
      if (viewport) {
        await page.setViewportSize(viewport);
      }
      await page.goto('/');
      const panel = page.locator('.bottom-panel');
      await expect(panel).toHaveClass(/snap-half/);

      const countHeader = panel.locator('.results-count');
      await expect(countHeader).toBeVisible();

      const firstCard = panel.locator('.result-card').first();
      await expect(firstCard).toBeVisible();

      const viewportHeight = page.viewportSize()?.height ?? 851;
      const countBox = await countHeader.boundingBox();
      const cardBox = await firstCard.boundingBox();
      expect(countBox).not.toBeNull();
      expect(cardBox).not.toBeNull();
      expect(countBox!.y).toBeLessThan(viewportHeight);
      expect(cardBox!.y).toBeLessThan(viewportHeight);
    }
  });

  test('UX-08: mobile landscape primary controls meet minimum 44x44px touch targets', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
      return;
    }
    await page.setViewportSize({ width: 568, height: 320 });
    await page.goto('/');

    const toggleBtn = page.locator('.btn-toggle-view');
    await expect(toggleBtn).toBeVisible();
    const toggleBox = await toggleBtn.boundingBox();
    expect(toggleBox).not.toBeNull();
    if (toggleBox) {
      expect(toggleBox.height).toBeGreaterThanOrEqual(44);
      expect(toggleBox.width).toBeGreaterThanOrEqual(44);
    }

    const privacyBtn = page.locator('.btn-mobile-privacy');
    await expect(privacyBtn).toBeVisible();
    const privacyBox = await privacyBtn.boundingBox();
    expect(privacyBox).not.toBeNull();
    if (privacyBox) {
      expect(privacyBox.height).toBeGreaterThanOrEqual(44);
      expect(privacyBox.width).toBeGreaterThanOrEqual(44);
    }

    // Verify results count and first card remain visible without scrolling
    const countHeader = page.locator('.bottom-panel .results-count');
    await expect(countHeader).toBeVisible();
    const firstCard = page.locator('.bottom-panel .result-card').first();
    await expect(firstCard).toBeVisible();
    const cardBox = await firstCard.boundingBox();
    expect(cardBox).not.toBeNull();
    if (cardBox) {
      expect(cardBox.y).toBeLessThan(320);
    }
  });

  test('mobile compact filters reveal results', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
      return;
    }
    const viewports = [
      { width: 320, height: 568 },
      { width: 393, height: 851 },
      { width: 568, height: 320 },
    ];

    for (const vp of viewports) {
      await page.setViewportSize(vp);
      await page.goto('/');

      const panel = page.locator('.bottom-panel');
      await expect(panel).toBeVisible();

      // At least 80px of map remains visible above default sheet
      const panelBox = await panel.boundingBox();
      expect(panelBox).not.toBeNull();
      expect(panelBox!.y).toBeGreaterThanOrEqual(80);

      // Compact filters toggle is present
      const filtersToggle = panel.locator('.btn-toggle-filters');
      await expect(filtersToggle).toBeVisible();
      await expect(filtersToggle).toHaveAttribute('aria-expanded', 'false');

      // Result count and first title intersect visible sheet
      const countHeader = panel.locator('.results-count');
      await expect(countHeader).toBeVisible();
      const firstCard = panel.locator('.result-card').first();
      await expect(firstCard).toBeVisible();

      const countBox = await countHeader.boundingBox();
      const cardBox = await firstCard.boundingBox();
      expect(countBox!.y).toBeLessThan(vp.height);
      expect(cardBox!.y).toBeLessThan(vp.height);

      // Card can be selected and controls do not overlap them
      await firstCard.click();
      await expect(panel.locator('.detail-view')).toBeVisible();

      // Go back to results
      await panel.locator('.btn-back-to-results').click();
      await expect(panel.locator('.results-list')).toBeVisible();
    }
  });

  test('mobile compact filters disclosure interaction and validation', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
      return;
    }
    await page.goto('/');
    const panel = page.locator('.bottom-panel');

    const filtersBtn = panel.locator('.btn-toggle-filters');
    await expect(filtersBtn).toHaveText('Filters');
    await expect(filtersBtn).toHaveAttribute('aria-expanded', 'false');

    // Open disclosure
    await filtersBtn.click();
    await expect(filtersBtn).toHaveAttribute('aria-expanded', 'true');
    const disclosure = panel.locator('.filters-disclosure');
    await expect(disclosure).toBeVisible();

    // Select a category
    const canyonPill = disclosure.locator('.cat-pill', { hasText: 'Twisties & Canyons' });
    await canyonPill.click();
    await expect(filtersBtn).toHaveText('Filters (1)');

    // Close disclosure with Done button
    const doneBtn = disclosure.locator('.btn-done-filters');
    await doneBtn.click();
    await expect(filtersBtn).toHaveAttribute('aria-expanded', 'false');
    await expect(filtersBtn).toBeFocused();

    // Verify filter is still active and chip is visible
    const chip = panel.locator('.chip-active-category', { hasText: 'Twisties & Canyons' });
    await expect(chip).toBeVisible();

    // Reopen and enter invalid mileage
    await filtersBtn.click();
    const minInput = disclosure.locator('.input-mileage').first();
    const maxInput = disclosure.locator('.input-mileage').last();
    await minInput.fill('150');
    await maxInput.fill('50');
    await maxInput.dispatchEvent('change');

    // Inverted mileage error is displayed inside disclosure
    const errorMsg = disclosure.locator('.inline-error');
    await expect(errorMsg).toBeVisible();
    await expect(errorMsg).toContainText('cannot exceed');

    // Clear filters
    const clearBtn = disclosure.locator('.btn-clear-filters');
    await clearBtn.click();
    await expect(filtersBtn).toHaveText('Filters');

    // Close disclosure
    await filtersBtn.click();
    await expect(filtersBtn).toHaveAttribute('aria-expanded', 'false');
    await expect(panel.locator('.chip-active-category')).toHaveCount(0);
  });

  test('UX-02: places empty state indicates no places added and provides Browse routes recovery', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);

    const placesTab = panel.locator('.tab-btn', { hasText: 'Places' });
    await placesTab.click();

    // Verify empty state message
    const emptyState = panel.locator('.state-empty');
    await expect(emptyState).toBeVisible();
    await expect(emptyState.locator('.empty-title')).toHaveText('No places have been added yet');
    await expect(panel.locator('.checkbox-places')).toHaveCount(0);

    // Click Browse routes to recover
    const browseBtn = emptyState.locator('.btn-browse-routes');
    await expect(browseBtn).toBeVisible();
    await browseBtn.click();

    // Returned to routes
    await expect(panel.locator('.tab-btn', { hasText: 'Routes' })).toHaveClass(/tab-active/);
    await expect(panel.locator('.results-list')).toBeVisible();
  });

  test('UX-03: empty saved list guides user and provides Browse routes recovery', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);

    await openFiltersIfMobile(panel, isMobile);

    const savedCheckbox = panel.locator('.checkbox-saved');
    await savedCheckbox.check();

    const emptyState = panel.locator('.state-empty');
    await expect(emptyState).toBeVisible();
    await expect(emptyState.locator('.empty-title')).toHaveText("You haven't saved any routes yet");
    await expect(emptyState.locator('.empty-hint')).toContainText("Tap Save on a route");

    const browseBtn = emptyState.locator('.btn-browse-all');
    await expect(browseBtn).toBeVisible();
    await browseBtn.click();

    // Saved only unchecked, routes restored
    await expect(panel.locator('.results-list')).toBeVisible();
  });

  test('QA-01: overview routes receive authored category colors', async ({ page }) => {
    await page.goto('/');
    const res = await page.request.get('/data/overview/routes/austin-to-llano-hill-country-blast.geojson');
    expect(res.ok()).toBe(true);
    const json = await res.json();
    expect(json.properties.color).toBe('#e74c3c');
  });

  test('UX-05: selected route is framed within map area visible above mobile bottom panel', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
      return;
    }
    await page.goto('/');
    const panel = page.locator('.bottom-panel');
    await expect(panel).toHaveClass(/snap-half/);

    // Select Austin to Llano route
    const routeCard = panel.locator('.result-card', { hasText: 'Austin to Llano' }).first();
    await routeCard.click();
    await expect(panel.locator('.detail-title')).toBeVisible();

    // Wait for map animation/tiles to settle
    await page.waitForTimeout(600);

    // Verify center of route in screen coordinates is in the visible map area above the panel
    const isFramedAbovePanel = await page.evaluate(() => {
      const container = document.querySelector('#map-canvas') as any;
      const adapter = container?._rideAtlasAdapter;
      if (!adapter) return false;
      const map = adapter.map;
      const panel = document.querySelector('.bottom-panel');
      if (!panel) return false;
      const panelTop = panel.getBoundingClientRect().top;

      // Approximate center of Austin to Llano route: [30.505, -98.205]
      const centerPoint = map.latLngToContainerPoint([30.505, -98.205]);
      return centerPoint.y < panelTop && centerPoint.y > 0;
    });

    expect(isFramedAbovePanel).toBe(true);
  });

  test('QA-04: privacy modal contains keyboard focus, supports Escape, and restores opener focus', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);

    const consentBtn = page.locator('.consent-btn-allow');
    if (await consentBtn.isVisible()) {
      await consentBtn.click();
    }

    const privacyOpener = isMobile
      ? panel.locator('.btn-mobile-privacy')
      : page.locator('.btn-privacy-settings');

    await privacyOpener.click();

    const dialog = page.locator('.privacy-dialog');
    await expect(dialog).toBeVisible();

    // 1. Initial focus inside modal (close button)
    const closeBtn = dialog.locator('.btn-close-privacy');
    await expect(closeBtn).toBeFocused();

    // 2. Tab cycling through all elements inside modal
    await page.keyboard.press('Tab');
    await expect(dialog.locator('.btn-toggle-consent-allow')).toBeFocused();

    await page.keyboard.press('Tab');
    await expect(dialog.locator('.btn-toggle-consent-decline')).toBeFocused();

    await page.keyboard.press('Tab');
    await expect(dialog.locator('.privacy-policy-link')).toBeFocused();

    // Wrap around to first element (close button)
    await page.keyboard.press('Tab');
    await expect(closeBtn).toBeFocused();

    // Shift+Tab backward wraps to last element (policy link)
    await page.keyboard.press('Shift+Tab');
    await expect(dialog.locator('.privacy-policy-link')).toBeFocused();

    // 3. Escape key closes dialog and restores opener focus
    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await expect(privacyOpener).toBeFocused();
  });

  test('UX-06: search this area button label does not wrap or overflow on desktop sidebar', async ({ page, isMobile }) => {
    if (isMobile) {
      test.skip();
      return;
    }
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Pan map to trigger pending bounds
    await page.evaluate(() => {
      const container = document.querySelector('#map-canvas') as any;
      const adapter = container?._rideAtlasAdapter;
      if (adapter) {
        adapter.map.panBy([100, 100], { animate: false });
      }
    });

    const searchAreaBtn = panel.locator('.btn-search-area');
    await expect(searchAreaBtn).toBeVisible();

    // Check that button label does not overflow vertically or wrap
    const overflowCheck = await searchAreaBtn.evaluate((el) => {
      return {
        clientHeight: el.clientHeight,
        scrollHeight: el.scrollHeight,
        clientWidth: el.clientWidth,
        scrollWidth: el.scrollWidth
      };
    });

    expect(overflowCheck.scrollHeight).toBeLessThanOrEqual(overflowCheck.clientHeight);
    expect(overflowCheck.scrollWidth).toBeLessThanOrEqual(overflowCheck.clientWidth);
  });

  test('QA-05: category badges use accessible high-contrast text and solid colors', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    const badge = panel.locator('.badge-category').first();
    await expect(badge).toBeVisible();

    // Check that list badge text color is dark high-contrast (e.g. #991b1b, #075985, etc.), not light/muted
    const badgeStyle = await badge.getAttribute('style');
    expect(badgeStyle).not.toBeNull();
    expect(badgeStyle).toContain('color: #991b1b');

    // Check full-page static route badge
    await page.goto('/routes/austin-to-llano-hill-country-blast/index.html');
    const fullBadge = page.locator('.badge');
    await expect(fullBadge).toBeVisible();
    const fullBadgeStyle = await fullBadge.getAttribute('style');
    // Twisties & Canyons full page badge uses high contrast #b91c1c
    expect(fullBadgeStyle).toContain('#b91c1c');
  });

  test('QA-06: desktop and mobile trees have unique IDs and correct control relationships', async ({ page, isMobile }) => {
    await page.goto('/');

    const prefix = isMobile ? 'mobile' : 'desktop';
    const searchInput = page.locator(`#${prefix}-search-input`);
    await expect(searchInput).toBeAttached();

    const searchLabel = page.locator(`label[for="${prefix}-search-input"]`);
    await expect(searchLabel).toBeAttached();
    await expect(searchLabel).toHaveText('Search catalog');

    const routesTab = page.locator(`#${prefix}-tab-routes`);
    await expect(routesTab).toBeAttached();
    await expect(routesTab).toHaveAttribute('aria-controls', `${prefix}-catalog-results-panel`);

    const resultsPanel = page.locator(`#${prefix}-catalog-results-panel`);
    await expect(resultsPanel).toBeAttached();
    await expect(resultsPanel).toHaveAttribute('role', 'region');
    await expect(resultsPanel).toHaveAttribute('aria-labelledby', `${prefix}-tab-routes`);
  });

  test('explicit sort changes results order and interacts with places and near-me', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    if (isMobile) {
      await panel.locator('.btn-toggle-filters').click();
    }

    const sortSelect = panel.locator('.select-sort');
    await expect(sortSelect).toBeVisible();

    // Default sort is Name
    await expect(sortSelect).toHaveValue('name');

    // Change to shortest first
    await sortSelect.selectOption('distance_asc');

    // Verify first route card has the smallest mileage in catalog
    const firstDistanceBadge = panel.locator('.results-list .badge-distance').first();
    await expect(firstDistanceBadge).toBeVisible();
    const firstDistText = await firstDistanceBadge.innerText();
    const firstDistVal = parseFloat(firstDistText);

    // Change to longest first
    await sortSelect.scrollIntoViewIfNeeded();
    await sortSelect.selectOption('distance_desc');
    const newFirstDistanceBadge = panel.locator('.results-list .badge-distance').first();
    await expect(newFirstDistanceBadge).toBeVisible();
    const newFirstDistText = await newFirstDistanceBadge.innerText();
    const newFirstDistVal = parseFloat(newFirstDistText);

    expect(newFirstDistVal).toBeGreaterThan(firstDistVal);

    // Verify route card has route-type badge (Loop or Corridor)
    const routeTypeBadge = panel.locator('.results-list .badge-route-type').first();
    await expect(routeTypeBadge).toBeVisible();

    // Switch to Places tab: distance options are hidden in sort select
    await sortSelect.scrollIntoViewIfNeeded();
    await panel.locator('button[role="tab"]', { hasText: 'Places' }).click();
    await expect(sortSelect.locator('option[value="distance_asc"]')).toHaveCount(0);
    await expect(sortSelect).toHaveValue('name');

    // Switch back to Routes tab: route sort is restored
    await panel.locator('button[role="tab"]', { hasText: 'Routes' }).click();
    await sortSelect.scrollIntoViewIfNeeded();
    await expect(sortSelect).toHaveValue('distance_desc');
  });

  test('UX-09: pending area does not appear as active filter or count until applied', async ({ page, isMobile }) => {
    if (!isMobile) {
      test.skip();
      return;
    }
    // Test in landscape 568x320 as reported in UX-09
    await page.setViewportSize({ width: 568, height: 320 });
    await page.goto('/');
    const panel = page.locator('.bottom-panel');
    await expect(panel.locator('.results-list')).toBeVisible();

    // Toggle Show map and then Show results
    const toggleBtn = panel.locator('.btn-toggle-view');
    await expect(toggleBtn).toBeVisible();
    await toggleBtn.click(); // collapse (Show map)
    await page.waitForTimeout(400);

    await toggleBtn.click(); // uncollapse (Show results)
    await page.waitForTimeout(400);

    // Filters button must NOT count pending area as an active filter
    const filtersBtn = panel.locator('.btn-toggle-filters');
    await expect(filtersBtn).toHaveText('Filters');

    // No Area indicator badge should be visible
    await expect(panel.locator('.active-filter-indicators .chip-active-category:has-text("Area")')).toHaveCount(0);

    // Pan map to trigger pending bounds
    await page.evaluate(() => {
      const container = document.querySelector('#map-canvas') as any;
      const adapter = container?._rideAtlasAdapter;
      if (adapter) {
        adapter.map.panBy([120, 120], { animate: false });
      }
    });

    // Search this area button appears
    const searchAreaBtn = panel.locator('.btn-search-area');
    await expect(searchAreaBtn).toBeVisible();

    // Still must NOT appear as active filter until user clicks "Search this area"
    await expect(filtersBtn).toHaveText('Filters');
    await expect(panel.locator('.active-filter-indicators .chip-active-category:has-text("Area")')).toHaveCount(0);

    // Click "Search this area"
    await searchAreaBtn.click();
    await page.waitForTimeout(300);

    // Now Area IS applied, so active filter count increments and Area chip appears
    await expect(filtersBtn).toHaveText('Filters (1)');
    const areaChip = panel.locator('.active-filter-indicators .chip-active-category:has-text("Area")');
    await expect(areaChip).toBeVisible();

    // Click remove on the Area chip
    await areaChip.click();
    await page.waitForTimeout(300);

    // Reverts to unapplied
    await expect(filtersBtn).toHaveText('Filters');
    await expect(areaChip).toHaveCount(0);
  });
});

