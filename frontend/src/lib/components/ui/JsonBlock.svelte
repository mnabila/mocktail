<script lang="ts">
  import { toast } from '$lib/state/toast.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    title: string;
    value: unknown;
    empty?: string;
  }

  // value: an object/array (pretty-printed) or a raw string (shown as-is, but
  // parsed to JSON first when it happens to be a JSON document).
  let { title, value, empty = '— (none)' }: Props = $props();

  let text = $derived(format(value));
  let isEmpty = $derived(value === null || value === undefined || value === '');

  function format(v: unknown): string {
    if (v === null || v === undefined || v === '') return '';
    if (typeof v === 'string') {
      try {
        return JSON.stringify(JSON.parse(v), null, 2);
      } catch {
        return v;
      }
    }
    return JSON.stringify(v, null, 2);
  }
</script>

<div class="field">
  <div class="flex items-center justify-between">
    <span class="label">{title}</span>
    {#if !isEmpty}
      <button
        class="text-faint transition-colors hover:text-fg"
        onclick={() => {
          navigator.clipboard?.writeText(text);
          toast(`${title} copied`);
        }}
        aria-label={`Copy ${title}`}
      >
        <Icon name="copy" size={13} />
      </button>
    {/if}
  </div>
  {#if isEmpty}
    <p class="text-sm text-faint">{empty}</p>
  {:else}
    <pre class="jsonblock mono">{text}</pre>
  {/if}
</div>

<style>
  .jsonblock {
    margin-top: 0.25rem;
    max-height: 16rem;
    overflow: auto;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--surface-2);
    padding: 0.625rem 0.75rem;
    font-size: 12px;
    line-height: 1.55;
    color: var(--fg);
    white-space: pre;
    tab-size: 2;
  }
</style>
