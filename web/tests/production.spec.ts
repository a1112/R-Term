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

test.beforeAll(async () => {
  const binary = process.env.RTERM_PRODUCTION_WEB_BINARY ?? '../target/release/rterm-web';
  server = spawn(binary, ['--listen', '127.0.0.1:0', '--web-root', 'dist']);
  server.stderr.on('data', (chunk: Buffer) => {
    serverStderr += chunk.toString('utf8');
  });
  bootstrapUrl = await new Promise<string>((resolve, reject) => {
    const timeout = setTimeout(
      () => reject(new Error(`production rterm-web did not start: ${serverStderr}`)),
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
      reject(new Error(`production rterm-web exited before startup with code ${String(code)}`));
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
      reject(new Error(`production rterm-web leaked after shutdown: ${serverStderr}`));
    }, 10_000);
    server.once('exit', () => {
      clearTimeout(timeout);
      resolve();
    });
  });
});

test('production Web package starts a real PTY, preserves exit status, and restarts', async ({
  page,
}) => {
  await installLoopbackOnlyNetworkPolicy(page);
  const response = await page.goto(bootstrapUrl);
  expect(response?.url()).not.toContain('token=');
  await expect(page.locator('#connection-status')).toHaveText('Connected');

  const terminal = page.locator('.workspace-view:not([hidden]) .terminal-container');
  await expect(terminal).toBeVisible();
  await terminal.click({ position: { x: 40, y: 40 } });
  await page.keyboard.type('exit 7');
  await page.keyboard.press('Enter');
  await expect(page.locator('#connection-status')).toHaveText('Exited (7)');

  await page.locator('#restart').click();
  await expect(page.locator('#connection-status')).toHaveText('Connected');
  await page.keyboard.type('exit');
  await page.keyboard.press('Enter');
  await expect(page.locator('#connection-status')).toHaveText('Exited (0)');
});
