<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  interface Props {
    title: string;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
    size?: 'md' | 'lg' | 'xl';
    overflowVisible?: boolean;
  }

  let { title, onClose, children, footer, size = 'lg', overflowVisible = false }: Props = $props();
  const widths: Record<'md' | 'lg' | 'xl', string> = {
    md: 'max-w-lg',
    lg: 'max-w-2xl',
    xl: 'max-w-4xl'
  };

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="modal-root">
  <div
    class="modal-scrim"
    role="button"
    tabindex="-1"
    aria-label="Close"
    onclick={onClose}
    onkeydown={(e) => e.key === 'Enter' && onClose()}
  ></div>
  <div class="modal-panel {widths[size]} {overflowVisible ? 'overflow-visible' : 'overflow-hidden'}" role="dialog" aria-modal="true" aria-label={title}>
    <header class="modal-head">
      <h2 class="font-medium text-fg">{title}</h2>
      <button class="btn btn-ghost btn-sm" onclick={onClose} aria-label="Close">
        <Icon name="plus" size={16} class="rotate-45" />
      </button>
    </header>
    <div class="modal-body" class:overflow-visible={overflowVisible}>
      {@render children()}
    </div>
    {#if footer}
      <footer class="modal-foot">
        {@render footer()}
      </footer>
    {/if}
  </div>
</div>

<style>
  .modal-root {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
  }
  .modal-scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    animation: m-fade 150ms ease-out;
  }
  .modal-panel {
    position: relative;
    display: flex;
    max-height: 90vh;
    width: 100%;
    flex-direction: column;
    border-radius: 0.75rem;
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-md);
    animation: m-pop 180ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  .modal-panel.overflow-hidden {
    overflow: hidden;
  }
  .modal-panel.overflow-visible {
    overflow: visible;
  }
  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
    padding: 1rem 1.25rem;
  }
  .modal-body {
    overflow-y: auto;
    padding: 1.25rem;
  }
  .modal-body.overflow-visible {
    overflow: visible;
  }
  .modal-foot {
    display: flex;
    gap: 0.5rem;
    border-top: 1px solid var(--border);
    background: var(--surface);
    padding: 0.875rem 1.25rem;
  }
  @keyframes m-fade {
    from {
      opacity: 0;
    }
  }
  @keyframes m-pop {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .modal-scrim,
    .modal-panel {
      animation: none;
    }
  }
</style>
