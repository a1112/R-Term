import { expect, test } from '@playwright/test';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';

let server: ChildProcessWithoutNullStreams;
let bootstrapUrl: string;
let serverStderr = '';

async function installLoopbackOnlyNetworkPolicy(
  page: import('@playwright/test').Page,
): Promise<void> {
  await page.route('**/*', async (route) => {
    const host = new URL(route.request().url()).hostname;
    if (host === 'localhost' || host === '127.0.0.1' || host === '[::1]') {
      await route.continue();
    } else {
      await route.abort('blockedbyclient');
    }
  });
}

async function typeIntoPane(
  page: import('@playwright/test').Page,
  pane: import('@playwright/test').Locator,
  command: string,
): Promise<void> {
  await pane.click({ position: { x: 36, y: 42 } });
  await page.keyboard.type(command);
  await page.keyboard.press('Enter');
}

async function preparePane(
  page: import('@playwright/test').Page,
  pane: import('@playwright/test').Locator,
  index: number,
): Promise<void> {
  await typeIntoPane(page, pane, "export PROMPT='rterm%# '; export PS1='rterm%# '");
  await typeIntoPane(page, pane, 'clear');
  await typeIntoPane(page, pane, `printf 'prod-pane-'; printf '${index + 1} pwd='; pwd`);
  await typeIntoPane(page, pane, `printf 'prod-pane-'; printf '${index + 1} stty='; stty size`);
  await typeIntoPane(page, pane, `printf 'prod-pane-'; printf '${index + 1} git='; git rev-parse --short HEAD`);
}

test.beforeAll(async () => {
  const binary = process.env.RTERM_PRODUCTION_WEB_BINARY ?? '../target/debug/rterm-web';
  server = spawn(binary, ['--listen', '127.0.0.1:0', '--web-root', 'dist']);
  server.stderr.on('data', (chunk: Buffer) => {
    serverStderr += chunk.toString('utf8');
  });
  bootstrapUrl = await new Promise<string>((resolve, reject) => {
    const timeout = setTimeout(
      () => reject(new Error(`production workspace server did not start: ${serverStderr}`)),
      30_000,
    );
    createInterface({ input: server.stdout }).on('line', (line) => {
      const match = line.match(/^R-Term Web terminal: (http:\/\/\S+)$/);
      if (match?.[1]) {
        clearTimeout(timeout);
        resolve(match[1]);
      }
    });
    server.once('exit', (code) => {
      clearTimeout(timeout);
      reject(new Error(`production workspace server exited before startup with code ${String(code)}`));
    });
  });
});

test.afterAll(async () => {
  if (!server.pid) {
    return;
  }
  server.kill('SIGTERM');
  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => {
      server.kill('SIGKILL');
      reject(new Error(`production workspace server leaked after shutdown: ${serverStderr}`));
    }, 10_000);
    server.once('exit', () => {
      clearTimeout(timeout);
      resolve();
    });
  });
});

test('production desktop workspace renders a real three-pane PTY layout', async ({ page }) => {
  await installLoopbackOnlyNetworkPolicy(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  const response = await page.goto(bootstrapUrl);
  expect(response?.url()).not.toContain('token=');
  await expect(page.locator('#connection-status')).toHaveText('Connected');
  expect(await page.evaluate(() => typeof (window as Window & { __RTERM_FUNCTIONAL_SNAPSHOT__?: unknown }).__RTERM_FUNCTIONAL_SNAPSHOT__)).toBe(
    'undefined',
  );
  await expect(page.locator('.session-tab-label')).toHaveText(/Local terminal · \S+/);

  await page.locator('#split-toggle').click();
  await page.locator('[data-split-action="three"]').click();
  await expect(page.locator('.terminal-pane')).toHaveCount(3);
  await expect(page.locator('.split-divider')).toHaveCount(2);

  const rootSplit = page.locator('.split-node').first();
  const rootChildren = rootSplit.locator(':scope > .split-child');
  const rootWidths = await rootChildren.evaluateAll((elements) =>
    elements.map((element) => element.getBoundingClientRect().width),
  );
  expect(rootWidths[0] / (rootWidths[0] + rootWidths[1])).toBeGreaterThan(0.53);
  expect(rootWidths[0] / (rootWidths[0] + rootWidths[1])).toBeLessThan(0.63);

  const panes = page.locator('.terminal-pane');
  for (let index = 0; index < 3; index += 1) {
    await preparePane(page, panes.nth(index).locator('.terminal-container'), index);
  }

  // The production bundle uses xterm's canvas renderer, so assert each real
  // buffer through the built-in search path rather than a mock DOM row.
  await expect(page.locator('.terminal-pane .xterm-screen')).toHaveCount(3);
  for (let index = 0; index < 3; index += 1) {
    await panes.nth(index).locator('.terminal-container').click({ position: { x: 36, y: 42 } });
    await page.locator('#search-toggle').click();
    await page.locator('#search-input').fill(`prod-pane-${index + 1} pwd=`);
    await expect(page.locator('#search-count')).toHaveText('1 / 1');
    await page.locator('#search-toggle').click();
  }

  await page.locator('#settings-toggle').click();
  await page.locator('#settings-font-size').fill('16');
  await page.locator('#settings-font-size').press('Tab');
  await page.locator('#settings-renderer').selectOption('cpu');
  await page.locator('#settings-scrollback').selectOption('5000');
  await expect(page.locator('#renderer-mode')).toHaveText('CPU');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'midnight');
  await page.locator('#settings-toggle').click();

  await page.screenshot({
    path: '../evidence/workspace-console-2026-10-18/web-workspace-production-3pane.png',
  });
});
