import { test as base, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

export interface TestFixtures {
  catalogPage: Page;
}

export const test = base.extend<TestFixtures>({
  catalogPage: async ({ page }, use) => {
    await use(page);
  },
});

export { expect };
