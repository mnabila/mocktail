// Thin client for the mocktail admin API (/__admin). Sends the session cookie
// (same-origin) and a CSRF token on mutations.

import type { MeResponse } from '$lib/types/domain';

const BASE = '/__admin';
let csrf = '';

export function setCsrf(token: string): void {
  csrf = token;
}

function mutating(method: string): boolean {
  return method === 'POST' || method === 'PUT' || method === 'PATCH' || method === 'DELETE';
}

interface Envelope<T = any> {
  status: { code: string; message: string };
  data: T;
  metadata?: Record<string, unknown>;
}

// Dashboard API responses are wrapped as
// `{ status: { code, message }, data, metadata }`; pages consume `data`.
function unwrap<T = any>(payload: unknown): T {
  if (
    payload &&
    typeof payload === 'object' &&
    'status' in payload &&
    payload.status &&
    typeof payload.status === 'object' &&
    'code' in payload.status &&
    'data' in payload
  ) {
    return (payload as Envelope<T>).data;
  }
  return payload as T;
}

async function request<T = any>(
  method: string,
  path: string,
  body?: unknown,
  raw?: string
): Promise<T> {
  const headers: Record<string, string> = {};
  if (mutating(method)) headers['X-CSRF-Token'] = csrf;
  let payload: string | undefined;
  if (raw !== undefined) {
    headers['Content-Type'] = 'application/json';
    payload = raw;
  } else if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
    payload = JSON.stringify(body);
  }
  const res = await fetch(BASE + path, {
    method,
    headers,
    credentials: 'same-origin',
    body: payload
  });
  const text = await res.text();
  const data = text ? JSON.parse(text) : null;
  if (!res.ok) {
    throw {
      status: res.status,
      error:
        data?.status?.message ??
        data?.error?.message ??
        (res.status === 401 ? 'unauthorized' : res.statusText)
    };
  }
  return unwrap<T>(data);
}

export const api = {
  get: <T = any>(p: string): Promise<T> => request<T>('GET', p),
  post: <T = any>(p: string, b?: unknown): Promise<T> => request<T>('POST', p, b),
  patch: <T = any>(p: string, b?: unknown): Promise<T> => request<T>('PATCH', p, b),
  put: <T = any>(p: string, b?: unknown): Promise<T> => request<T>('PUT', p, b),
  del: <T = any>(p: string): Promise<T> => request<T>('DELETE', p),
  /** POST a raw string body (e.g. an uploaded OpenAPI document). */
  postRaw: <T = any>(p: string, s: string): Promise<T> => request<T>('POST', p, undefined, s)
};

/** Fetch the current user; also primes the CSRF token. */
export async function loadMe(): Promise<MeResponse> {
  const me = await api.get<MeResponse>('/me');
  setCsrf(me.csrf);
  return me;
}
