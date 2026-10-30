<script lang="ts">
  import type { MockEditorDraft } from '$lib/types/domain';
  import { methodVar } from '$lib/utils/http';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select from '$lib/components/ui/Select.svelte';

  interface Props {
    editing: MockEditorDraft;
    onSave: () => void | Promise<void>;
  }

  let { editing = $bindable(), onSave }: Props = $props();
</script>

<div class="reqbar mt-4">
  <div class="reqbar-method-wrap" style="color: {methodVar(editing.method)}">
    <Select
      bind:value={editing.method}
      options={['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD']}
    />
  </div>
  <span class="reqbar-sep"></span>
  <input
    class="reqbar-url mono"
    placeholder="/todos/{'{id}'}"
    bind:value={editing.pathTemplate}
    aria-label="Path template"
  />
  <button class="reqbar-send" onclick={onSave}
    ><Icon name="check" size={15} /> <span class="reqbar-send-label">Save</span></button
  >
</div>
<p class="mt-1.5 text-xs text-faint">
  Requests matching this method + path template return the response below.
</p>

<style>
  .reqbar {
    display: flex;
    align-items: stretch;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--surface-2);
  }
  .reqbar:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 25%, transparent);
  }
  .reqbar-method-wrap {
    display: flex;
    align-items: center;
  }
  .reqbar-method-wrap :global(.sel) {
    width: auto;
  }
  .reqbar-method-wrap :global(.sel-trigger) {
    height: auto;
    border: 0;
    border-radius: 0.4375rem 0 0 0.4375rem;
    background: transparent;
    color: inherit;
    font-size: 0.8125rem;
    font-weight: 600;
    padding: 0.625rem 0.875rem;
    box-shadow: none;
  }
  .reqbar-method-wrap :global(.sel-trigger:hover) {
    border-color: transparent;
  }
  .reqbar-method-wrap :global(.sel-menu) {
    min-width: 8rem;
  }
  .reqbar-sep {
    width: 1px;
    align-self: stretch;
    background: var(--border);
  }
  .reqbar-url {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    padding: 0.625rem 0.875rem;
    font-size: 0.8125rem;
    color: var(--fg);
  }
  .reqbar-url:focus {
    outline: none;
  }
  .reqbar-send {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    border: 0;
    border-left: 1px solid var(--border);
    border-radius: 0 0.4375rem 0.4375rem 0;
    background: var(--accent);
    padding: 0 1rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--accent-fg, #fff);
    cursor: pointer;
  }
  .reqbar-send:hover {
    filter: brightness(1.05);
  }
  /* tight screens: drop the "Save" label and trim padding so the path field keeps room */
  @media (max-width: 420px) {
    .reqbar-send-label {
      display: none;
    }
    .reqbar-url {
      padding-inline: 0.5rem;
    }
    .reqbar-send {
      padding-inline: 0.75rem;
    }
    .reqbar-method-wrap :global(.sel-trigger) {
      padding-inline: 0.5rem;
    }
  }
</style>
