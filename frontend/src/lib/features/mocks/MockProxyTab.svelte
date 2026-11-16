<script lang="ts">
  import type { MockEditorDraft } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  interface Props {
    editing: MockEditorDraft;
  }

  let { editing = $bindable() }: Props = $props();
</script>

<div class="field">
  <label class="label" for="e-forward-url">Forward to external API</label>
  <input
    id="e-forward-url"
    class="input mono"
    placeholder="https://api.upstream.com"
    bind:value={editing.response.forward.baseUrl}
  />
  <p class="text-xs text-faint">
    Enter an upstream base URL to proxy matching requests and return its response as-is; leave empty
    to disable. The request sub-path and query string are appended to this base URL.
    Private/loopback addresses are rejected on save.
  </p>
</div>

<div
  class="mt-3 flex gap-2.5 rounded-lg border border-[var(--accent)] bg-[var(--accent-soft)] px-3 py-2.5 text-sm text-fg"
  role="alert"
>
  <span class="mt-0.5 shrink-0 text-[var(--accent)]"><Icon name="alert" size={16} /></span>
  <div>
    <p class="font-medium">When a base URL is set, this mock proxies to an external API.</p>
    <p class="mt-0.5 text-xs text-muted">
      The <strong>Response</strong>, <strong>Headers</strong> and <strong>Fault</strong> settings are
      ignored while forwarding; the upstream's status, body and headers are returned instead. Only the
      <strong>Delay</strong> still applies.
    </p>
  </div>
</div>
