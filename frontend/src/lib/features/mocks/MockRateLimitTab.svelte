<script lang="ts">
  import type { MockEditorDraft } from '$lib/types/domain';
  import Select from '$lib/components/ui/Select.svelte';

  interface Props {
    editing: MockEditorDraft;
  }

  let { editing = $bindable() }: Props = $props();
</script>

<p class="text-xs text-muted">
  Throttle requests matching this endpoint, keyed by client IP. Set
  <span class="mono">Requests</span> to <span class="mono">0</span> to disable.
</p>
<div class="mt-3 grid gap-3 sm:grid-cols-3">
  <div class="field">
    <label class="label" for="rl-limit">Requests</label>
    <input id="rl-limit" type="number" min="0" class="input" bind:value={editing.rateLimit.limit} />
  </div>
  <div class="field">
    <label class="label" for="rl-window">Per window</label>
    <Select
      id="rl-window"
      bind:value={editing.rateLimit.windowSecs}
      options={[
        { value: 1, label: 'second' },
        { value: 60, label: 'minute' },
        { value: 3600, label: 'hour' }
      ]}
    />
  </div>
  <div class="field">
    <label class="label" for="rl-status">Reject status</label>
    <input id="rl-status" type="number" class="input" bind:value={editing.rateLimit.status} />
  </div>
</div>
{#if Number(editing.rateLimit.limit) > 0}
  <p class="mt-3 text-xs text-faint">
    Over the limit → <span class="mono">{editing.rateLimit.status}</span> with a
    <span class="mono">Retry-After</span> header. Counter resets every
    <span class="mono">{editing.rateLimit.windowSecs}s</span>.
  </p>
{:else}
  <p class="mt-3 text-xs text-faint">Disabled (Requests = 0).</p>
{/if}
