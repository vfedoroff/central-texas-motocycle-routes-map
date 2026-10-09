import { test, expect } from '@playwright/test';

test('all route sharing pages expose complete static preview images', async ({ request }) => {
  const index = await (await request.get('/catalog-index.json')).json();
  const routes = index.objects.filter((object: any) => object.key.kind === 'route');
  expect(routes.length).toBeGreaterThan(0);
  for (const route of routes) {
    const id = route.key.id;
    const response = await request.get(`/routes/${id}/`);
    expect(response.ok(), id).toBeTruthy();
    const html = await response.text();
    expect(html).toContain('route on a geographic map');
    expect(html).toContain(`id=${id}`);
    expect(html).toMatch(new RegExp(`property="og:image" content="https://[^" ]+/social/routes/${id}\\.png"`));
    const image = await request.get(`/social/routes/${id}.png`);
    expect(image.ok(), id).toBeTruthy();
    const bytes = await image.body();
    expect(bytes.subarray(0, 8).toString('hex')).toBe('89504e470d0a1a0a');
    expect(bytes.readUInt32BE(16)).toBe(1200);
    expect(bytes.readUInt32BE(20)).toBe(630);
    expect(bytes.length, id).toBeGreaterThan(30_000);
  }
});

for (const id of ['triangle', 'the-three-twisted-sisters-circuit']) {
  test(`${id} preview is visible without JavaScript and opens the selected ride`, async ({ browser, page, isMobile }, testInfo) => {
    const context = await browser.newContext({ javaScriptEnabled: false,
      viewport: isMobile ? { width: 393, height: 851 } : { width: 1280, height: 900 } });
    const staticPage = await context.newPage();
    await staticPage.goto(`http://127.0.0.1:8000/routes/${id}/`);
    const preview = staticPage.locator('.route-preview');
    await expect(preview).toBeVisible();
    await expect(preview).toHaveAttribute('alt', /geographic map/);
    await expect.poll(() => preview.evaluate((img: HTMLImageElement) => img.naturalHeight)).toBe(630);
    await staticPage.screenshot({ path: testInfo.outputPath(`${id}-static.png`), fullPage: true });
    await context.close();
    await page.goto(`/routes/${id}/`);
    await expect(page).toHaveURL(new RegExp(`kind=route&id=${id}`));
    const panel = page.locator(isMobile ? '.bottom-panel' : '.sidebar');
    await expect(panel.locator('.detail-title')).toBeVisible();
    await expect(panel.getByRole('button', { name: 'Share ride', exact: true })).toBeVisible();
  });
}
