<script lang="ts">
  import { goto } from '$app/navigation';
  import { createQuery } from '@tanstack/svelte-query';
  import { logsQueryOptions, queryClient } from '$lib/api/query';
  import { toast } from '$lib/state/toast.svelte';
  import type { Collection, RequestLog } from '$lib/types/domain';
  import { methodClass, statusTone, logUrl, toCurl, toFetch } from '$lib/utils/http';
  import { fmtTime } from '$lib/utils/format';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select from '$lib/components/ui/Select.svelte';

  interface Props {
    wsSlug: string;
    wsId: string | null;
    collSlug: string;
    coll: Collection;
  }

  let { wsSlug, wsId, collSlug, coll }: Props = $props();

  const LOG_PAGE_SIZE = 20;
  let logPath = $state('');
  let logStatus = $state('');
  let logPathApplied = $state('');
  let logStatusApplied = $state('');
  let logCursor = $state<string | null>(null);
  let logPrevCursors = $state<Array<string | null>>([]);
  let logMenuFor = $state<string | null>(null);

  // Reset pagination when the collection changes.
  $effect(() => {
    coll?.id;
    logCursor = null;
    logPrevCursors = [];
  });

  const logsQuery = createQuery(
    () =>
      logsQueryOptions(wsId, {
        collectionId: coll?.id ?? null,
        path: logPathApplied || undefined,
        status: logStatusApplied || undefined,
        cursor: logCursor,
        limit: LOG_PAGE_SIZE
      }),
    () => queryClient
  );

  let logs = $derived<RequestLog[]>(logsQuery.data?.items ?? []);
  let logNextCursor = $derived<string | null>(logsQuery.data?.nextCursor ?? null);
  let logLoading = $derived(logsQuery.isFetching);

  function searchLogs() {
    logPathApplied = logPath.trim();
    logStatusApplied = logStatus;
    logPrevCursors = [];
    logCursor = null;
    logsQuery.refetch();
  }

  function nextLogPage() {
    if (!logNextCursor || logLoading) return;
    logPrevCursors = [...logPrevCursors, logCursor];
    logCursor = logNextCursor;
  }

  function prevLogPage() {
    if (!logPrevCursors.length || logLoading) return;
    const prev = logPrevCursors[logPrevCursors.length - 1];
    logPrevCursors = logPrevCursors.slice(0, -1);
    logCursor = prev;
  }

  function copyFromMenu(e: MouseEvent, text: string, msg: string) {
    e.stopPropagation();
    navigator.clipboard?.writeText(text);
    toast(msg);
    logMenuFor = null;
  }
</script>

<svelte:window onclick={() => (logMenuFor = null)} />

<div class="mt-4 space-y-3">
  <div class="flex flex-wrap gap-2">
    <div class="relative flex-1 sm:min-w-[16rem]">
      <span class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-faint">
        <Icon name="search" size={15} />
      </span>
      <input
        class="input mono pl-9"
        placeholder="Filter by path, e.g. /todos"
        bind:value={logPath}
      />
    </div>
    <div class="sm:max-w-[10rem] sm:min-w-[10rem]">
      <Select
        bind:value={logStatus}
        options={[
          { value: '', label: 'Any status' },
          { value: '2xx', label: '2xx success' },
          { value: '3xx', label: '3xx redirect' },
          { value: '4xx', label: '4xx client error' },
          { value: '5xx', label: '5xx server error' }
        ]}
      />
    </div>
    <button class="btn btn-secondary" onclick={searchLogs} disabled={logLoading}>
      <Icon name="search" size={15} /> Search
    </button>
  </div>

  <div class="card p-0">
    <table class="tbl">
      <thead>
        <tr>
          <th class="w-48">Received</th>
          <th class="w-20">Method</th>
          <th>Path</th>
          <th class="w-20">Status</th>
          <th>Matched</th>
        </tr>
      </thead>
      <tbody>
        {#each logs as l (l.id ?? l.requestId)}
          <tr
            class="cursor-pointer"
            onclick={() => goto(`/${wsSlug}/collections/${collSlug}/logs/${l.requestId}`)}
          >
            <td class="text-xs text-muted" title={new Date(l.receivedAt).toISOString()}
              >{fmtTime(l.receivedAt)}</td
            >
            <td><span class={methodClass(l.method)}>{l.method}</span></td>
            <td class="mono text-xs">{l.path}</td>
            <td><span class="mono font-semibold {statusTone(l.status)}">{l.status}</span></td>
            <td>
              <div class="flex items-center justify-between gap-2">
                <span class="mono text-xs text-muted">{l.matchReason ?? 'no match'}</span>
                <div class="relative">
                  <button
                    class="log-more"
                    class:log-more-open={logMenuFor === l.requestId}
                    aria-haspopup="menu"
                    aria-expanded={logMenuFor === l.requestId}
                    aria-label="Log actions"
                    onclick={(e) => {
                      e.stopPropagation();
                      logMenuFor = logMenuFor === l.requestId ? null : l.requestId;
                    }}
                  >
                    <Icon name="more" size={16} />
                  </button>
                  {#if logMenuFor === l.requestId}
                    <div class="dropdown right-0 top-full mt-1 w-44" role="menu">
                      <button
                        class="menu-item w-full"
                        onclick={(e) => copyFromMenu(e, toCurl(wsSlug, l), 'Copied as cURL')}
                        role="menuitem"
                      >
                        <Icon name="copy" size={15} /> Copy as cURL
                      </button>
                      <button
                        class="menu-item w-full"
                        onclick={(e) => copyFromMenu(e, toFetch(wsSlug, l), 'Copied as fetch')}
                        role="menuitem"
                      >
                        <Icon name="copy" size={15} /> Copy as fetch
                      </button>
                      <button
                        class="menu-item w-full"
                        onclick={(e) => copyFromMenu(e, logUrl(wsSlug, l), 'URL copied')}
                        role="menuitem"
                      >
                        <Icon name="copy" size={15} /> Copy URL
                      </button>
                      <button
                        class="menu-item w-full"
                        onclick={(e) =>
                          copyFromMenu(e, JSON.stringify(l, null, 2), 'Log JSON copied')}
                        role="menuitem"
                      >
                        <Icon name="copy" size={15} /> Copy JSON
                      </button>
                      {#if l.matchedMockId}
                        <a
                          class="menu-item w-full"
                          href={`/${wsSlug}/collections/${collSlug}/${l.matchedMockId}`}
                          onclick={(e) => e.stopPropagation()}
                          role="menuitem"
                        >
                          <Icon name="external-link" size={15} /> Open matched mock
                        </a>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>
            </td>
          </tr>
        {/each}
        {#if !logs.length}
          <tr>
            <td colspan="5">
              <div class="flex flex-col items-center gap-2 py-12 text-center">
                <span class="text-faint"><Icon name="inbox" size={28} /></span>
                <p class="text-sm text-muted">
                  {logPathApplied || logStatusApplied
                    ? 'No logs match the filter.'
                    : 'No logs. Make a request to your mock URL.'}
                </p>
              </div>
            </td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  {#if logs.length || logPrevCursors.length}
    <div class="flex flex-wrap items-center justify-between gap-3 text-sm">
      <span class="text-muted">
        Page {logPrevCursors.length + 1}
        {#if logs.length}
          · Showing {logPrevCursors.length * LOG_PAGE_SIZE + 1}–{logPrevCursors.length *
            LOG_PAGE_SIZE +
            logs.length}
        {/if}
        · newest first
      </span>
      <div class="flex items-center gap-1">
        <button
          class="btn btn-ghost btn-sm"
          disabled={!logPrevCursors.length || logLoading}
          onclick={prevLogPage}
          aria-label="Previous page"
        >
          <Icon name="chevron-down" size={15} class="rotate-90" /> Previous
        </button>
        <button
          class="btn btn-ghost btn-sm"
          disabled={!logNextCursor || logLoading}
          onclick={nextLogPage}
          aria-label="Next page"
        >
          Next <Icon name="chevron-down" size={15} class="-rotate-90" />
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .log-more {
    display: flex;
    flex-shrink: 0;
    padding: 0.25rem;
    border-radius: 0.375rem;
    color: var(--muted);
    background: var(--surface-2);
    cursor: pointer;
    transition: all 150ms;
  }
  .log-more:hover,
  .log-more-open {
    color: var(--fg);
    background: var(--surface-hover);
  }
</style>
