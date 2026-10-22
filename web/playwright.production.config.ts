import { defineConfig } from '@playwright/test';

const browserChannel = process.env.RTERM_BROWSER_CHANNEL;

export default defineConfig({
  testDir: './tests',
  testMatch: 'production*.spec.ts',
  timeout: 60_000,
  retries: 0,
  workers: 1,
  outputDir: '../evidence/production-web/results',
  reporter: [
    ['list'],
    ['json', { outputFile: '../evidence/production-web/playwright.json' }],
  ],
  projects: [
    {
      name: browserChannel ? `chromium-${browserChannel}` : 'chromium',
      use: {
        browserName: 'chromium',
        headless: true,
        ...(browserChannel ? { channel: browserChannel } : {}),
      },
    },
  ],
  use: {
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    viewport: { width: 1440, height: 900 },
  },
});
