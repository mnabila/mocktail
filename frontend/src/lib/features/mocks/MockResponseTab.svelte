<script module lang="ts">
  export const CONTENT_TYPES = [
    { label: 'JSON', value: 'application/json' },
    { label: 'Text', value: 'text/plain' },
    { label: 'HTML', value: 'text/html' },
    { label: 'XML', value: 'application/xml' },
    { label: 'JavaScript', value: 'application/javascript' },
    { label: 'CSS', value: 'text/css' },
    { label: 'Binary', value: 'application/octet-stream' }
  ] as const;
  export const PRESET_TYPES: readonly string[] = CONTENT_TYPES.map((t) => t.value);
</script>

<script lang="ts">
  import { toast } from '$lib/state/toast.svelte';
  import type { MockEditorDraft } from '$lib/types/domain';
  import Select from '$lib/components/ui/Select.svelte';

  interface Props {
    editing: MockEditorDraft;
  }

  let { editing = $bindable() }: Props = $props();

  function formatBody() {
    if (editing.response.contentType !== 'application/json') return;
    try {
      editing.response.body = JSON.stringify(JSON.parse(editing.response.body), null, 2);
    } catch {
      toast('Body is not valid JSON', 'error');
    }
  }

  function onTypeChange(v: string) {
    if (v === 'custom') {
      editing.response.customType = true;
      editing.response.contentType = '';
    } else {
      editing.response.customType = false;
      editing.response.contentType = v;
    }
  }
</script>

<div class="grid gap-3 sm:grid-cols-2">
  <div class="field">
    <label class="label" for="e-status">Status code</label>
    <input id="e-status" type="number" class="input" bind:value={editing.response.status} />
  </div>
  <div class="field">
    <label class="label" for="e-type">Response type</label>
    <Select
      id="e-type"
      value={editing.response.customType ? 'custom' : editing.response.contentType}
      onchange={onTypeChange}
      options={[...CONTENT_TYPES, { value: 'custom', label: 'Custom…' }]}
    />
  </div>
</div>
{#if editing.response.customType}
  <div class="field mt-3">
    <label class="label" for="e-ctype">Content-Type</label>
    <input
      id="e-ctype"
      class="input mono"
      placeholder="application/vnd.api+json"
      bind:value={editing.response.contentType}
    />
  </div>
{/if}
<div class="field mt-3">
  <div class="flex items-center justify-between">
    <label class="label" for="e-body">Body</label>
    {#if editing.response.contentType === 'application/json'}
      <button class="text-xs text-link hover:underline" onclick={formatBody}>Format JSON</button>
    {/if}
  </div>
  <textarea id="e-body" class="textarea mono h-72 text-xs" bind:value={editing.response.body}
  ></textarea>
  <p class="text-xs text-faint">
    Templating: <span class="mono">{`{{param}}`}</span>
    <span class="mono">{`{{query.x}}`}</span>
    <span class="mono">{`{{header.x}}`}</span>
    <span class="mono">{`{{body}}`}</span>
  </p>
</div>
