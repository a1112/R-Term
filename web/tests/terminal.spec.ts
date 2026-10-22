import { expect, test } from '@playwright/test';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';

let server: ChildProcessWithoutNullStreams;
let bootstrapUrl: string;
let serverStderr = '';

type FunctionalSnapshot = {
  schema: number;
  terminalText: string;
  cursorRow: number;
  cursorColumn: number;
  cols: number;
  rows: number;
  connectionState: string;
  activeSessionId?: string | null;
  sessionCount?: number;
  selection?: {
    start: { x: number; y: number };
    end: { x: number; y: number };
  } | null;
};

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

async function snapshot(page: import('@playwright/test').Page): Promise<FunctionalSnapshot> {
  return page.evaluate(() => {
    const observer = (window as typeof window & {
      __RTERM_FUNCTIONAL_SNAPSHOT__?: () => FunctionalSnapshot;
    }).__RTERM_FUNCTIONAL_SNAPSHOT__;
    if (!observer) {
      throw new Error('functional Web observer is unavailable');
    }
    return observer();
  });
}

test.beforeAll(async () => {
  server = spawn('../target/debug/rterm-web', [
    '--listen',
    '127.0.0.1:0',
    '--web-root',
    'dist',
  ]);
  server.stderr.on('data', (chunk: Buffer) => {
    serverStderr += chunk.toString('utf8');
  });
  bootstrapUrl = await new Promise<string>((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error('rterm-web did not publish a bootstrap URL')), 15_000);
    createInterface({ input: server.stdout }).on('line', (line) => {
      const match = line.match(/^R-Term Web terminal: (http:\/\/\S+)$/);
      if (match?.[1]) {
        clearTimeout(timeout);
        resolve(match[1]);
      }
    });
    server.once('exit', (code) => {
      clearTimeout(timeout);
      reject(new Error(`rterm-web exited before startup with code ${String(code)}`));
    });
  });
});

test.afterAll(async () => {
  server.kill('SIGTERM');
  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => {
      server.kill('SIGKILL');
      reject(new Error(`rterm-web did not stop within cleanup deadline: ${serverStderr}`));
    }, 10_000);
    server.once('exit', () => {
      clearTimeout(timeout);
      resolve();
    });
  });
});

test('redeems the bootstrap ticket and opens a real PTY session', async ({ page, request, browserName }) => {
  await installLoopbackOnlyNetworkPolicy(page);
  await page.setViewportSize({ width: 960, height: 620 });
  const response = await page.goto(bootstrapUrl);
  expect(response?.url()).not.toContain('token=');
  expect(response?.headers()['cache-control']).toContain('no-store');
  expect(response?.headers()['referrer-policy']).toBe('no-referrer');
  expect(response?.headers()['content-security-policy']).toContain("frame-ancestors 'none'");
  await expect(page.locator('#connection-status')).toHaveText('Connected');
  await expect(page.locator('#terminal')).toBeVisible();

  const replay = await request.get(bootstrapUrl, { maxRedirects: 0 });
  expect(replay.status()).toBe(401);
  expect(replay.headers()['cache-control']).toContain('no-store');
  expect(replay.headers()['referrer-policy']).toBe('no-referrer');
  if (browserName === 'chromium') {
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write'], {
      origin: new URL(bootstrapUrl).origin,
    });
  }
  const terminal = page.locator('.workspace-view:not([hidden]) .terminal-container');
  await expect(terminal).toHaveCount(1);
  const bounds = await terminal.boundingBox();
  if (!bounds) {
    throw new Error('terminal has no browser layout box');
  }
  await terminal.click({ position: { x: 40, y: 40 } });
  await page.keyboard.type('echo web-keyboard-probe');
  await page.keyboard.press('Enter');
  await expect
    .poll(async () => (await snapshot(page)).terminalText, { timeout: 15_000 })
    .toContain('web-keyboard-probe');

  await page.evaluate(() => navigator.clipboard.writeText('echo web-clipboard-probe'));
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+V' : 'Control+V');
  await page.keyboard.press('Enter');
  await expect
    .poll(async () => (await snapshot(page)).terminalText, { timeout: 15_000 })
    .toContain('web-clipboard-probe');

  const firstSession = await snapshot(page);
  await page.locator('#new-session').click();
  await expect(page.locator('.session-tab')).toHaveCount(2);
  await expect(page.locator('#connection-status')).toHaveText('Connected');
  const secondSession = await snapshot(page);
  expect(secondSession.sessionCount).toBe(2);
  expect(secondSession.activeSessionId).not.toBe(firstSession.activeSessionId);

  const secondTerminalBounds = await terminal.boundingBox();
  if (!secondTerminalBounds) {
    throw new Error('second terminal has no browser layout box');
  }
  await terminal.click({ position: { x: 40, y: 40 } });
  await page.keyboard.type('echo web-second-session-probe');
  await page.keyboard.press('Enter');
  await expect
    .poll(async () => (await snapshot(page)).terminalText, { timeout: 15_000 })
    .toContain('web-second-session-probe');
  await page.screenshot({
    path: process.env.RTERM_TITLEBAR_SCREENSHOT ?? '../evidence/ui-2026-10-18/web-titlebar.png',
  });

  await page.locator('.session-tab').first().click();
  await expect
    .poll(async () => (await snapshot(page)).activeSessionId, { timeout: 5_000 })
    .toBe(firstSession.activeSessionId);
  await expect
    .poll(async () => (await snapshot(page)).terminalText, { timeout: 5_000 })
    .toContain('web-clipboard-probe');
  await page.locator('.session-tab').nth(1).locator('.session-tab-close').click();
  await expect(page.locator('.session-tab')).toHaveCount(1);
  expect((await snapshot(page)).sessionCount).toBe(1);

  const before = await snapshot(page);
  await page.setViewportSize({ width: 720, height: 480 });
  await expect
    .poll(async () => await snapshot(page), { timeout: 15_000 })
    .not.toMatchObject({ cols: before.cols, rows: before.rows });

  await terminal.click({ position: { x: 40, y: 40 } });
  await page.keyboard.type('exit');
  await page.keyboard.press('Enter');
  await expect(page.locator('#connection-status')).toContainText('Exited');
  await page.locator('#restart').click();
  await expect(page.locator('#connection-status')).toHaveText('Connected');
  expect((await snapshot(page)).connectionState).toBe('open');

  // Exercise the real split tree. Each pane must own its PTY and retain only
  // its own output when focus moves between the three panes.
  await page.locator('#split-toggle').click();
  await page.locator('[data-split-action="three"]').click();
  await expect(page.locator('.terminal-pane')).toHaveCount(3);
  await expect(page.locator('.split-divider')).toHaveCount(2);
  expect((await snapshot(page)).sessionCount).toBe(3);

  const paneIds = await page.locator('.terminal-pane').evaluateAll((elements) =>
    elements.map((element) => element.getAttribute('data-pane-id')).filter((id): id is string => Boolean(id)),
  );
  expect(paneIds).toHaveLength(3);
  const paneMarkers = paneIds.map((_, index) => `web-split-pane-${index + 1}`);
  for (let index = 0; index < paneIds.length; index += 1) {
    const paneTerminal = page.locator(`[data-pane-id="${paneIds[index]}"] .terminal-container`);
    await paneTerminal.click({ position: { x: 28, y: 42 } });
    await page.keyboard.type(`echo ${paneMarkers[index]}`);
    await page.keyboard.press('Enter');
    await expect
      .poll(async () => (await snapshot(page)).terminalText, { timeout: 15_000 })
      .toContain(paneMarkers[index]);
    const paneText = (await snapshot(page)).terminalText;
    for (let other = 0; other < paneMarkers.length; other += 1) {
      if (other !== index) {
        expect(paneText).not.toContain(paneMarkers[other]);
      }
    }
  }

  // Search is scoped to the focused xterm buffer. The marker is emitted in a
  // second command so it occurs only in the output line, after CJK/emoji cells.
  const unicodeMarker = 'web-unicode-marker';
  await page.keyboard.type("printf '\\n前🙂web-'; printf 'unicode-marker\\n'");
  await page.keyboard.press('Enter');
  await expect
    .poll(async () => (await snapshot(page)).terminalText, { timeout: 15_000 })
    .toContain(unicodeMarker);
  await page.locator('#search-toggle').click();
  await page.locator('#search-input').fill(unicodeMarker);
  await expect(page.locator('#search-count')).toHaveText('1 / 1');
  const searchSnapshot = await snapshot(page);
  // In this real xterm build the CJK/emoji prefix occupies two cells; using
  // UTF-16 offsets would start one cell later (x=4) instead of x=3.
  expect(searchSnapshot.selection?.start.x).toBe(3);
  expect(searchSnapshot.selection?.end.x).toBe(21);
  await page.screenshot({
    path: '../evidence/workspace-console-2026-10-18/web-search.png',
  });

  // New output invalidates the cached match coordinates without rescanning
  // during the hot write path; the next search action refreshes them.
  await page.locator('#search-toggle').click();
  await page.locator('.terminal-pane[data-active="true"] .terminal-container').click({
    position: { x: 28, y: 42 },
  });
  await page.keyboard.type("printf '\\n前🙂web-'; printf 'unicode-marker\\n'");
  await page.keyboard.press('Enter');
  await expect
    .poll(async () => (await snapshot(page)).terminalText, { timeout: 15_000 })
    .toContain(unicodeMarker);
  await page.locator('#search-toggle').click();
  await expect(page.locator('#search-count')).toHaveText('1 / 2');
  await page.locator('#search-next').click();
  await expect(page.locator('#search-count')).toHaveText('2 / 2');

  // An empty result is a real cached search state: it clears any previous
  // selection and pressing Next must not rescan the full scrollback.
  await page.locator('#search-input').fill('web-search-no-match');
  await expect(page.locator('#search-count')).toHaveText('No matches');
  expect((await snapshot(page)).selection).toBeNull();
  await page.locator('#search-next').click();
  await expect(page.locator('#search-count')).toHaveText('No matches');
  expect((await snapshot(page)).selection).toBeNull();

  // The same query must be recomputed after a fit/reflow, even when output
  // has not changed and the search input remains open across the resize.
  await page.locator('#search-input').fill(unicodeMarker);
  await expect(page.locator('#search-count')).toHaveText('1 / 2');
  await page.locator('#search-toggle').click();
  const beforeSearchResize = await snapshot(page);
  await page.setViewportSize({ width: 800, height: 540 });
  await expect
    .poll(async () => await snapshot(page), { timeout: 15_000 })
    .not.toMatchObject({ cols: beforeSearchResize.cols, rows: beforeSearchResize.rows });
  await page.locator('#search-toggle').click();
  await expect(page.locator('#search-count')).toHaveText('1 / 2');
  await page.locator('#search-toggle').click();

  // Settings take effect immediately and survive a reload through localStorage.
  await page.locator('#settings-toggle').click();
  await page.locator('#settings-font-size').fill('18');
  await page.locator('#settings-font-size').press('Tab');
  await page.locator('#settings-theme').selectOption('light');
  await page.locator('#settings-renderer').selectOption('cpu');
  await page.locator('#settings-scrollback').selectOption('5000');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.locator('#renderer-mode')).toHaveText('CPU');
  const storedSettings = await page.evaluate(() => localStorage.getItem('rterm.web.settings.v2'));
  expect(storedSettings).toContain('"fontSize":18');
  expect(storedSettings).toContain('"scrollback":5000');
  expect(storedSettings).toContain('"renderer":"cpu"');
  await page.locator('#settings-theme').selectOption('midnight');
  await page.screenshot({
    path: '../evidence/workspace-console-2026-10-18/web-settings.png',
  });
  await page.locator('#settings-toggle').click();

  // Drag the root divider and verify the rendered layout ratio changes.
  const rootSplit = page.locator('.split-node').first();
  const rootChildren = rootSplit.locator(':scope > .split-child');
  const beforeDividerWidths = await rootChildren.evaluateAll((elements) =>
    elements.map((element) => Math.round(element.getBoundingClientRect().width)),
  );
  const divider = rootSplit.locator(':scope > .split-divider');
  const dividerBounds = await divider.boundingBox();
  if (!dividerBounds) {
    throw new Error('split divider has no browser layout box');
  }
  await page.mouse.move(dividerBounds.x + dividerBounds.width / 2, dividerBounds.y + dividerBounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(dividerBounds.x + dividerBounds.width / 2 + 36, dividerBounds.y + dividerBounds.height / 2);
  await page.mouse.up();
  await expect
    .poll(async () =>
      rootChildren.evaluateAll((elements) => elements.map((element) => Math.round(element.getBoundingClientRect().width))),
    )
    .not.toEqual(beforeDividerWidths);

  await page.screenshot({
    path: '../evidence/workspace-console-2026-10-18/web-workspace-3pane.png',
  });

  // Closing a pane disposes its socket and leaves the other two sessions live.
  const activePaneElement = page.locator('.terminal-pane[data-active="true"]');
  await activePaneElement.locator('.pane-more').click();
  await activePaneElement.locator('.pane-menu [data-pane-action="close"]').click();
  await expect(page.locator('.terminal-pane')).toHaveCount(2);
  expect((await snapshot(page)).sessionCount).toBe(2);
  await page.screenshot({
    path: process.env.RTERM_TITLEBAR_SCREENSHOT ?? '../evidence/workspace-console-2026-10-18/web-workspace-2pane-after-close.png',
  });
});
