<script lang="ts">
  import { page } from '$app/state';
  import { createQuery } from '@tanstack/svelte-query';
  import {
    apiErrorMessage,
    logDetailQueryOptions,
    queryClient,
    workspacesQueryOptions
  } from '$lib/api/query';
  import { wsIdFromSlug } from '$lib/state/session.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import { methodClass, statusTone, logUrl, toCurl, toFetch } from '$lib/utils/http';
  import { fmtTime } from '$lib/utils/format';
  import type { RequestLog } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';
  import JsonBlock from '$lib/components/ui/JsonBlock.svelte';

  let wsSlug = $derived(page.params.ws ?? '');
  let collSlug = $derived(page.params.coll ?? '');
  let rid = $derived(page.params.rid ?? '');

  const workspacesQuery = createQuery(
    () => workspacesQueryOptions(),
    () => queryClient
  );
  let wsId = $derived(wsIdFromSlug(workspacesQuery.data ?? [], wsSlug));

  const logDetailQuery = createQuery(
    () => logDetailQueryOptions(wsId, rid),
    () => queryClient
  );

  let log = $derived<RequestLog | null>(logDetailQuery.data ?? null);
  let loading = $derived(workspacesQuery.isPending || Boolean(wsId && logDetailQuery.isPending));
  let error = $derived.by(() => {
    const rawErr = workspacesQuery.error ?? logDetailQuery.error;
    if (rawErr) {
      if ((rawErr as { status?: number })?.status === 401) return 'unauthorized';
      return apiErrorMessage(rawErr, 'failed');
    }
    if (!loading && !log) return 'log not found';
    return '';
  });

  function copyText(text: string, msg = 'Copied') {
    navigator.clipboard?.writeText(text);
    toast(msg);
  }
</script>

<a
  href={`/${wsSlug}/collections/${collSlug}?tab=logs`}
  class="inline-flex items-center gap-1 text-sm text-muted hover:text-fg"
>
  <Icon name="chevron-down" size={15} class="rotate-90" /> Back to logs
</a>

{#if loading}
  <p class="mt-6 text-muted">loading…</p>
{:else if error}
  <p class="mt-6 flex items-center gap-1.5 text-sm text-danger">
    <Icon name="alert" size={15} />
    {error}
  </p>
{:else if log}
  <!-- header -->
  <div class="mt-2 flex flex-wrap items-center gap-3">
    <span class={methodClass(log.method)}>{log.method}</span>
    <span class="mono text-lg font-semibold {statusTone(log.status)}">{log.status}</span>
    <h1 class="mono min-w-0 flex-1 truncate text-lg font-medium text-fg">{log.path}</h1>
  </div>

  <div class="mt-2 flex flex-wrap items-center gap-x-5 gap-y-1 text-sm text-muted">
    <span class="flex items-center gap-1"
      ><Icon name="clock" size={14} /> {fmtTime(log.receivedAt, { relative: true })}</span
    >
    <button
      class="mono flex items-center gap-1 hover:text-fg"
      title="Copy request ID"
      onclick={() => copyText(log.requestId, 'Request ID copied')}
    >
      <Icon name="hash" size={14} />
      {log.requestId}
    </button>
    {#if log.matchReason}
      <span class="mono text-accent">{log.matchReason}</span>
    {:else}
      <span class="mono text-danger">no match (fell through)</span>
    {/if}
  </div>

  <!-- actions -->
  <div class="mt-4 flex flex-wrap gap-2">
    <button
      class="btn btn-secondary btn-sm"
      onclick={() => copyText(toCurl(wsSlug, log), 'Copied as cURL')}
    >
      <Icon name="copy" size={14} /> Copy as cURL
    </button>
    <button
      class="btn btn-secondary btn-sm"
      onclick={() => copyText(toFetch(wsSlug, log), 'Copied as fetch')}
    >
      <Icon name="copy" size={14} /> Copy as fetch
    </button>
    <button
      class="btn btn-secondary btn-sm"
      onclick={() => copyText(logUrl(wsSlug, log), 'URL copied')}
    >
      <Icon name="copy" size={14} /> Copy URL
    </button>
    <button
      class="btn btn-secondary btn-sm"
      onclick={() => copyText(JSON.stringify(log, null, 2), 'Log JSON copied')}
    >
      <Icon name="copy" size={14} /> Copy JSON
    </button>
    {#if log.matchedMockId}
      <a
        class="btn btn-secondary btn-sm"
        href={`/${wsSlug}/collections/${collSlug}/${log.matchedMockId}`}
      >
        <Icon name="external-link" size={14} /> Open matched mock
      </a>
    {/if}
  </div>

  <!-- two-column detail -->
  <div class="mt-6 grid gap-6 lg:grid-cols-2">
    <section class="card space-y-4">
      <h2 class="text-xs font-semibold uppercase tracking-wide text-faint">Request</h2>
      <JsonBlock title="HTTP method" value={log.method} />
      <div class="field">
        <span class="label">HTTP URL</span>
        <button
          class="group flex items-start gap-1.5 rounded-md border border-border bg-surface-2 px-2.5 py-1.5 text-left"
          onclick={() => copyText(logUrl(wsSlug, log), 'URL copied')}
        >
          <span class="mono break-all text-xs text-fg">{logUrl(wsSlug, log)}</span>
          <span class="ml-auto shrink-0 text-faint group-hover:text-fg"
            ><Icon name="copy" size={13} /></span
          >
        </button>
      </div>
      <JsonBlock title="HTTP headers" value={log.requestHeaders} empty="— (not captured)" />
      <JsonBlock title="HTTP request body" value={log.requestBody} empty="— (no body)" />
    </section>

    <section class="card space-y-4">
      <h2 class="text-xs font-semibold uppercase tracking-wide text-faint">Response</h2>
      <JsonBlock title="HTTP response body" value={log.response?.body} empty="— (no body)" />
    </section>
  </div>
{/if}
