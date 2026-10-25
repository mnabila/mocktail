<script lang="ts">
  import { page } from '$app/state';
  import { createMutation, createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import {
    apiErrorMessage,
    collectionsQueryOptions,
    queryClient,
    queryKeys,
    workspacesQueryOptions
  } from '$lib/api/query';
  import { uiState, wsIdFromSlug } from '$lib/state/session.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import { mountUrl } from '$lib/utils/format';
  import type { Collection } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  let wsSlug = $derived(page.params.ws ?? '');
  const workspacesQuery = createQuery(
    () => workspacesQueryOptions(),
    () => queryClient
  );
  let wsId = $derived(wsIdFromSlug(workspacesQuery.data ?? [], wsSlug));

  const collectionsQuery = createQuery(
    () => collectionsQueryOptions(wsId),
    () => queryClient
  );

  let items = $derived<Collection[]>(collectionsQuery.data ?? []);
  let loading = $derived(workspacesQuery.isPending || Boolean(wsId && collectionsQuery.isPending));
  let error = $derived(
    workspacesQuery.error
      ? apiErrorMessage(workspacesQuery.error, 'failed')
      : !workspacesQuery.isPending && !wsId
        ? 'workspace not found'
        : collectionsQuery.error
          ? apiErrorMessage(collectionsQuery.error, 'failed')
          : ''
  );

  let selected = $state<Set<string>>(new Set());
  const pageSize = 10;
  let cursor = $state<string | null>(null);
  let prevCursors = $state<Array<string | null>>([]);

  let filtered = $derived(
    items.filter((c) => {
      const q = uiState.searchQuery.trim().toLowerCase();
      return !q || c.name.toLowerCase().includes(q) || c.slug.toLowerCase().includes(q);
    })
  );
  let startIdx = $derived(cursor ? filtered.findIndex((c) => c.id === cursor) + 1 : 0);
  let pageItems = $derived(filtered.slice(startIdx, startIdx + pageSize));
  let hasPrev = $derived(prevCursors.length > 0);
  let hasNext = $derived(startIdx + pageItems.length < filtered.length);
  let stats = $derived({
    total: items.length,
    openapi: items.filter((c) => c.source === 'openapi').length,
    manual: items.filter((c) => c.source !== 'openapi').length
  });

  // Reset to the first page whenever the search filter changes.
  $effect(() => {
    uiState.searchQuery;
    cursor = null;
    prevCursors = [];
  });

  function nextPage() {
    if (!hasNext) return;
    prevCursors = [...prevCursors, cursor];
    cursor = pageItems[pageItems.length - 1].id;
  }
  function prevPage() {
    if (!hasPrev) return;
    cursor = prevCursors[prevCursors.length - 1];
    prevCursors = prevCursors.slice(0, -1);
  }

  function toggle(id: string) {
    const s = new Set(selected);
    s.has(id) ? s.delete(id) : s.add(id);
    selected = s;
  }
  let allOnPage = $derived(pageItems.length > 0 && pageItems.every((c) => selected.has(c.id)));
  function toggleAll() {
    const s = new Set(selected);
    if (allOnPage) pageItems.forEach((c) => s.delete(c.id));
    else pageItems.forEach((c) => s.add(c.id));
    selected = s;
  }

  const deleteSelectedMutation = createMutation(
    () => ({
      mutationFn: async (ids: string[]) => {
        await Promise.all(ids.map((id) => api.del(`/collections/${id}`)));
        return ids;
      },
      onSuccess: async (ids) => {
        selected = new Set();
        for (const id of ids) {
          queryClient.removeQueries({ queryKey: queryKeys.collectionTree(id) });
        }
        await queryClient.invalidateQueries({ queryKey: queryKeys.collections(wsId) });
        toast(`Deleted ${ids.length} collection${ids.length > 1 ? 's' : ''}`);
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not delete collections'), 'error');
      }
    }),
    () => queryClient
  );

  function deleteSelected() {
    if (!selected.size || deleteSelectedMutation.isPending) return;
    if (
      !confirm(
        `Delete ${selected.size} collection${selected.size > 1 ? 's' : ''}? Their mock URLs stop working.`
      )
    )
      return;
    deleteSelectedMutation.mutate([...selected]);
  }

  function copy(text: string) {
    navigator.clipboard?.writeText(text);
    toast('Mount URL copied');
  }
</script>

<!-- header -->
<div class="flex flex-wrap items-center justify-between gap-3">
  <div>
    <h1 class="text-2xl font-semibold tracking-tight">Collections</h1>
    <p class="mt-0.5 text-sm text-muted">Grouped mocks served under one mount path.</p>
  </div>
  <button class="btn btn-primary" onclick={() => (uiState.newCollectionOpen = true)}>
    <Icon name="plus" size={16} /> New collection
  </button>
</div>

<!-- stat cards -->
<div class="mt-5 grid gap-4 sm:grid-cols-3">
  {#each [{ label: 'Total collections', value: stats.total, icon: 'layers' }, { label: 'OpenAPI', value: stats.openapi, icon: 'download' }, { label: 'Manual', value: stats.manual, icon: 'pencil' }] as s (s.label)}
    <div class="card flex items-center gap-4 p-4">
      <span
        class="flex h-10 w-10 items-center justify-center rounded-lg bg-accent-soft text-accent"
      >
        <Icon name={s.icon} size={18} />
      </span>
      <div>
        <div class="text-xs text-muted">{s.label}</div>
        <div class="text-2xl font-semibold tracking-tight text-fg">{s.value}</div>
      </div>
    </div>
  {/each}
</div>

{#if error}
  <p class="mt-3 flex items-center gap-1.5 text-sm text-danger">
    <Icon name="alert" size={15} />
    {error}
  </p>
{/if}

<!-- table -->
<div class="card mt-5 overflow-hidden p-0">
  <table class="tbl">
    <thead>
      <tr>
        <th class="w-10 pl-4">
          <input
            type="checkbox"
            class="cbx"
            checked={allOnPage}
            onchange={toggleAll}
            aria-label="Select all"
          />
        </th>
        <th>Name</th>
        <th>Mount URL</th>
        <th class="w-28">Source</th>
      </tr>
    </thead>
    <tbody>
      {#each pageItems as c (c.id)}
        <tr class:row-selected={selected.has(c.id)}>
          <td class="pl-4">
            <input
              type="checkbox"
              class="cbx"
              checked={selected.has(c.id)}
              onchange={() => toggle(c.id)}
              aria-label="Select {c.name}"
            />
          </td>
          <td>
            <a class="link" href={`/${wsSlug}/collections/${c.slug}`}>{c.name}</a>
          </td>
          <td>
            <button
              class="group inline-flex items-center gap-1.5 text-muted transition-colors hover:text-fg"
              onclick={() => copy(mountUrl(wsSlug, c.slug, c.basePath))}
              title="Copy mount URL"
            >
              <span class="mono text-xs">{mountUrl(wsSlug, c.slug, c.basePath)}</span>
              <span class="opacity-0 transition-opacity group-hover:opacity-100"
                ><Icon name="copy" size={13} /></span
              >
            </button>
          </td>
          <td>
            <span class="pill" class:pill-openapi={c.source === 'openapi'}>{c.source}</span>
          </td>
        </tr>
      {/each}

      {#if loading}
        <tr><td colspan="4" class="py-8 text-center text-faint">loading…</td></tr>
      {:else if !filtered.length}
        <tr>
          <td colspan="4">
            <div class="flex flex-col items-center gap-3 py-12 text-center">
              <span class="text-faint"><Icon name="inbox" size={30} /></span>
              <div>
                <p class="font-medium text-fg">
                  {uiState.searchQuery ? 'No matches' : 'No collections yet'}
                </p>
                <p class="mt-0.5 text-sm text-muted">
                  {uiState.searchQuery
                    ? 'Try a different search.'
                    : 'Create one to start serving mocks.'}
                </p>
              </div>
              {#if !uiState.searchQuery}
                <button
                  class="btn btn-secondary btn-sm"
                  onclick={() => (uiState.newCollectionOpen = true)}
                >
                  <Icon name="plus" size={15} /> New collection
                </button>
              {/if}
            </div>
          </td>
        </tr>
      {/if}
    </tbody>
  </table>

  {#if filtered.length}
    <div class="flex items-center justify-between border-t border-border px-4 py-3 text-sm">
      <span class="text-muted">
        Showing {startIdx + 1}–{startIdx + pageItems.length}
      </span>
      {#if hasPrev || hasNext}
        <div class="flex items-center gap-1">
          <button
            class="btn btn-ghost btn-sm"
            disabled={!hasPrev}
            onclick={prevPage}
            aria-label="Previous"
          >
            <Icon name="chevron-down" size={15} class="rotate-90" /> Previous
          </button>
          <button
            class="btn btn-ghost btn-sm"
            disabled={!hasNext}
            onclick={nextPage}
            aria-label="Next"
          >
            Next <Icon name="chevron-down" size={15} class="-rotate-90" />
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if selected.size}
  <div class="bulkbar">
    <span
      class="flex h-6 items-center rounded-md bg-accent-soft px-2 text-xs font-semibold text-accent"
    >
      {selected.size} selected
    </span>
    <button class="btn btn-ghost btn-sm text-danger" onclick={deleteSelected}>
      <Icon name="trash" size={15} /> Delete
    </button>
    <button
      class="btn btn-ghost btn-sm"
      onclick={() => (selected = new Set())}
      aria-label="Clear selection"
    >
      <Icon name="plus" size={15} class="rotate-45" />
    </button>
  </div>
{/if}

<style>
  :global(.row-selected) {
    background: var(--accent-soft) !important;
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .bulkbar {
    position: fixed;
    bottom: 1.25rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 60;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 0.625rem;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    padding: 0.5rem 0.625rem;
    box-shadow: var(--shadow-md);
    animation: bar-in 160ms ease-out;
  }
  @keyframes bar-in {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bulkbar {
      animation: none;
    }
  }
</style>
