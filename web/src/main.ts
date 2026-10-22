import '@xterm/xterm/css/xterm.css';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { Terminal } from '@xterm/xterm';

import {
  PROTOCOL_VERSION,
  encodeBinaryInput,
  encodeControl,
  encodeTerminalInput,
  parseServerMessage,
  type ClientMessage,
} from './protocol';
import './styles.css';

type ConnectionState = 'connecting' | 'open' | 'closing' | 'exited' | 'failed';
type RendererMode = 'GPU' | 'CPU';
type SplitDirection = 'horizontal' | 'vertical';
type ThemeName = 'midnight' | 'dark' | 'light';
type RendererSetting = 'auto' | 'gpu' | 'cpu';

type Settings = {
  fontSize: number;
  theme: ThemeName;
  renderer: RendererSetting;
  scrollback: number;
};

type PaneNode = { kind: 'pane'; paneId: string };
type SplitNode = {
  kind: 'split';
  id: string;
  direction: SplitDirection;
  ratio: number;
  first: WorkspaceNode;
  second: WorkspaceNode;
};
type WorkspaceNode = PaneNode | SplitNode;

type Workspace = {
  id: string;
  label: string;
  tab: HTMLElement;
  labelElement: HTMLElement;
  element: HTMLElement;
  content: HTMLElement;
  root: WorkspaceNode;
  paneIds: Set<string>;
  activePaneId: string;
};

type TerminalPane = {
  id: string;
  workspaceId: string;
  label: string;
  title: string;
  shell?: string;
  cwd?: string;
  terminal: Terminal;
  fitAddon: FitAddon;
  element: HTMLElement;
  mount: HTMLElement;
  titleElement: HTMLElement;
  metaElement: HTMLElement;
  socket: WebSocket | null;
  sessionOpen: boolean;
  closeRequested: boolean;
  removed: boolean;
  state: ConnectionState;
  statusText: string;
  bufferRevision: number;
  lastSize: { cols: number; rows: number };
  resizeFrame: number | null;
  resizeObserver: ResizeObserver;
  webglAddon?: WebglAddon;
  rendererMode: RendererMode;
};

type SearchMatch = { paneId: string; row: number; column: number; length: number };

const terminalWorkspace = document.querySelector<HTMLElement>('#terminal');
const sessionTabs = document.querySelector<HTMLElement>('#session-tabs');
const newSessionElement = document.querySelector<HTMLButtonElement>('#new-session');
const searchToggleElement = document.querySelector<HTMLButtonElement>('#search-toggle');
const splitToggleElement = document.querySelector<HTMLButtonElement>('#split-toggle');
const settingsToggleElement = document.querySelector<HTMLButtonElement>('#settings-toggle');
const searchPopoverElement = document.querySelector<HTMLElement>('#search-popover');
const splitPopoverElement = document.querySelector<HTMLElement>('#split-popover');
const settingsPopoverElement = document.querySelector<HTMLElement>('#settings-popover');
const searchInputElement = document.querySelector<HTMLInputElement>('#search-input');
const searchPreviousElement = document.querySelector<HTMLButtonElement>('#search-previous');
const searchNextElement = document.querySelector<HTMLButtonElement>('#search-next');
const searchCountElement = document.querySelector<HTMLElement>('#search-count');
const settingsFontSizeElement = document.querySelector<HTMLInputElement>('#settings-font-size');
const settingsThemeElement = document.querySelector<HTMLSelectElement>('#settings-theme');
const settingsRendererElement = document.querySelector<HTMLSelectElement>('#settings-renderer');
const settingsScrollbackElement = document.querySelector<HTMLSelectElement>('#settings-scrollback');
const statusElement = document.querySelector<HTMLElement>('#connection-status');
const sessionContextElement = document.querySelector<HTMLElement>('#session-context');
const workspaceNoticeElement = document.querySelector<HTMLElement>('#workspace-notice');
const terminalSizeElement = document.querySelector<HTMLElement>('#terminal-size');
const rendererModeElement = document.querySelector<HTMLElement>('#renderer-mode');
const restartElement = document.querySelector<HTMLButtonElement>('#restart');
const windowControlsElement = document.querySelector<HTMLElement>('#window-controls');
const minimizeElement = document.querySelector<HTMLButtonElement>('#window-minimize');
const maximizeElement = document.querySelector<HTMLButtonElement>('#window-maximize');
const closeElement = document.querySelector<HTMLButtonElement>('#window-close');

if (
  !terminalWorkspace ||
  !sessionTabs ||
  !newSessionElement ||
  !searchToggleElement ||
  !splitToggleElement ||
  !settingsToggleElement ||
  !searchPopoverElement ||
  !splitPopoverElement ||
  !settingsPopoverElement ||
  !searchInputElement ||
  !searchPreviousElement ||
  !searchNextElement ||
  !searchCountElement ||
  !settingsFontSizeElement ||
  !settingsThemeElement ||
  !settingsRendererElement ||
  !settingsScrollbackElement ||
  !statusElement ||
  !sessionContextElement ||
  !workspaceNoticeElement ||
  !terminalSizeElement ||
  !rendererModeElement ||
  !restartElement ||
  !windowControlsElement ||
  !minimizeElement ||
  !maximizeElement ||
  !closeElement
) {
  throw new Error('R-Term Web terminal markup is incomplete');
}

const workspaceRoot = terminalWorkspace;
const tabs = sessionTabs;
const newSessionButton = newSessionElement;
const searchToggle = searchToggleElement;
const splitToggle = splitToggleElement;
const settingsToggle = settingsToggleElement;
const searchPopover = searchPopoverElement;
const splitPopover = splitPopoverElement;
const settingsPopover = settingsPopoverElement;
const searchInput = searchInputElement;
const searchPrevious = searchPreviousElement;
const searchNext = searchNextElement;
const searchCount = searchCountElement;
const settingsFontSize = settingsFontSizeElement;
const settingsTheme = settingsThemeElement;
const settingsRenderer = settingsRendererElement;
const settingsScrollback = settingsScrollbackElement;
const status = statusElement;
const sessionContext = sessionContextElement;
const workspaceNotice = workspaceNoticeElement;
const terminalSize = terminalSizeElement;
const rendererMode = rendererModeElement;
const restart = restartElement;
const windowControls = windowControlsElement;
const minimize = minimizeElement;
const maximize = maximizeElement;
const close = closeElement;

const MAX_SESSIONS = 8;
const SOCKET_INPUT_HIGH_WATER_BYTES = 1_048_576;
const SETTINGS_STORAGE_KEY = 'rterm.web.settings.v2';
const DEFAULT_SETTINGS: Settings = {
  fontSize: 14,
  theme: 'midnight',
  renderer: 'auto',
  scrollback: 10_000,
};
const validScrollback = new Set([1000, 5000, 10_000, 20_000]);
const workspaces = new Map<string, Workspace>();
const panes = new Map<string, TerminalPane>();
const settings = loadSettings();
let activeWorkspaceId: string | null = null;
let activePaneId: string | null = null;
let workspaceSequence = 0;
let paneSequence = 0;
let splitSequence = 0;
let functionalPublishFrame: number | null = null;
let noticeTimer: number | null = null;
let searchMatches: SearchMatch[] = [];
let searchIndex = -1;
let searchPaneId: string | null = null;
let searchRevision = -1;
let searchQuery: string | null = null;

function loadSettings(): Settings {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(SETTINGS_STORAGE_KEY) ?? 'null');
    if (!isRecord(value)) {
      return { ...DEFAULT_SETTINGS };
    }
    const fontSize = isNumber(value.fontSize) ? Math.min(24, Math.max(10, Math.round(value.fontSize))) : DEFAULT_SETTINGS.fontSize;
    const theme = value.theme === 'dark' || value.theme === 'light' || value.theme === 'midnight' ? value.theme : DEFAULT_SETTINGS.theme;
    const renderer = value.renderer === 'auto' || value.renderer === 'gpu' || value.renderer === 'cpu' ? value.renderer : DEFAULT_SETTINGS.renderer;
    const scrollback = isNumber(value.scrollback) && validScrollback.has(value.scrollback) ? value.scrollback : DEFAULT_SETTINGS.scrollback;
    return { fontSize, theme, renderer, scrollback };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

function saveSettings(): void {
  try {
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Persistence is optional in private browsing or restricted webviews.
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function setMaximizedState(value: boolean): void {
  maximize.dataset.maximized = String(value);
  maximize.setAttribute('aria-label', value ? 'Restore window' : 'Maximize window');
  maximize.title = value ? 'Restore window' : 'Maximize window';
  const glyph = maximize.querySelector('span');
  if (glyph) {
    glyph.textContent = value ? '❐' : '□';
  }
}

function isMacOSWindow(): boolean {
  return /Macintosh|Mac OS X/.test(navigator.userAgent);
}

function isAppSessionShortcut(event: KeyboardEvent): boolean {
  const key = event.key.toLowerCase();
  if (event.type !== 'keydown' || event.altKey || (key !== 't' && key !== 'w')) {
    return false;
  }
  return isMacOSWindow()
    ? event.metaKey && !event.ctrlKey && !event.shiftKey
    : event.ctrlKey && !event.metaKey && event.shiftKey;
}

function isSearchShortcut(event: KeyboardEvent): boolean {
  return (
    event.type === 'keydown' &&
    event.key.toLowerCase() === 'f' &&
    !event.altKey &&
    !event.shiftKey &&
    (isMacOSWindow() ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey)
  );
}

async function setupTauriWindowControls(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  const appWindow = getCurrentWindow();
  const macOS = isMacOSWindow();
  document.documentElement.dataset.tauriWindow = 'true';
  windowControls.dataset.platform = macOS ? 'macos' : 'other';
  windowControls.hidden = false;
  minimize.addEventListener('click', () => void appWindow.minimize());
  maximize.addEventListener('click', () => {
    void appWindow
      .toggleMaximize()
      .then(() => appWindow.isMaximized())
      .then(setMaximizedState)
      .catch(() => {});
  });
  close.addEventListener('click', () => void appWindow.close());
  try {
    setMaximizedState(await appWindow.isMaximized());
    await appWindow.onResized(() => {
      void appWindow.isMaximized().then(setMaximizedState);
    });
  } catch {
    // Window state is optional; controls remain usable without it.
  }
}

void setupTauriWindowControls();

function terminalTheme(theme: ThemeName): Record<string, string> {
  if (theme === 'light') {
    return {
      background: '#f7f9fb',
      foreground: '#253547',
      cursor: '#008f8a',
      selectionBackground: '#b8d7e8',
      black: '#253547',
      red: '#b42318',
      green: '#18794e',
      yellow: '#8a5a00',
      blue: '#175cd3',
      magenta: '#8e3ec9',
      cyan: '#087f8c',
      white: '#eef2f6',
      brightBlack: '#66788a',
      brightRed: '#d92d20',
      brightGreen: '#039855',
      brightYellow: '#b54708',
      brightBlue: '#1570ef',
      brightMagenta: '#9e5bd3',
      brightCyan: '#0e9f9a',
      brightWhite: '#101828',
    };
  }
  const dark = theme === 'dark';
  return {
    background: dark ? '#0d1117' : '#090e15',
    foreground: '#d6deeb',
    cursor: '#41d9cf',
    selectionBackground: '#264f78',
    black: dark ? '#0d1117' : '#090e15',
    red: '#ff7b72',
    green: '#7ee787',
    yellow: '#d2a8ff',
    blue: '#79c0ff',
    magenta: '#d2a8ff',
    cyan: '#a5d6ff',
    white: '#f0f6fc',
    brightBlack: '#484f58',
    brightRed: '#ffa198',
    brightGreen: '#56d364',
    brightYellow: '#e3b341',
    brightBlue: '#a5d6ff',
    brightMagenta: '#b58cf0',
    brightCyan: '#b3f0ff',
    brightWhite: '#ffffff',
  };
}

function applyDocumentTheme(): void {
  document.documentElement.dataset.theme = settings.theme;
}

function createTerminal(): Terminal {
  return new Terminal({
    allowTransparency: false,
    cursorBlink: true,
    cursorStyle: 'block',
    fontFamily: '"SFMono-Regular", "Cascadia Code", "Roboto Mono", Menlo, Consolas, monospace',
    fontSize: settings.fontSize,
    scrollback: settings.scrollback,
    theme: terminalTheme(settings.theme),
  });
}

function activeWorkspace(): Workspace | undefined {
  return activeWorkspaceId ? workspaces.get(activeWorkspaceId) : undefined;
}

function activePane(): TerminalPane | undefined {
  return activePaneId ? panes.get(activePaneId) : undefined;
}

function showNotice(message: string): void {
  workspaceNotice.textContent = message;
  if (noticeTimer !== null) {
    window.clearTimeout(noticeTimer);
  }
  noticeTimer = window.setTimeout(() => {
    workspaceNotice.textContent = '';
    noticeTimer = null;
  }, 3200);
}

function updateNewSessionState(): void {
  const atCapacity = panes.size >= MAX_SESSIONS;
  newSessionButton.disabled = atCapacity;
  newSessionButton.title = atCapacity ? 'Maximum sessions reached' : 'New terminal';
}

function shellName(shell: string): string {
  const normalized = shell.replaceAll('\\', '/');
  return normalized.slice(normalized.lastIndexOf('/') + 1) || shell;
}

function paneMetadata(pane: TerminalPane): string {
  const values: string[] = [];
  if (pane.shell) {
    values.push(shellName(pane.shell));
  }
  if (pane.cwd) {
    values.push(pane.cwd);
  }
  return values.join(' · ');
}

function updatePaneHeader(pane: TerminalPane): void {
  pane.element.dataset.active = String(pane.id === activePaneId);
  pane.element.dataset.state = pane.state;
  pane.titleElement.textContent = pane.title || pane.label;
  pane.metaElement.textContent = paneMetadata(pane);
  pane.metaElement.hidden = paneMetadata(pane).length === 0;
  const workspace = workspaces.get(pane.workspaceId);
  if (workspace) {
    updateWorkspaceTabLabel(workspace);
  }
  if (pane.id === activePaneId) {
    updateSessionChrome(pane);
  }
}

function updateWorkspaceTabLabel(workspace: Workspace): void {
  const pane = panes.get(workspace.activePaneId) ?? [...workspace.paneIds]
    .map((paneId) => panes.get(paneId))
    .find((candidate): candidate is TerminalPane => Boolean(candidate));
  const hasCustomTitle = Boolean(pane?.title && pane.title !== pane.label);
  const label = hasCustomTitle
    ? pane?.title ?? workspace.label
    : pane?.shell
      ? `${workspace.label} · ${shellName(pane.shell)}`
      : workspace.label;
  workspace.labelElement.textContent = label;
  workspace.tab.setAttribute('aria-label', label);
}

function updateWorkspaceTab(workspace: Workspace): void {
  const active = workspace.id === activeWorkspaceId;
  workspace.tab.dataset.active = String(active);
  workspace.tab.setAttribute('aria-selected', String(active));
  workspace.tab.setAttribute('tabindex', active ? '0' : '-1');
}

function updateSessionChrome(pane: TerminalPane | undefined): void {
  if (!pane || pane.removed || pane.id !== activePaneId) {
    return;
  }
  status.dataset.state = pane.state;
  status.textContent = pane.statusText;
  sessionContext.textContent = pane.title || pane.label;
  terminalSize.textContent = `${pane.terminal.cols} × ${pane.terminal.rows}`;
  rendererMode.textContent = pane.rendererMode;
  restart.hidden = pane.state !== 'exited' && pane.state !== 'failed';
}

function updateAllHeaders(): void {
  for (const pane of panes.values()) {
    updatePaneHeader(pane);
  }
}

function setPaneStatus(pane: TerminalPane, state: ConnectionState, text: string): void {
  if (pane.removed) {
    return;
  }
  pane.state = state;
  pane.statusText = text;
  updatePaneHeader(pane);
  scheduleFunctionalPublish();
}

function isPaneVisible(pane: TerminalPane): boolean {
  const workspace = workspaces.get(pane.workspaceId);
  return Boolean(workspace && workspace.id === activeWorkspaceId && !workspace.element.hidden && pane.element.isConnected);
}

function enableRenderer(pane: TerminalPane): void {
  if (pane.removed || pane.webglAddon || settings.renderer === 'cpu') {
    return;
  }
  let webglAddon: WebglAddon | undefined;
  try {
    webglAddon = new WebglAddon();
    webglAddon.onContextLoss(() => {
      webglAddon?.dispose();
      if (!pane.removed) {
        pane.webglAddon = undefined;
        pane.rendererMode = 'CPU';
        updatePaneHeader(pane);
      }
    });
    pane.terminal.loadAddon(webglAddon);
    pane.webglAddon = webglAddon;
    pane.rendererMode = 'GPU';
  } catch {
    // xterm may register an addon before loadAddon reports a renderer failure.
    // Dispose the failed instance so every visibility retry does not retain one.
    webglAddon?.dispose();
    pane.webglAddon = undefined;
    pane.rendererMode = 'CPU';
  }
}

function disableRenderer(pane: TerminalPane): void {
  if (!pane.webglAddon) {
    pane.rendererMode = 'CPU';
    return;
  }
  pane.webglAddon.dispose();
  pane.webglAddon = undefined;
  pane.rendererMode = 'CPU';
}

function updateVisiblePanes(): void {
  for (const pane of panes.values()) {
    const visible = isPaneVisible(pane);
    if (visible) {
      pane.resizeObserver.observe(pane.mount);
      if (settings.renderer === 'cpu') {
        disableRenderer(pane);
      } else {
        enableRenderer(pane);
      }
      scheduleFit(pane);
    } else {
      pane.resizeObserver.disconnect();
      disableRenderer(pane);
    }
  }
  updateAllHeaders();
}

function setActiveWorkspace(id: string): void {
  const next = workspaces.get(id);
  if (!next) {
    return;
  }
  activeWorkspaceId = id;
  if (!panes.has(next.activePaneId)) {
    next.activePaneId = firstPaneId(next.root);
  }
  activePaneId = next.activePaneId;
  updateWorkspaceTabLabel(next);
  for (const workspace of workspaces.values()) {
    workspace.element.hidden = workspace.id !== id;
    updateWorkspaceTab(workspace);
  }
  updateVisiblePanes();
  updateSessionChrome(activePane());
  activePane()?.terminal.focus();
  scheduleFunctionalPublish();
}

function setActivePane(id: string): void {
  const pane = panes.get(id);
  if (!pane) {
    return;
  }
  const workspace = workspaces.get(pane.workspaceId);
  if (!workspace) {
    return;
  }
  activeWorkspaceId = workspace.id;
  activePaneId = pane.id;
  workspace.activePaneId = pane.id;
  updateWorkspaceTabLabel(workspace);
  for (const candidate of workspaces.values()) {
    candidate.element.hidden = candidate.id !== workspace.id;
    updateWorkspaceTab(candidate);
  }
  updateVisiblePanes();
  updateSessionChrome(pane);
  pane.terminal.focus();
  scheduleFunctionalPublish();
}

function scheduleFit(pane: TerminalPane): void {
  if (pane.resizeFrame === null && !pane.removed) {
    pane.resizeFrame = requestAnimationFrame(() => fitAndResize(pane));
  }
}

function fitAndResize(pane: TerminalPane): void {
  pane.resizeFrame = null;
  if (pane.removed || !isPaneVisible(pane)) {
    return;
  }
  pane.fitAddon.fit();
  // FitAddon can reflow wrapped rows even when the resulting dimensions are
  // unchanged. Search coordinates therefore become stale after every fit.
  pane.bufferRevision += 1;
  const nextSize = { cols: pane.terminal.cols, rows: pane.terminal.rows };
  if (nextSize.cols === pane.lastSize.cols && nextSize.rows === pane.lastSize.rows) {
    updateSessionChrome(pane);
    return;
  }
  pane.lastSize = nextSize;
  if (pane.sessionOpen) {
    sendControl(pane, { type: 'resize', ...nextSize });
  }
  updateSessionChrome(pane);
  scheduleFunctionalPublish();
}

function sendControl(pane: TerminalPane, message: ClientMessage): void {
  if (pane.socket?.readyState === WebSocket.OPEN) {
    pane.socket.send(encodeControl(message));
  }
}

function sendTerminalInput(pane: TerminalPane, bytes: Uint8Array): void {
  const socket = pane.socket;
  if (!pane.sessionOpen || !socket || socket.readyState !== WebSocket.OPEN) {
    return;
  }
  if (socket.bufferedAmount > SOCKET_INPUT_HIGH_WATER_BYTES) {
    pane.sessionOpen = false;
    setPaneStatus(pane, 'failed', 'Connection backpressure');
    socket.close(1009, 'input backpressure');
    return;
  }
  socket.send(bytes);
}

function parseOsc7Cwd(data: string): string | undefined {
  const candidate = data.includes(';') ? data.slice(data.indexOf(';') + 1) : data;
  try {
    const url = new URL(candidate);
    if (url.protocol !== 'file:') {
      return undefined;
    }
    return decodeURIComponent(url.pathname) || undefined;
  } catch {
    return candidate.startsWith('/') ? candidate : undefined;
  }
}

function connectPane(pane: TerminalPane, reset: boolean): void {
  if (pane.removed) {
    return;
  }
  const previousSocket = pane.socket;
  if (previousSocket && previousSocket.readyState !== WebSocket.CLOSED) {
    if (previousSocket.readyState === WebSocket.OPEN) {
      previousSocket.send(encodeControl({ type: 'close' }));
    }
    previousSocket.close(1000, 'reconnecting');
  }
  pane.socket = null;
  pane.sessionOpen = false;
  pane.closeRequested = false;
  if (reset) {
    pane.terminal.clear();
    pane.terminal.reset();
    pane.bufferRevision += 1;
  }
  setPaneStatus(pane, 'connecting', 'Connecting');
  pane.fitAddon.fit();
  pane.lastSize = { cols: pane.terminal.cols, rows: pane.terminal.rows };

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const nextSocket = new WebSocket(`${protocol}//${window.location.host}/api/v1/terminal`);
  pane.socket = nextSocket;
  nextSocket.binaryType = 'arraybuffer';
  nextSocket.addEventListener('open', () => {
    if (pane.removed || pane.socket !== nextSocket) {
      return;
    }
    sendControl(pane, {
      type: 'open',
      protocol: PROTOCOL_VERSION,
      cols: pane.terminal.cols,
      rows: pane.terminal.rows,
      profile: 'local-default',
    });
  });
  nextSocket.addEventListener('message', (event: MessageEvent<string | ArrayBuffer>) => {
    if (pane.removed || pane.socket !== nextSocket) {
      return;
    }
    if (typeof event.data !== 'string') {
      pane.terminal.write(new Uint8Array(event.data));
      return;
    }
    const message = parseServerMessage(event.data);
    if (!message) {
      setPaneStatus(pane, 'failed', 'Invalid server message');
      return;
    }
    if (message.type === 'opened') {
      pane.sessionOpen = true;
      pane.lastSize = { cols: message.cols, rows: message.rows };
      pane.shell = message.shell;
      pane.cwd = message.cwd;
      updatePaneHeader(pane);
      setPaneStatus(pane, 'open', 'Connected');
      scheduleFit(pane);
      if (pane.id === activePaneId) {
        pane.terminal.focus();
      }
      return;
    }
    if (message.type === 'exit') {
      pane.sessionOpen = false;
      setPaneStatus(pane, 'exited', `Exited (${message.code})`);
      pane.terminal.writeln(`\r\n[process exited with code ${message.code}]`);
      return;
    }
    if (message.fatal) {
      pane.sessionOpen = false;
      setPaneStatus(pane, 'failed', message.message);
      pane.terminal.writeln(`\r\n[${message.code}] ${message.message}`);
    }
  });
  nextSocket.addEventListener('close', () => {
    if (pane.removed || pane.socket !== nextSocket) {
      return;
    }
    pane.socket = null;
    pane.sessionOpen = false;
    if (pane.closeRequested) {
      setPaneStatus(pane, 'exited', 'Closed');
    } else if (pane.state !== 'exited') {
      setPaneStatus(pane, 'failed', 'Disconnected');
    }
  });
  nextSocket.addEventListener('error', () => {
    if (pane.removed || pane.socket !== nextSocket) {
      return;
    }
    if (!pane.closeRequested) {
      setPaneStatus(pane, 'failed', 'Connection failed');
    }
  });
}

function disposePane(pane: TerminalPane): void {
  pane.removed = true;
  pane.sessionOpen = false;
  pane.closeRequested = true;
  const socket = pane.socket;
  pane.socket = null;
  if (socket?.readyState === WebSocket.OPEN) {
    socket.send(encodeControl({ type: 'close' }));
  }
  socket?.close(1000, 'pane closed');
  pane.resizeObserver.disconnect();
  if (pane.resizeFrame !== null) {
    cancelAnimationFrame(pane.resizeFrame);
    pane.resizeFrame = null;
  }
  pane.webglAddon?.dispose();
  pane.fitAddon.dispose();
  pane.terminal.dispose();
  pane.element.remove();
  panes.delete(pane.id);
  workspaces.get(pane.workspaceId)?.paneIds.delete(pane.id);
}

function firstPaneId(node: WorkspaceNode): string {
  return node.kind === 'pane' ? node.paneId : firstPaneId(node.first);
}

function replacePaneNode(node: WorkspaceNode, paneId: string, replacement: WorkspaceNode): WorkspaceNode {
  if (node.kind === 'pane') {
    return node.paneId === paneId ? replacement : node;
  }
  return {
    ...node,
    first: replacePaneNode(node.first, paneId, replacement),
    second: replacePaneNode(node.second, paneId, replacement),
  };
}

function removePaneNode(node: WorkspaceNode, paneId: string): WorkspaceNode | null {
  if (node.kind === 'pane') {
    return node.paneId === paneId ? null : node;
  }
  const first = removePaneNode(node.first, paneId);
  const second = removePaneNode(node.second, paneId);
  if (!first) {
    return second;
  }
  if (!second) {
    return first;
  }
  return { ...node, first, second };
}

function splitChildBasis(ratio: number, first: boolean): string {
  const value = first ? ratio : 1 - ratio;
  return `calc(${Math.round(value * 10000) / 100}% - 3px)`;
}

function renderNode(node: WorkspaceNode): HTMLElement {
  if (node.kind === 'pane') {
    const pane = panes.get(node.paneId);
    if (!pane) {
      throw new Error(`Missing pane ${node.paneId}`);
    }
    return pane.element;
  }
  const split = document.createElement('div');
  split.className = 'split-node';
  split.dataset.splitId = node.id;
  split.dataset.direction = node.direction;
  split.style.setProperty('--split-ratio', String(node.ratio));
  const first = document.createElement('div');
  first.className = 'split-child';
  first.style.flexBasis = splitChildBasis(node.ratio, true);
  first.append(renderNode(node.first));
  const divider = document.createElement('div');
  divider.className = 'split-divider';
  divider.dataset.splitId = node.id;
  divider.setAttribute('role', 'separator');
  divider.setAttribute('aria-orientation', node.direction === 'horizontal' ? 'vertical' : 'horizontal');
  divider.tabIndex = 0;
  divider.addEventListener('pointerdown', (event) => beginDividerDrag(event, node, split, first));
  const second = document.createElement('div');
  second.className = 'split-child';
  second.style.flexBasis = splitChildBasis(node.ratio, false);
  second.append(renderNode(node.second));
  split.append(first, divider, second);
  return split;
}

function renderWorkspace(workspace: Workspace): void {
  workspace.content.replaceChildren(renderNode(workspace.root));
  updateVisiblePanes();
}

function beginDividerDrag(event: PointerEvent, node: SplitNode, split: HTMLElement, first: HTMLElement): void {
  event.preventDefault();
  event.stopPropagation();
  const divider = event.currentTarget as HTMLElement;
  const rect = split.getBoundingClientRect();
  const size = node.direction === 'horizontal' ? rect.width : rect.height;
  if (size <= 0) {
    return;
  }
  divider.dataset.dragging = 'true';
  const update = (moveEvent: PointerEvent) => {
    const offset = node.direction === 'horizontal' ? moveEvent.clientX - rect.left : moveEvent.clientY - rect.top;
    node.ratio = Math.min(0.8, Math.max(0.2, offset / size));
    split.style.setProperty('--split-ratio', String(node.ratio));
    first.style.flexBasis = splitChildBasis(node.ratio, true);
    const second = first.nextElementSibling?.nextElementSibling as HTMLElement | null;
    if (second) {
      second.style.flexBasis = splitChildBasis(node.ratio, false);
    }
    scheduleFunctionalPublish();
  };
  const finish = () => {
    divider.dataset.dragging = 'false';
    window.removeEventListener('pointermove', update);
    window.removeEventListener('pointerup', finish);
    updateVisiblePanes();
  };
  window.addEventListener('pointermove', update);
  window.addEventListener('pointerup', finish, { once: true });
}

function createPaneMenu(pane: TerminalPane, header: HTMLElement, more: HTMLButtonElement): void {
  const menu = document.createElement('div');
  menu.className = 'pane-menu';
  menu.hidden = true;
  const actions: Array<[string, string]> = [
    ['right', 'Split right'],
    ['down', 'Split down'],
    ['three', 'Three-pane layout'],
    ['close', 'Close pane'],
  ];
  for (const [action, label] of actions) {
    const button = document.createElement('button');
    button.type = 'button';
    button.dataset.paneAction = action;
    button.textContent = label;
    button.addEventListener('click', (event) => {
      event.stopPropagation();
      menu.hidden = true;
      const workspace = workspaces.get(pane.workspaceId);
      if (!workspace) {
        return;
      }
      if (action === 'right') {
        splitPane(workspace, pane.id, 'horizontal');
      } else if (action === 'down') {
        splitPane(workspace, pane.id, 'vertical');
      } else if (action === 'three') {
        threePaneLayout(workspace, pane.id);
      } else {
        closePane(pane);
      }
    });
    menu.append(button);
  }
  more.addEventListener('click', (event) => {
    event.stopPropagation();
    setActivePane(pane.id);
    hidePaneMenus(menu);
    menu.hidden = false;
  });
  header.append(menu);
}

function hidePaneMenus(except?: HTMLElement): void {
  document.querySelectorAll<HTMLElement>('.pane-menu').forEach((menu) => {
    if (menu !== except) {
      menu.hidden = true;
    }
  });
}

function createPane(workspace: Workspace): TerminalPane {
  const sequence = ++paneSequence;
  const id = `pane-${sequence}`;
  const paneNumber = workspace.paneIds.size + 1;
  const label = paneNumber === 1 ? workspace.label : `${workspace.label} · Pane ${paneNumber}`;
  const element = document.createElement('article');
  element.className = 'terminal-pane';
  element.id = id;
  element.dataset.paneId = id;
  element.dataset.workspaceId = workspace.id;
  element.setAttribute('role', 'tabpanel');
  const header = document.createElement('header');
  header.className = 'pane-header';
  const titleElement = document.createElement('span');
  titleElement.className = 'pane-title';
  const metaElement = document.createElement('span');
  metaElement.className = 'pane-meta';
  metaElement.hidden = true;
  const more = document.createElement('button');
  more.className = 'pane-more';
  more.type = 'button';
  more.setAttribute('aria-label', `More actions for ${label}`);
  more.title = 'Pane actions';
  more.textContent = '⋯';
  header.append(titleElement, metaElement, more);
  const mount = document.createElement('section');
  mount.className = 'terminal-container';
  mount.setAttribute('aria-label', `${label} terminal`);
  const paneTerminal = document.createElement('div');
  paneTerminal.className = 'pane-terminal';
  paneTerminal.append(mount);
  element.append(header, paneTerminal);
  workspace.content.append(element);

  const terminal = createTerminal();
  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(mount);
  const pane: TerminalPane = {
    id,
    workspaceId: workspace.id,
    label,
    title: label,
    terminal,
    fitAddon,
    element,
    mount,
    titleElement,
    metaElement,
    socket: null,
    sessionOpen: false,
    closeRequested: false,
    removed: false,
    state: 'connecting',
    statusText: 'Connecting',
    bufferRevision: 0,
    lastSize: { cols: 0, rows: 0 },
    resizeFrame: null,
    resizeObserver: new ResizeObserver(() => scheduleFit(pane)),
    rendererMode: 'CPU',
  };
  panes.set(id, pane);
  workspace.paneIds.add(id);

  terminal.attachCustomKeyEventHandler((event) => {
    const pasteShortcut =
      event.type === 'keydown' &&
      event.key.toLowerCase() === 'v' &&
      (event.ctrlKey || event.metaKey) &&
      !event.altKey;
    return !(pasteShortcut || isAppSessionShortcut(event) || isSearchShortcut(event));
  });
  terminal.onData((data) => sendTerminalInput(pane, encodeTerminalInput(data)));
  terminal.onBinary((data) => sendTerminalInput(pane, encodeBinaryInput(data)));
  terminal.onResize(({ cols, rows }) => {
    if (pane.sessionOpen && (cols !== pane.lastSize.cols || rows !== pane.lastSize.rows)) {
      pane.lastSize = { cols, rows };
      sendControl(pane, { type: 'resize', cols, rows });
    }
    updateSessionChrome(pane);
    scheduleFunctionalPublish();
  });
  terminal.onWriteParsed(() => {
    pane.bufferRevision += 1;
    scheduleFunctionalPublish();
  });
  terminal.onTitleChange((title) => {
    const nextTitle = title.trim();
    if (nextTitle) {
      pane.title = nextTitle;
      updatePaneHeader(pane);
    }
  });
  terminal.parser.registerOscHandler(7, (data) => {
    const cwd = parseOsc7Cwd(data);
    if (cwd) {
      pane.cwd = cwd;
      updatePaneHeader(pane);
    }
    return false;
  });
  element.addEventListener('pointerdown', () => setActivePane(pane.id));
  createPaneMenu(pane, header, more);
  updatePaneHeader(pane);
  updateNewSessionState();
  return pane;
}

function createWorkspace(): Workspace | undefined {
  if (panes.size >= MAX_SESSIONS) {
    showNotice('Maximum of 8 terminal panes reached');
    return undefined;
  }
  const sequence = ++workspaceSequence;
  const id = `workspace-${sequence}`;
  const label = sequence === 1 ? 'Local terminal' : `Terminal ${sequence}`;
  const element = document.createElement('section');
  element.className = 'workspace-view';
  element.hidden = true;
  element.id = id;
  const content = document.createElement('div');
  content.className = 'workspace-content';
  element.append(content);
  workspaceRoot.append(element);
  const tab = document.createElement('div');
  tab.className = 'session-tab';
  tab.id = `tab-${id}`;
  tab.dataset.workspaceId = id;
  tab.setAttribute('role', 'tab');
  tab.setAttribute('aria-controls', id);
  tab.setAttribute('aria-label', label);
  const state = document.createElement('span');
  state.className = 'session-tab-state';
  state.setAttribute('aria-hidden', 'true');
  const labelElement = document.createElement('span');
  labelElement.className = 'session-tab-label';
  labelElement.textContent = label;
  const closeButton = document.createElement('button');
  closeButton.className = 'session-tab-close';
  closeButton.type = 'button';
  closeButton.setAttribute('aria-label', `Close ${label}`);
  closeButton.title = `Close ${label}`;
  closeButton.textContent = '×';
  tab.append(state, labelElement, closeButton);
  tabs.append(tab);
  const workspace: Workspace = {
    id,
    label,
    tab,
    labelElement,
    element,
    content,
    root: { kind: 'pane', paneId: '' },
    paneIds: new Set(),
    activePaneId: '',
  };
  workspaces.set(id, workspace);
  const pane = createPane(workspace);
  workspace.root = { kind: 'pane', paneId: pane.id };
  workspace.activePaneId = pane.id;
  updateWorkspaceTabLabel(workspace);
  tab.addEventListener('click', (event) => {
    if ((event.target as HTMLElement).closest('.session-tab-close')) {
      return;
    }
    setActiveWorkspace(workspace.id);
  });
  tab.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      setActiveWorkspace(workspace.id);
    }
  });
  closeButton.addEventListener('click', (event) => {
    event.stopPropagation();
    removeWorkspace(workspace);
  });
  renderWorkspace(workspace);
  return workspace;
}

function startWorkspace(workspace: Workspace): void {
  setActiveWorkspace(workspace.id);
  for (const paneId of workspace.paneIds) {
    const pane = panes.get(paneId);
    if (pane && !pane.socket) {
      connectPane(pane, false);
    }
  }
}

function splitPane(workspace: Workspace, paneId: string, direction: SplitDirection): void {
  if (panes.size >= MAX_SESSIONS) {
    showNotice('Maximum of 8 terminal panes reached');
    return;
  }
  const pane = panes.get(paneId);
  if (!pane) {
    return;
  }
  const newPane = createPane(workspace);
  const replacement: SplitNode = {
    kind: 'split',
    id: `split-${++splitSequence}`,
    direction,
    ratio: 0.5,
    first: { kind: 'pane', paneId },
    second: { kind: 'pane', paneId: newPane.id },
  };
  workspace.root = replacePaneNode(workspace.root, paneId, replacement);
  renderWorkspace(workspace);
  setActivePane(newPane.id);
  connectPane(newPane, false);
}

function threePaneLayout(workspace: Workspace, paneId: string): void {
  if (panes.size + 2 > MAX_SESSIONS) {
    showNotice('Three-pane layout needs two available pane slots');
    return;
  }
  const pane = panes.get(paneId);
  if (!pane) {
    return;
  }
  const top = createPane(workspace);
  const bottom = createPane(workspace);
  const right: SplitNode = {
    kind: 'split',
    id: `split-${++splitSequence}`,
    direction: 'vertical',
    ratio: 0.5,
    first: { kind: 'pane', paneId: top.id },
    second: { kind: 'pane', paneId: bottom.id },
  };
  workspace.root = replacePaneNode(workspace.root, paneId, {
    kind: 'split',
    id: `split-${++splitSequence}`,
    direction: 'horizontal',
    ratio: 0.58,
    first: { kind: 'pane', paneId },
    second: right,
  });
  renderWorkspace(workspace);
  setActivePane(top.id);
  connectPane(top, false);
  connectPane(bottom, false);
}

function closePane(pane: TerminalPane): void {
  const workspace = workspaces.get(pane.workspaceId);
  if (!workspace || pane.removed) {
    return;
  }
  if (workspace.paneIds.size === 1) {
    if (workspaces.size > 1) {
      removeWorkspace(workspace);
      return;
    }
    disposePane(pane);
    const replacement = createPane(workspace);
    workspace.root = { kind: 'pane', paneId: replacement.id };
    workspace.activePaneId = replacement.id;
    renderWorkspace(workspace);
    setActivePane(replacement.id);
    connectPane(replacement, false);
    return;
  }
  const replacement = removePaneNode(workspace.root, pane.id);
  if (!replacement) {
    return;
  }
  const wasActive = pane.id === activePaneId;
  workspace.root = replacement;
  disposePane(pane);
  renderWorkspace(workspace);
  if (wasActive) {
    const nextId = firstPaneId(workspace.root);
    workspace.activePaneId = nextId;
    setActivePane(nextId);
  }
  updateNewSessionState();
}

function removeWorkspace(workspace: Workspace): void {
  const wasActive = workspace.id === activeWorkspaceId;
  const remaining = [...workspaces.values()].filter((candidate) => candidate.id !== workspace.id);
  for (const paneId of [...workspace.paneIds]) {
    const pane = panes.get(paneId);
    if (pane) {
      disposePane(pane);
    }
  }
  workspace.tab.remove();
  workspace.element.remove();
  workspaces.delete(workspace.id);
  if (remaining.length === 0) {
    const replacement = createWorkspace();
    if (replacement) {
      startWorkspace(replacement);
    }
  } else if (wasActive) {
    setActiveWorkspace(remaining[remaining.length - 1].id);
  }
  updateNewSessionState();
}

function applySettingsToPane(pane: TerminalPane): void {
  const previousScrollback = pane.terminal.options.scrollback;
  pane.terminal.options.fontSize = settings.fontSize;
  pane.terminal.options.scrollback = settings.scrollback;
  pane.terminal.options.theme = terminalTheme(settings.theme);
  if (previousScrollback !== settings.scrollback) {
    pane.bufferRevision += 1;
  }
  if (pane.terminal.rows > 0) {
    pane.terminal.refresh(0, pane.terminal.rows - 1);
  }
}

function applySettings(): void {
  applyDocumentTheme();
  for (const pane of panes.values()) {
    applySettingsToPane(pane);
  }
  updateVisiblePanes();
  updateSessionChrome(activePane());
  saveSettings();
}

function syncSettingsForm(): void {
  settingsFontSize.value = String(settings.fontSize);
  settingsTheme.value = settings.theme;
  settingsRenderer.value = settings.renderer;
  settingsScrollback.value = String(settings.scrollback);
}

function toggleHeaderPopover(button: HTMLButtonElement, popover: HTMLElement): void {
  const open = popover.hidden;
  closeHeaderPopovers();
  if (open) {
    popover.hidden = false;
    button.setAttribute('aria-expanded', 'true');
    if (popover === searchPopover) {
      searchInput.focus();
      updateSearchMatches();
    }
  }
}

function closeHeaderPopovers(): void {
  for (const [button, popover] of [
    [searchToggle, searchPopover],
    [splitToggle, splitPopover],
    [settingsToggle, settingsPopover],
  ] as Array<[HTMLButtonElement, HTMLElement]>) {
    popover.hidden = true;
    button.setAttribute('aria-expanded', 'false');
  }
}

function searchLines(pane: TerminalPane, query: string): SearchMatch[] {
  const normalized = query.toLocaleLowerCase();
  const matches: SearchMatch[] = [];
  const buffer = pane.terminal.buffer.active;
  for (let row = 0; row < buffer.length; row += 1) {
    const line = buffer.getLine(row);
    if (!line) {
      continue;
    }
    // `translateToString` is UTF-16 text while Terminal.select expects cell
    // columns. Build a lower-cased search string and map every code unit back
    // to the cell that produced it, preserving CJK, emoji, and combining text.
    let normalizedLine = '';
    const cellColumns: number[] = [];
    for (let cellColumn = 0; cellColumn < line.length; cellColumn += 1) {
      const cell = line.getCell(cellColumn);
      if (!cell) {
        continue;
      }
      const chars = cell.getChars();
      const cellText = chars.length > 0 ? chars : cell.getWidth() === 0 ? '' : ' ';
      const normalizedCellText = cellText.toLocaleLowerCase();
      normalizedLine += normalizedCellText;
      for (let offset = 0; offset < normalizedCellText.length; offset += 1) {
        cellColumns.push(cellColumn);
      }
    }
    let start = 0;
    while (start < normalizedLine.length) {
      const column = normalizedLine.indexOf(normalized, start);
      if (column < 0) {
        break;
      }
      const startColumn = cellColumns[column];
      const lastColumn = cellColumns[column + normalized.length - 1] ?? startColumn;
      const lastCell = line.getCell(lastColumn);
      const length = Math.max(
        1,
        lastColumn - startColumn + Math.max(1, lastCell?.getWidth() ?? 1),
      );
      matches.push({ paneId: pane.id, row, column: startColumn, length });
      start = column + Math.max(1, normalized.length);
    }
  }
  return matches;
}

function selectSearchMatch(): void {
  const pane = activePane();
  const match = searchMatches[searchIndex];
  if (!pane || !match || match.paneId !== pane.id) {
    pane?.terminal.clearSelection();
    searchCount.textContent = searchMatches.length === 0 ? 'No matches' : `${searchMatches.length} matches`;
    return;
  }
  pane.terminal.scrollToLine(match.row);
  pane.terminal.select(match.column, match.row, match.length);
  searchCount.textContent = `${searchIndex + 1} / ${searchMatches.length}`;
}

function updateSearchMatches(direction = 1): void {
  const pane = activePane();
  const query = searchInput.value;
  if (!pane || query.length === 0) {
    searchMatches = [];
    searchIndex = -1;
    searchPaneId = pane?.id ?? null;
    searchRevision = pane?.bufferRevision ?? -1;
    searchQuery = query.length === 0 ? '' : null;
    searchInput.dataset.query = searchQuery ?? '';
    searchCount.textContent = 'No matches';
    pane?.terminal.clearSelection();
    return;
  }
  const needsRefresh =
    searchPaneId !== pane.id ||
    searchQuery !== query ||
    searchRevision !== pane.bufferRevision;
  if (needsRefresh) {
    searchMatches = searchLines(pane, query);
    searchPaneId = pane.id;
    searchRevision = pane.bufferRevision;
    searchQuery = query;
    searchInput.dataset.query = query;
    searchIndex = direction > 0 ? 0 : Math.max(0, searchMatches.length - 1);
  } else if (searchMatches.length > 0) {
    searchIndex = (searchIndex + direction + searchMatches.length) % searchMatches.length;
  }
  selectSearchMatch();
}

function functionalSnapshot() {
  const pane = activePane();
  const terminal = pane?.terminal;
  const buffer = terminal?.buffer.active;
  const lines: string[] = [];
  if (buffer) {
    for (let index = 0; index < buffer.length; index += 1) {
      lines.push(buffer.getLine(index)?.translateToString(true) ?? '');
    }
  }
  const selection = terminal?.getSelectionPosition();
  return Object.freeze({
    schema: 1,
    terminalText: lines.join('\n'),
    cursorRow: buffer?.cursorY ?? 0,
    cursorColumn: buffer?.cursorX ?? 0,
    cols: terminal?.cols ?? 0,
    rows: terminal?.rows ?? 0,
    windowWidth: window.innerWidth,
    windowHeight: window.innerHeight,
    connectionState: pane?.state ?? 'unknown',
    activeSessionId: pane?.id ?? null,
    sessionCount: panes.size,
    workspaceCount: workspaces.size,
    selection: selection
      ? {
          start: { x: selection.start.x, y: selection.start.y },
          end: { x: selection.end.x, y: selection.end.y },
        }
      : null,
  });
}

function scheduleFunctionalPublish(): void {
  if (import.meta.env.MODE !== 'functional' || !isTauri() || functionalPublishFrame !== null) {
    return;
  }
  functionalPublishFrame = requestAnimationFrame(() => {
    functionalPublishFrame = null;
    void invoke('functional_observer_publish', { snapshot: functionalSnapshot() }).catch(() => {});
  });
}

newSessionButton.addEventListener('click', () => {
  const workspace = createWorkspace();
  if (workspace) {
    startWorkspace(workspace);
  }
});

restart.addEventListener('click', () => {
  const pane = activePane();
  if (pane) {
    connectPane(pane, true);
  }
});

searchToggle.addEventListener('click', () => toggleHeaderPopover(searchToggle, searchPopover));
splitToggle.addEventListener('click', () => toggleHeaderPopover(splitToggle, splitPopover));
settingsToggle.addEventListener('click', () => toggleHeaderPopover(settingsToggle, settingsPopover));
searchInput.addEventListener('input', () => updateSearchMatches(1));
searchInput.addEventListener('keydown', (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    updateSearchMatches(event.shiftKey ? -1 : 1);
  } else if (event.key === 'Escape') {
    event.preventDefault();
    closeHeaderPopovers();
    activePane()?.terminal.focus();
  }
});
searchPrevious.addEventListener('click', () => updateSearchMatches(-1));
searchNext.addEventListener('click', () => updateSearchMatches(1));
splitPopover.querySelectorAll<HTMLButtonElement>('[data-split-action]').forEach((button) => {
  button.addEventListener('click', () => {
    const workspace = activeWorkspace();
    const pane = activePane();
    if (!workspace || !pane) {
      return;
    }
    closeHeaderPopovers();
    if (button.dataset.splitAction === 'right') {
      splitPane(workspace, pane.id, 'horizontal');
    } else if (button.dataset.splitAction === 'down') {
      splitPane(workspace, pane.id, 'vertical');
    } else {
      threePaneLayout(workspace, pane.id);
    }
  });
});
settingsFontSize.addEventListener('change', () => {
  const value = Number(settingsFontSize.value);
  settings.fontSize = Math.min(24, Math.max(10, Number.isFinite(value) ? Math.round(value) : DEFAULT_SETTINGS.fontSize));
  settingsFontSize.value = String(settings.fontSize);
  applySettings();
});
settingsTheme.addEventListener('change', () => {
  if (settingsTheme.value === 'dark' || settingsTheme.value === 'light' || settingsTheme.value === 'midnight') {
    settings.theme = settingsTheme.value;
    applySettings();
  }
});
settingsRenderer.addEventListener('change', () => {
  if (settingsRenderer.value === 'auto' || settingsRenderer.value === 'gpu' || settingsRenderer.value === 'cpu') {
    settings.renderer = settingsRenderer.value;
    applySettings();
  }
});
settingsScrollback.addEventListener('change', () => {
  const value = Number(settingsScrollback.value);
  if (validScrollback.has(value)) {
    settings.scrollback = value;
    applySettings();
  }
});

window.addEventListener('resize', () => {
  for (const pane of panes.values()) {
    if (isPaneVisible(pane)) {
      scheduleFit(pane);
    }
  }
  scheduleFunctionalPublish();
});

window.addEventListener('pointerdown', (event) => {
  const target = event.target as HTMLElement;
  if (!target.closest('.header-tools')) {
    closeHeaderPopovers();
  }
  if (!target.closest('.pane-menu') && !target.closest('.pane-more')) {
    hidePaneMenus();
  }
});

window.addEventListener('keydown', (event) => {
  if (isSearchShortcut(event)) {
    event.preventDefault();
    closeHeaderPopovers();
    searchPopover.hidden = false;
    searchToggle.setAttribute('aria-expanded', 'true');
    searchInput.focus();
    updateSearchMatches();
    return;
  }
  if (!isAppSessionShortcut(event)) {
    return;
  }
  const key = event.key.toLowerCase();
  if (key === 't') {
    event.preventDefault();
    newSessionButton.click();
  } else if (key === 'w') {
    const workspace = activeWorkspace();
    if (workspace) {
      event.preventDefault();
      closePane(activePane() ?? panes.get(workspace.activePaneId)!);
    }
  }
});

window.addEventListener('beforeunload', () => {
  for (const pane of panes.values()) {
    const socket = pane.socket;
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(encodeControl({ type: 'close' }));
    }
    socket?.close(1000, 'window closed');
  }
});

applyDocumentTheme();
syncSettingsForm();
if (import.meta.env.MODE === 'functional') {
  Object.defineProperty(window, '__RTERM_FUNCTIONAL_SNAPSHOT__', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: functionalSnapshot,
  });
}

const firstWorkspace = createWorkspace();
if (firstWorkspace) {
  startWorkspace(firstWorkspace);
}
