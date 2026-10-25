<script lang="ts">
  import { toasts, dismiss } from '$lib/state/toast.svelte';
  import Icon from './Icon.svelte';
</script>

<div
  class="pointer-events-none fixed inset-x-0 bottom-0 z-[1000] flex flex-col items-center gap-2 p-4"
  aria-live="polite"
  aria-atomic="false"
>
  {#each toasts as t (t.id)}
    <div
      class="toast pointer-events-auto flex items-center gap-2.5 rounded-lg border px-3.5 py-2.5 text-sm shadow-lg"
      class:toast-error={t.type === 'error'}
      role={t.type === 'error' ? 'alert' : 'status'}
    >
      <span class="shrink-0 {t.type === 'error' ? 'text-danger' : 'text-accent'}">
        <Icon name={t.type === 'error' ? 'alert' : 'check'} size={16} />
      </span>
      <span class="text-fg">{t.message}</span>
      <button
        class="ml-1 text-faint hover:text-fg"
        onclick={() => dismiss(t.id)}
        aria-label="Dismiss"
      >
        <Icon name="plus" size={14} class="rotate-45" />
      </button>
    </div>
  {/each}
</div>

<style>
  .toast {
    background: var(--surface);
    border-color: var(--border-strong);
    animation: toast-in 180ms ease-out;
  }
  .toast-error {
    border-color: color-mix(in srgb, var(--danger) 55%, transparent);
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }
</style>
