<script lang="ts">
  import type { SelectOption } from '$lib/types/domain';
  import Icon from './Icon.svelte';

  interface NormalizedOption {
    value: any;
    label: string;
  }

  interface Props {
    value?: any;
    options?: SelectOption[];
    placeholder?: string;
    searchable?: boolean;
    disabled?: boolean;
    size?: 'md' | 'sm';
    id?: string;
    onchange?: (value: any) => void;
    class?: string;
  }

  let {
    value = $bindable(),
    options = [],
    placeholder = 'Select…',
    searchable = false,
    disabled = false,
    size = 'md',
    id = undefined,
    onchange = undefined,
    class: klass = ''
  }: Props = $props();

  // Normalize to { value, label }; bare strings/numbers become their own label.
  let opts = $derived<NormalizedOption[]>(
    options.map((o) =>
      o && typeof o === 'object' && 'value' in o
        ? { value: o.value, label: o.label ?? String(o.value) }
        : { value: o, label: String(o) }
    )
  );

  let open = $state(false);
  let query = $state('');
  let active = $state(-1);
  let root = $state<HTMLDivElement | null>(null);
  let input = $state<HTMLInputElement | null>(null);

  let selected = $derived(opts.find((o) => o.value === value) ?? null);
  let shown = $derived(
    searchable && query.trim()
      ? opts.filter((o) => o.label.toLowerCase().includes(query.trim().toLowerCase()))
      : opts
  );

  function openMenu() {
    if (disabled) return;
    open = true;
    query = '';
    active = shown.findIndex((o) => o.value === value);
    queueMicrotask(() => input?.focus());
  }
  function close() {
    open = false;
    query = '';
    active = -1;
  }
  function pick(o: NormalizedOption) {
    value = o.value;
    onchange?.(o.value);
    close();
  }
  function onKey(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'Enter' || e.key === 'ArrowDown' || e.key === ' ') {
        e.preventDefault();
        openMenu();
      }
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = Math.min(active + 1, shown.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (shown[active]) pick(shown[active]);
    }
  }
  function onWindowClick(e: MouseEvent) {
    if (open && root && e.target instanceof Node && !root.contains(e.target)) close();
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="sel {klass}" class:sel-open={open} class:sel-disabled={disabled} bind:this={root}>
  <button
    type="button"
    {id}
    class="sel-trigger"
    class:sel-sm={size === 'sm'}
    aria-haspopup="listbox"
    aria-expanded={open}
    {disabled}
    onclick={() => (open ? close() : openMenu())}
    onkeydown={onKey}
  >
    {#if open && searchable}
      <input
        bind:this={input}
        class="sel-input"
        placeholder={selected ? selected.label : placeholder}
        bind:value={query}
        onkeydown={onKey}
        onclick={(e) => e.stopPropagation()}
      />
      <Icon name="search" size={15} class="sel-icon" />
    {:else}
      <span class="sel-value" class:sel-placeholder={!selected}>
        {selected ? selected.label : placeholder}
      </span>
      <Icon name="chevron-down" size={15} class="sel-chev" />
    {/if}
  </button>

  {#if open}
    <ul class="sel-menu" role="listbox" tabindex="-1">
      {#each shown as o, i (o.value)}
        <li>
          <button
            type="button"
            class="sel-opt"
            class:sel-opt-active={i === active}
            class:sel-opt-selected={o.value === value}
            role="option"
            aria-selected={o.value === value}
            onmouseenter={() => (active = i)}
            onclick={() => pick(o)}
          >
            <span class="truncate">{o.label}</span>
            {#if o.value === value}<Icon name="check" size={15} />{/if}
          </button>
        </li>
      {:else}
        <li class="sel-empty">No matches</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    width: 100%;
  }
  .sel-trigger {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    height: 2.25rem;
    padding: 0 0.625rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border-strong, var(--border));
    background: var(--surface);
    color: var(--fg);
    font-size: 0.875rem;
    text-align: left;
    cursor: pointer;
    transition:
      border-color 150ms,
      box-shadow 150ms;
  }
  .sel-trigger.sel-sm {
    height: 2rem;
    font-size: 0.8125rem;
  }
  .sel-trigger:hover {
    border-color: var(--accent);
  }
  .sel-open .sel-trigger {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .sel-disabled .sel-trigger {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .sel-value {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sel-placeholder {
    color: var(--faint);
  }
  :global(.sel-chev) {
    color: var(--muted);
    flex-shrink: 0;
    transition: transform 150ms;
  }
  .sel-open :global(.sel-chev) {
    transform: rotate(180deg);
  }
  .sel-input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--fg);
    font-size: inherit;
    outline: none;
  }
  .sel-input::placeholder {
    color: var(--faint);
  }
  :global(.sel-icon) {
    color: var(--muted);
    flex-shrink: 0;
  }
  .sel-menu {
    position: absolute;
    z-index: 95;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    max-height: 16rem;
    overflow-y: auto;
    padding: 0.25rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-md);
    animation: sel-pop 140ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  .sel-opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    width: 100%;
    padding: 0.4375rem 0.5rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    color: var(--fg);
    cursor: pointer;
    text-align: left;
  }
  .sel-opt-active {
    background: var(--surface-hover);
  }
  .sel-opt-selected {
    color: var(--accent);
    font-weight: 500;
  }
  .sel-empty {
    padding: 0.5rem;
    text-align: center;
    font-size: 0.8125rem;
    color: var(--faint);
  }
  @keyframes sel-pop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .sel-menu {
      animation: none;
    }
  }
</style>
