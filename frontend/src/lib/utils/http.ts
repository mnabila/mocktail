// Shared HTTP method badges, status color tones, and request log code generators.

import type { RequestLog } from '$lib/types/domain';

export const methodClass = (m?: string | null): string =>
  `method method-${(m ?? 'get').toLowerCase()}`;

export const methodVar = (m?: string | null): string => `var(--m-${(m ?? 'get').toLowerCase()})`;

export const methodTextColor = (m?: string | null): string =>
  `text-[var(--m-${(m || 'get').toLowerCase()})]`;

export const statusTone = (s: number): string =>
  s >= 500
    ? 'text-danger'
    : s >= 400
      ? 'text-[var(--m-put)]'
      : s >= 200 && s < 300
        ? 'text-accent'
        : 'text-muted';

export function logUrl(wsSlug: string, l: Pick<RequestLog, 'path'>): string {
  if (typeof location === 'undefined') return l.path;
  return `${location.protocol}//${wsSlug}.${location.host}${l.path}`;
}

// Shell single-quote escaping: close quote, add escaped ', reopen.
const shq = (s: unknown): string => `'${String(s).replace(/'/g, `'\\''`)}'`;

const bodyStr = (b: unknown): string =>
  b == null ? '' : typeof b === 'string' ? b : JSON.stringify(b);

const hasBody = (l: Pick<RequestLog, 'method' | 'requestBody'>): boolean =>
  Boolean(bodyStr(l.requestBody)) && !['GET', 'HEAD'].includes((l.method ?? '').toUpperCase());

export function toCurl(wsSlug: string, l: RequestLog): string {
  const parts = [`curl -X ${l.method ?? 'GET'} ${shq(logUrl(wsSlug, l))}`];
  for (const [k, v] of Object.entries(l.requestHeaders ?? {}))
    parts.push(`  -H ${shq(`${k}: ${v}`)}`);
  if (hasBody(l)) parts.push(`  --data-raw ${shq(bodyStr(l.requestBody))}`);
  return parts.join(' \\\n');
}

export function toFetch(wsSlug: string, l: RequestLog): string {
  const opts: { method: string; headers?: Record<string, string>; body?: string } = {
    method: l.method ?? 'GET'
  };
  if (l.requestHeaders && Object.keys(l.requestHeaders).length) opts.headers = l.requestHeaders;
  if (hasBody(l)) opts.body = bodyStr(l.requestBody);
  return `fetch(${JSON.stringify(logUrl(wsSlug, l))}, ${JSON.stringify(opts, null, 2)});`;
}
