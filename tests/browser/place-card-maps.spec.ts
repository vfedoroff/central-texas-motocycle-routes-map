import { test, expect } from './fixtures';

test('place card Maps link opens separately without selecting the card', async ({ page, context, isMobile }) => {
  await page.route('**/catalog-index.json', async (route) => {
    const response = await route.fetch();
    const index = await response.json();
    index.objects = [{
      key: { kind: 'place', id: 'test-lakeside-stop' },
      title: 'Test Lakeside Stop', summary: 'A lakeside stop.', tags: [],
      search_text: 'test lakeside stop', bounds: [-97.65, 31.01, -97.65, 31.01],
      point: [-97.65, 31.01], place_category: 'stop',
      page_url: '/places/test-lakeside-stop/', detail_url: '/data/places/test-lakeside-stop.json',
    }];
    await route.fulfill({ response, json: index });
  });
  await context.route('https://www.google.com/maps/**', (route) => route.fulfill({ body: 'Google Maps' }));
  await page.goto('/');
  const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
  await panel.getByRole('tab', { name: 'Places', exact: true }).click();
  const card = panel.locator('.result-card[data-id="test-lakeside-stop"]');
  const link = card.getByRole('link', { name: 'Open in Google Maps', exact: true });
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute('href', 'https://www.google.com/maps/search/?api=1&query=31.01,-97.65');
  await expect(link).toHaveAttribute('target', '_blank');
  await expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  for (const keyboard of [false, true]) {
    const popupPromise = page.waitForEvent('popup');
    if (keyboard) {
      await link.focus();
      await link.press('Enter');
    } else {
      await link.click();
    }
    const popup = await popupPromise;
    await expect(popup).toHaveURL(/google\.com\/maps\/search/);
    await expect(panel.locator('.detail-view')).toHaveCount(0);
    await expect(card).not.toHaveClass(/card-selected/);
    await popup.close();
  }
});

test('place details offer Maps listing and directions as separate actions', async ({ page, isMobile }) => {
  await page.route('**/catalog-index.json', async (route) => {
    const response = await route.fetch();
    const index = await response.json();
    index.objects = [{
      key: { kind: 'place', id: 'test-lakeside-stop' },
      title: 'Test Lakeside Stop', summary: 'A lakeside stop.', tags: [],
      search_text: 'test lakeside stop', bounds: [-97.65, 31.01, -97.65, 31.01],
      point: [-97.65, 31.01], place_category: 'stop',
      page_url: '/places/test-lakeside-stop/', detail_url: '/data/places/test-lakeside-stop.json',
    }];
    await route.fulfill({ response, json: index });
  });
  await page.route('**/data/places/test-lakeside-stop.json', (route) => route.fulfill({ json: {
    schema_version: 1, key: { kind: 'place', id: 'test-lakeside-stop' },
    title: 'Test Lakeside Stop', summary: 'A lakeside stop.', body_html: '',
    tags: [], sources: [], photos: [], related: [], stops: [], nearby_places: [],
    bounds: [-97.65, 31.01, -97.65, 31.01], coordinates: [-97.65, 31.01],
    place_category: 'stop', navigation_links: [{
      label: 'Directions to Test Lakeside Stop',
      url: 'https://www.google.com/maps/dir/?api=1&destination=31.01,-97.65', mode: 'destination',
    }],
  }}));
  await page.goto('/?kind=place&id=test-lakeside-stop');
  const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
  await expect(panel.locator('.detail-title')).toHaveText('Test Lakeside Stop');
  await expect(panel.locator('.detail-page-link')).toHaveCount(0);
  const link = panel.getByRole('link', { name: 'Open in Google Maps', exact: true });
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute('href', 'https://www.google.com/maps/search/?api=1&query=31.01,-97.65');
  await expect(link).toHaveAttribute('target', '_blank');
  await expect(panel.getByRole('link', { name: 'Directions to Test Lakeside Stop' })).toBeVisible();
});

test('catalog exposes the project GitHub link', async ({ page, isMobile }) => {
  await page.goto('/');
  const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
  const link = panel.getByRole('link', { name: 'GitHub', exact: true });
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute('href', 'https://github.com/vfedoroff/central-texas-motocycle-routes-map');
  await expect(link).toHaveAttribute('target', '_blank');
});

test('Places has its own location action and optional nearby road paths', async ({ page, isMobile }) => {
  await page.route('**/catalog-index.json', async route => {
    const response = await route.fetch();
    const index = await response.json();
    const base = index.objects.find((o: any) => o.key.kind === 'route');
    index.objects = [
      { key: { kind: 'place', id: 'test-stop' }, title: 'Test Stop', summary: 'Stop', tags: [], search_text: 'test stop', bounds: [-97.65,31.01,-97.65,31.01], point: [-97.65,31.01], place_category: 'stop', page_url: '/places/test-stop/', detail_url: '/data/places/test-stop.json' },
      { ...base, key: { kind: 'route', id: 'near-road' }, bounds: [-97.7,31.01,-97.6,31.01], color: '#123456', geometry_url: '/near-road.geojson', overview_url: '/near-road.geojson' },
      { ...base, key: { kind: 'route', id: 'far-road' }, bounds: [-97.7,30.9,-97.6,31.1], color: '#654321', geometry_url: '/far-road.geojson', overview_url: '/far-road.geojson' },
    ];
    await route.fulfill({ response, json: index });
  });
  for (const [name, latitude] of [['near-road',31.01], ['far-road',31.1]] as const) {
    await page.route(`**/${name}.geojson`, route => route.fulfill({ json: { type: 'Feature', properties: {}, geometry: { type: 'LineString', coordinates: [[-97.7,latitude],[-97.6,latitude]] } } }));
  }
  await page.goto('/');
  const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
  await panel.getByRole('tab', { name: 'Places', exact: true }).click();
  if (isMobile) await panel.getByRole('button', { name: /Filters/ }).click();
  await expect(panel.getByRole('button', { name: 'Find places near me' })).toBeVisible();
  const toggle = panel.getByRole('checkbox', { name: 'Show nearby routes' });
  await expect(toggle).not.toBeChecked();
  const countColor = (color: string) => page.evaluate(color => {
    let count = 0;
    (document.getElementById('map-canvas') as any)._rideAtlasAdapter.map.eachLayer((layer: any) => {
      if (layer.feature?.geometry?.type === 'LineString' && layer.options.color === color) count++;
    });
    return count;
  }, color);
  await toggle.check();
  await expect.poll(() => countColor('#123456')).toBe(1);
  await expect.poll(() => countColor('#654321')).toBe(0);
  await toggle.uncheck();
  await expect.poll(() => countColor('#123456')).toBe(0);
});
