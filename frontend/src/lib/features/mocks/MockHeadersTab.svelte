<script lang="ts">
  import type { MockEditorDraft } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  interface Props {
    editing: MockEditorDraft;
  }

  let { editing = $bindable() }: Props = $props();

  function addHeader() {
    editing.response.headers.push({ name: '', value: '' });
  }
  function removeHeader(i: number) {
    editing.response.headers.splice(i, 1);
  }
</script>

<div class="flex flex-col items-start gap-2 sm:flex-row sm:items-center sm:justify-between">
  <p class="text-xs text-muted">
    Extra headers sent with the response. <span class="mono">Content-Type</span> comes from the response
    type.
  </p>
  <button class="btn btn-secondary btn-sm" onclick={addHeader}
    ><Icon name="plus" size={14} /> Add header</button
  >
</div>
{#if editing.response.headers.length}
  <div class="mt-3 overflow-hidden rounded-lg border border-border">
    <div class="grid grid-cols-[1fr_1fr_auto] gap-px bg-border">
      <div class="bg-surface-2 px-3 py-2 text-xs font-medium text-faint">Key</div>
      <div class="bg-surface-2 px-3 py-2 text-xs font-medium text-faint">Value</div>
      <div class="bg-surface-2 px-3 py-2"></div>
      {#each editing.response.headers as h, i (i)}
        <input class="hdr-input mono bg-surface" placeholder="Header-Name" bind:value={h.name} />
        <input class="hdr-input mono bg-surface" placeholder="value" bind:value={h.value} />
        <button
          class="flex items-center justify-center bg-surface px-2 text-faint hover:text-danger"
          onclick={() => removeHeader(i)}
          aria-label="Remove header"
        >
          <Icon name="trash" size={15} />
        </button>
      {/each}
    </div>
  </div>
{:else}
  <div
    class="mt-3 flex flex-col items-center gap-2 rounded-lg border border-dashed border-border py-10 text-center"
  >
    <span class="text-faint"><Icon name="layers" size={24} /></span>
    <p class="text-sm text-muted">No custom headers.</p>
  </div>
{/if}

<style>
  .hdr-input {
    min-width: 0;
    border: 0;
    padding: 0.625rem 0.875rem;
    font-size: 0.8125rem;
    color: var(--fg);
  }
  .hdr-input:focus {
    outline: none;
  }
</style>
