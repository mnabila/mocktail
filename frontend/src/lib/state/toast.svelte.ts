// Minimal toast store (Svelte 5 runes). Non-focus-stealing status messages;
// the <Toasts /> container renders with aria-live=polite (WCAG toast rules).

import type { ToastItem, ToastType } from '$lib/types/domain';

let seq = 0;
export const toasts = $state<ToastItem[]>([]);

export function toast(message: string, type: ToastType = 'success', ttl = 3000): number {
  const id = ++seq;
  toasts.push({ id, message, type });
  setTimeout(() => dismiss(id), ttl);
  return id;
}

export function dismiss(id: number): void {
  const i = toasts.findIndex((t) => t.id === id);
  if (i !== -1) toasts.splice(i, 1);
}
