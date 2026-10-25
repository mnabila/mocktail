<script lang="ts">
  import type { MockEditorDraft } from '$lib/types/domain';
  import Select from '$lib/components/ui/Select.svelte';

  interface Props {
    editing: MockEditorDraft;
  }

  let { editing = $bindable() }: Props = $props();
</script>

<div class="grid gap-3 sm:grid-cols-2">
  <div class="field">
    <label class="label" for="e-pri">Priority</label>
    <input id="e-pri" type="number" class="input" bind:value={editing.priority} />
    <p class="text-xs text-faint">Higher wins when multiple mocks match.</p>
  </div>
  <div class="field">
    <label class="label" for="e-delay">Delay (ms)</label>
    <input id="e-delay" type="number" class="input" bind:value={editing.delayMs} />
    <p class="text-xs text-faint">Artificial latency before responding.</p>
  </div>
</div>
<div class="field mt-3">
  <label class="label" for="e-fault">Fault injection</label>
  <Select
    id="e-fault"
    bind:value={editing.fault}
    options={[
      { value: null, label: 'None' },
      { value: 'connectionReset', label: 'connectionReset' },
      { value: 'malformedResponse', label: 'malformedResponse' }
    ]}
  />
</div>
