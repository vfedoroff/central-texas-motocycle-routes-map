import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';

function getPanel(page: Page, isMobile: boolean) {
  return page.locator(isMobile ? '.bottom-panel' : '.sidebar');
}

test.describe('catalog analytics', () => {
  test('zero Google network requests on default local preview (consent unset)', async ({ page, isMobile }) => {
    const googleRequests: string[] = [];

    // Intercept any potential Google Analytics / GTM requests
    await page.route(/(googletagmanager|google-analytics)\.com/, (route) => {
      googleRequests.push(route.request().url());
      return route.abort();
    });

    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Perform interactions: select route, apply filter, search
    const firstCard = panel.locator('.result-card').first();
    await expect(firstCard).toBeVisible();
    await firstCard.scrollIntoViewIfNeeded();
    await firstCard.click();
    await expect(panel.locator('.detail-title')).toBeVisible();
    await panel.locator('.btn-close-detail').click();

    const twistiesPill = panel.locator('.cat-pill', { hasText: 'Twisties & Canyons' });
    if (await twistiesPill.isVisible()) {
      await twistiesPill.click();
    }

    const searchInput = panel.locator('.input-search');
    await searchInput.fill('river');
    await page.waitForTimeout(300);

    expect(googleRequests).toHaveLength(0);
  });

  test('zero Google requests when consent is explicitly declined', async ({ page, isMobile }) => {
    const googleRequests: string[] = [];
    await page.route(/(googletagmanager|google-analytics)\.com/, (route) => {
      googleRequests.push(route.request().url());
      return route.abort();
    });

    // Pre-seed localStorage with declined consent
    await page.addInitScript(() => {
      localStorage.setItem('catalog_analytics_consent', 'declined');
    });

    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    await panel.locator('.result-card').first().click();
    await expect(panel.locator('.detail-title')).toBeVisible();

    expect(googleRequests).toHaveLength(0);
  });

  test('zero Google requests when analytics is disabled in config', async ({ page, isMobile }) => {
    const googleRequests: string[] = [];
    await page.route(/(googletagmanager|google-analytics)\.com/, (route) => {
      googleRequests.push(route.request().url());
      return route.abort();
    });

    await page.addInitScript(() => {
      localStorage.setItem('catalog_analytics_consent', 'allowed');
      window['__CATALOG_CONFIG__'] = {
        environment: 'production',
        analytics: {
          enabled: false,
          measurement_id: 'G-TEST1234',
          production_host: '127.0.0.1',
        },
      };
    });

    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    await panel.locator('.result-card').first().click();
    await expect(panel.locator('.detail-title')).toBeVisible();

    expect(googleRequests).toHaveLength(0);
  });

  test('zero Google requests when environment is not production', async ({ page, isMobile }) => {
    const googleRequests: string[] = [];
    await page.route(/(googletagmanager|google-analytics)\.com/, (route) => {
      googleRequests.push(route.request().url());
      return route.abort();
    });

    await page.addInitScript(() => {
      localStorage.setItem('catalog_analytics_consent', 'allowed');
      window['__CATALOG_CONFIG__'] = {
        environment: 'development',
        analytics: {
          enabled: true,
          measurement_id: 'G-TEST1234',
          production_host: '127.0.0.1',
        },
      };
    });

    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    await panel.locator('.result-card').first().click();
    await expect(panel.locator('.detail-title')).toBeVisible();

    expect(googleRequests).toHaveLength(0);
  });

  test('zero Google requests when hostname mismatches production_host', async ({ page, isMobile }) => {
    const googleRequests: string[] = [];
    await page.route(/(googletagmanager|google-analytics)\.com/, (route) => {
      googleRequests.push(route.request().url());
      return route.abort();
    });

    await page.addInitScript(() => {
      localStorage.setItem('catalog_analytics_consent', 'allowed');
      window['__CATALOG_CONFIG__'] = {
        environment: 'production',
        analytics: {
          enabled: true,
          measurement_id: 'G-TEST1234',
          production_host: 'central-texas-routes-map.netlify.app', // differs from 127.0.0.1
        },
      };
    });

    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    await panel.locator('.result-card').first().click();
    await expect(panel.locator('.detail-title')).toBeVisible();

    expect(googleRequests).toHaveLength(0);
  });

  test('consent banner appears when consent is unset and allows acceptance/declining', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Banner should be visible
    const banner = page.locator('.consent-banner');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText('Ride Atlas uses optional analytics');

    // Click Decline
    const declineBtn = page.locator('.consent-btn-decline');
    await declineBtn.click();

    // Banner should disappear
    await expect(banner).not.toBeVisible();

    // Check localStorage
    const stored = await page.evaluate(() => localStorage.getItem('catalog_analytics_consent'));
    expect(stored).toBe('declined');
  });

  test('consent banner allow button stores allowed preference', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    const banner = page.locator('.consent-banner');
    await expect(banner).toBeVisible();

    const allowBtn = page.locator('.consent-btn-allow');
    await allowBtn.click();

    await expect(banner).not.toBeVisible();

    const stored = await page.evaluate(() => localStorage.getItem('catalog_analytics_consent'));
    expect(stored).toBe('allowed');
  });

  test('privacy settings modal allows changing consent and stops further tracking when declined', async ({ page, isMobile }) => {
    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Dismiss banner first if visible
    const declineBanner = page.locator('.consent-btn-decline');
    if (await declineBanner.isVisible()) {
      await declineBanner.click();
    }

    // Open Privacy Settings dialog
    const privacyBtn = isMobile
      ? page.locator('.btn-mobile-privacy')
      : page.locator('.sidebar-footer .btn-privacy-settings');
    await privacyBtn.click();

    const dialog = page.locator('.privacy-dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.locator('.privacy-status-value')).toContainText('Analytics Declined');

    // Allow analytics
    await dialog.locator('.btn-toggle-consent-allow').click();
    await expect(dialog.locator('.privacy-status-value')).toContainText('Analytics Allowed');
    let stored = await page.evaluate(() => localStorage.getItem('catalog_analytics_consent'));
    expect(stored).toBe('allowed');

    // Decline analytics
    await dialog.locator('.btn-toggle-consent-decline').click();
    await expect(dialog.locator('.privacy-status-value')).toContainText('Analytics Declined');
    stored = await page.evaluate(() => localStorage.getItem('catalog_analytics_consent'));
    expect(stored).toBe('declined');

    // Close dialog
    await dialog.locator('.btn-close-privacy').click();
    await expect(dialog).not.toBeVisible();
  });

  test('static privacy page renders policy details and interactive consent controls', async ({ page }) => {
    await page.goto('/privacy/index.html');
    await expect(page.locator('h1')).toHaveText('Privacy Policy');

    // Check event documentation
    await expect(page.locator('body')).toContainText('catalog_object_open');
    await expect(page.locator('body')).toContainText('catalog_filter_apply');
    await expect(page.locator('body')).toContainText('catalog_search_empty');
    await expect(page.locator('body')).toContainText('catalog_maps_click');
    await expect(page.locator('body')).toContainText('catalog_gpx_download');

    // Test consent controls on the static privacy page
    const btnAllow = page.locator('#btn-allow-consent');
    await btnAllow.click();
    const badge = page.locator('#consent-status-badge');
    await expect(badge).toHaveText('Allowed');

    const btnDecline = page.locator('#btn-decline-consent');
    await btnDecline.click();
    await expect(badge).toHaveText('Declined');
  });

  test('allowed production fixture triggers explicit events with strictly sanitized payloads', async ({ page, isMobile }) => {
    // Stub gtag to capture events and payloads in window.__gtag_calls
    await page.addInitScript(() => {
      window['__CATALOG_DISABLE_SCRIPT_INJECTION__'] = true;
      localStorage.setItem('catalog_analytics_consent', 'allowed');

      window['__CATALOG_CONFIG__'] = {
        environment: 'production',
        analytics: {
          enabled: true,
          measurement_id: 'G-PROD1234',
          production_host: window.location.hostname || '127.0.0.1',
        },
      };

      window['__gtag_calls'] = [];
      window['gtag'] = function (...args: any[]) {
        window['__gtag_calls'].push(args);
      };
    });

    await page.goto('/');
    const panel = getPanel(page, isMobile);
    await expect(panel.locator('.results-list')).toBeVisible();

    // Verify config was initialized with privacy-preserving settings
    const initialConfigCalls = await page.evaluate(() => {
      return (window['__gtag_calls'] || []).filter((call: any[]) => call[0] === 'config');
    });
    expect(initialConfigCalls.length).toBeGreaterThanOrEqual(1);
    expect(initialConfigCalls[0][1]).toBe('G-PROD1234');
    expect(initialConfigCalls[0][2]).toMatchObject({
      send_page_view: false,
      allow_google_signals: false,
      restricted_data_processing: true,
    });

    // 1. catalog_object_open event
    await page.evaluate(() => { window['__gtag_calls'] = []; });
    await panel.locator('.result-card').first().click();
    await expect(panel.locator('.detail-title')).toBeVisible();

    let openEvents = await page.evaluate(() => {
      return (window['__gtag_calls'] || []).filter((call: any[]) => call[0] === 'event' && call[1] === 'catalog_object_open');
    });
    expect(openEvents).toHaveLength(1);
    expect(openEvents[0][2]).toHaveProperty('object_kind');
    expect(openEvents[0][2]).toHaveProperty('object_id');
    // Ensure no spurious fields like user location or timestamp
    expect(Object.keys(openEvents[0][2]).sort()).toEqual(['object_id', 'object_kind']);

    // 2. catalog_maps_click event
    await page.evaluate(() => { window['__gtag_calls'] = []; });
    const mapsLink = panel.locator('.detail-maps-link').first();
    if (await mapsLink.isVisible()) {
      await mapsLink.click();
      const mapsEvents = await page.evaluate(() => {
        return (window['__gtag_calls'] || []).filter((call: any[]) => call[0] === 'event' && call[1] === 'catalog_maps_click');
      });
      expect(mapsEvents).toHaveLength(1);
      expect(mapsEvents[0][2]).toHaveProperty('object_kind');
      expect(mapsEvents[0][2]).toHaveProperty('object_id');
      expect(Object.keys(mapsEvents[0][2]).sort()).toEqual(['object_id', 'object_kind']);
    }

    // 3. catalog_gpx_download event
    await page.evaluate(() => { window['__gtag_calls'] = []; });
    const gpxLink = panel.locator('.detail-gpx-download');
    if (await gpxLink.isVisible()) {
      await gpxLink.click();
      const gpxEvents = await page.evaluate(() => {
        return (window['__gtag_calls'] || []).filter((call: any[]) => call[0] === 'event' && call[1] === 'catalog_gpx_download');
      });
      expect(gpxEvents).toHaveLength(1);
      expect(gpxEvents[0][2]).toEqual({
        object_kind: 'route',
        object_id: expect.any(String),
      });
    }

    // Close details
    await panel.locator('.btn-close-detail').click();
    await expect(panel.locator('.results-list')).toBeVisible();

    // 4. catalog_filter_apply event
    await page.evaluate(() => { window['__gtag_calls'] = []; });
    if (isMobile) {
      await panel.locator('.btn-toggle-filters').click();
    }
    const twistiesPill = panel.locator('.cat-pill', { hasText: 'Twisties & Canyons' });
    await twistiesPill.click();

    const filterEvents = await page.evaluate(() => {
      return (window['__gtag_calls'] || []).filter((call: any[]) => call[0] === 'event' && call[1] === 'catalog_filter_apply');
    });
    expect(filterEvents).toHaveLength(1);
    expect(filterEvents[0][2]).toEqual({
      category: 'Twisties & Canyons',
    });

    // 5. catalog_search_empty event
    await page.evaluate(() => { window['__gtag_calls'] = []; });
    const searchInput = panel.locator('.input-search');
    await searchInput.fill('nonexistent-query-xyz-999');
    await page.waitForTimeout(300);

    const emptyEvents = await page.evaluate(() => {
      return (window['__gtag_calls'] || []).filter((call: any[]) => call[0] === 'event' && call[1] === 'catalog_search_empty');
    });
    expect(emptyEvents).toHaveLength(1);
    // Crucial: query string MUST NOT be in payload
    expect(emptyEvents[0][2]).toEqual({
      result_count: 0,
    });

    // 6. Declining consent halts further events
    await page.evaluate(() => {
      window['CatalogAnalytics'].setConsent('declined');
      window['__gtag_calls'] = [];
    });

    // Clear search and select another route
    await searchInput.fill('');
    await page.waitForTimeout(300);
    const postCard = panel.locator('.result-card').first();
    await postCard.scrollIntoViewIfNeeded();
    await postCard.click({ force: true });

    const postDeclineEvents = await page.evaluate(() => {
      return (window['__gtag_calls'] || []).filter((call: any[]) => call[0] === 'event');
    });
    expect(postDeclineEvents).toHaveLength(0);
  });
});
