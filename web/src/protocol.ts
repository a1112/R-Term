export const PROTOCOL_VERSION = 1;

export type ClientMessage =
  | {
      type: 'open';
      protocol: number;
      cols: number;
      rows: number;
      profile: string;
    }
  | { type: 'resize'; cols: number; rows: number }
  | { type: 'close' };

export type ServerMessage =
  | {
      type: 'opened';
      protocol: number;
      sessionId: string;
      cols: number;
      rows: number;
      shell?: string;
      cwd?: string;
    }
  | { type: 'exit'; code: number; signal: string | null }
  | { type: 'error'; code: string; message: string; fatal: boolean };

export function encodeControl(message: ClientMessage): string {
  return JSON.stringify(message);
}

export function parseServerMessage(data: string): ServerMessage | null {
  try {
    const message: unknown = JSON.parse(data);
    if (!isRecord(message) || typeof message.type !== 'string') {
      return null;
    }
    if (message.type === 'opened') {
      if (!(isNumber(message.protocol) &&
        typeof message.sessionId === 'string' &&
        isNumber(message.cols) &&
        isNumber(message.rows))) {
        return null;
      }
      const opened: ServerMessage & { type: 'opened' } = {
        type: 'opened',
        protocol: message.protocol,
        sessionId: message.sessionId,
        cols: message.cols,
        rows: message.rows,
      };
      if (isNonEmptyString(message.shell)) {
        opened.shell = message.shell;
      }
      if (isNonEmptyString(message.cwd)) {
        opened.cwd = message.cwd;
      }
      return opened;
    }
    if (message.type === 'exit') {
      return isNumber(message.code) &&
        (message.signal === null || typeof message.signal === 'string')
        ? { type: 'exit', code: message.code, signal: message.signal }
        : null;
    }
    if (message.type === 'error') {
      return typeof message.code === 'string' &&
        typeof message.message === 'string' &&
        typeof message.fatal === 'boolean'
        ? {
            type: 'error',
            code: message.code,
            message: message.message,
            fatal: message.fatal,
          }
        : null;
    }
    return null;
  } catch {
    return null;
  }
}

export function encodeTerminalInput(data: string): Uint8Array {
  return new TextEncoder().encode(data);
}

export function encodeBinaryInput(data: string): Uint8Array {
  const bytes = new Uint8Array(data.length);
  for (let index = 0; index < data.length; index += 1) {
    bytes[index] = data.charCodeAt(index) & 0xff;
  }
  return bytes;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}
