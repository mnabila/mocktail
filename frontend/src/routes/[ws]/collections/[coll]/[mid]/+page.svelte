<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { createMutation, createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import {
    apiErrorMessage,
    collectionsQueryOptions,
    mockQueryOptions,
    queryClient,
    queryKeys,
    workspacesQueryOptions
  } from '$lib/api/query';
  import { wsIdFromSlug } from '$lib/state/session.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import type {
    Collection,
    Mock,
    MockEditorDraft,
    MockHeader,
    RateLimitConfig
  } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MockRequestBar from '$lib/features/mocks/MockRequestBar.svelte';
  import MockResponseTab, { PRESET_TYPES } from '$lib/features/mocks/MockResponseTab.svelte';
  import MockHeadersTab from '$lib/features/mocks/MockHeadersTab.svelte';
  import MockSettingsTab from '$lib/features/mocks/MockSettingsTab.svelte';
  import MockRateLimitTab from '$lib/features/mocks/MockRateLimitTab.svelte';

  let wsSlug = $derived(page.params.ws ?? '');
  let collSlug = $derived(page.params.coll ?? '');
  let mid = $derived(page.params.mid ?? ''); // 'new' or a mock id
  let folderParam = $derived(page.url.searchParams.get('folder') || null);

  let mutationError = $state('');
  let editing = $state<MockEditorDraft | null>(null);

  const EDITOR_TABS = [
    { id: 'response', label: 'Response' },
    { id: 'headers', label: 'Headers' },
    { id: 'settings', label: 'Settings' },
    { id: 'ratelimit', label: 'Rate limit' }
  ] as const;
  let etab = $state<(typeof EDITOR_TABS)[number]['id']>('response');

  function blankRateLimit(): RateLimitConfig {
    return { limit: 0, windowSecs: 60, status: 429 };
  }
  function blankMock(folderId: string | null = null): MockEditorDraft {
    return {
      method: 'GET',
      pathTemplate: '/',
      folderId: folderId || null,
      response: {
        status: 200,
        headers: [],
        body: '',
        contentType: 'application/json',
        customType: false
      },
      priority: 0,
      delayMs: 0,
      fault: null,
      rateLimit: blankRateLimit()
    };
  }
  function normalizeHeaders(h: unknown): MockHeader[] {
    if (Array.isArray(h))
      return h.map((x) =>
        x && typeof x === 'object' && 'name' in x && x.name !== undefined
          ? { name: String(x.name), value: String((x as { value?: unknown }).value ?? '') }
          : {
              name: String((x as unknown[])?.[0] ?? ''),
              value: String((x as unknown[])?.[1] ?? '')
            }
      );
    if (h && typeof h === 'object')
      return Object.entries(h).map(([name, value]) => ({ name, value: String(value) }));
    return [];
  }
  function fromMock(m: Mock): MockEditorDraft {
    const ct = m.response.contentType ?? 'application/json';
    return {
      id: m.id,
      method: m.method,
      pathTemplate: m.pathTemplate,
      folderId: m.folderId ?? null,
      response: {
        status: m.response.status,
        headers: normalizeHeaders(m.response.headers),
        body: m.response.body ?? '',
        contentType: ct,
        customType: !PRESET_TYPES.includes(ct)
      },
      priority: m.priority,
      delayMs: m.delayMs,
      fault: m.fault,
      rateLimit:
        m.rateLimit && m.rateLimit.limit > 0
          ? { ...blankRateLimit(), ...m.rateLimit }
          : blankRateLimit()
    };
  }

  const workspacesQuery = createQuery(
    () => workspacesQueryOptions(),
    () => queryClient
  );
  let wsId = $derived(wsIdFromSlug(workspacesQuery.data ?? [], wsSlug));

  const collectionsQuery = createQuery(
    () => collectionsQueryOptions(wsId),
    () => queryClient
  );
  let coll = $derived<Collection | null>(
    (collectionsQuery.data ?? []).find((c) => c.slug === collSlug) ?? null
  );

  let isNewMock = $derived(mid === 'new');
  const mockQuery = createQuery(
    () => mockQueryOptions(mid, !isNewMock),
    () => queryClient
  );

  let loading = $derived(
    workspacesQuery.isPending ||
      Boolean(wsId && collectionsQuery.isPending) ||
      (!isNewMock && mockQuery.isPending)
  );

  let queryError = $derived.by(() => {
    const rawErr =
      workspacesQuery.error ?? collectionsQuery.error ?? (!isNewMock ? mockQuery.error : null);
    if (rawErr) {
      if ((rawErr as { status?: number })?.status === 401) return 'unauthorized';
      return apiErrorMessage(rawErr, 'failed');
    }
    if (!workspacesQuery.isPending && !collectionsQuery.isPending && !coll) {
      return 'collection not found';
    }
    return '';
  });

  let error = $derived(mutationError || queryError);

  $effect(() => {
    const targetMid = mid;
    const targetFolder = folderParam;
    const resolvedColl = coll;
    const fetchedMock = mockQuery.data;
    mutationError = '';
    if (!resolvedColl) {
      editing = null;
      return;
    }
    if (targetMid === 'new') {
      editing = blankMock(targetFolder);
    } else if (fetchedMock && fetchedMock.id === targetMid) {
      editing = fromMock(fetchedMock);
    } else {
      editing = null;
    }
  });

  function backToCollection() {
    goto(`/${wsSlug}/collections/${collSlug}`);
  }

  const saveMockMutation = createMutation(
    () => ({
      mutationFn: async ({
        draft,
        collectionId
      }: {
        draft: MockEditorDraft;
        collectionId: string;
      }) => {
        const rl = draft.rateLimit;
        const rlEnabled = Number(rl.limit) > 0;
        const body = {
          method: draft.method,
          pathTemplate: draft.pathTemplate,
          folderId: draft.folderId ?? null,
          response: {
            status: Number(draft.response.status),
            body: draft.response.body,
            contentType: draft.response.contentType || 'text/plain',
            headers: draft.response.headers
              .filter((h) => h.name.trim())
              .map((h) => ({ name: h.name.trim(), value: h.value }))
          },
          priority: Number(draft.priority),
          delayMs: Number(draft.delayMs),
          fault: draft.fault || null,
          rateLimit: rlEnabled
            ? {
                limit: Number(rl.limit),
                windowSecs: Number(rl.windowSecs),
                status: Number(rl.status)
              }
            : null
        };
        if (draft.id) {
          return api.put<Mock>(`/mocks/${draft.id}`, body);
        }
        return api.post<Mock>(`/collections/${collectionId}/mocks`, body);
      },
      onSuccess: async (_saved, { draft, collectionId }) => {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: queryKeys.collectionTree(collectionId) }),
          draft.id
            ? queryClient.invalidateQueries({ queryKey: queryKeys.mock(draft.id) })
            : Promise.resolve()
        ]);
        toast('Mock saved');
        backToCollection();
      },
      onError: (err) => {
        mutationError = apiErrorMessage(err, 'failed');
        toast(mutationError, 'error');
      }
    }),
    () => queryClient
  );

  function saveMock() {
    if (!editing || !coll || saveMockMutation.isPending) return;
    mutationError = '';
    saveMockMutation.mutate({ draft: editing, collectionId: coll.id });
  }
</script>

{#if error && !editing}
  <p class="flex items-center gap-1.5 text-sm text-danger">
    <Icon name="alert" size={15} />
    {error}
  </p>
{/if}

{#if loading && !editing}
  <p class="text-sm text-muted">Loading…</p>
{/if}

{#if editing}
  <!-- breadcrumb + page actions -->
  <div class="flex flex-wrap items-center justify-between gap-3">
    <nav class="flex items-center gap-1.5 text-sm text-muted">
      <a class="hover:text-fg" href={`/${wsSlug}/collections`}>Collections</a>
      <Icon name="chevron-down" size={13} class="-rotate-90 text-faint" />
      <a class="hover:text-fg" href={`/${wsSlug}/collections/${collSlug}`}
        >{coll?.name ?? collSlug}</a
      >
      <Icon name="chevron-down" size={13} class="-rotate-90 text-faint" />
      <span class="text-fg">{editing.id ? 'Edit mock' : 'New mock'}</span>
    </nav>
    <div class="flex gap-2">
      <button class="btn btn-ghost btn-sm" onclick={backToCollection}>Cancel</button>
      <button class="btn btn-primary btn-sm" onclick={saveMock}
        ><Icon name="check" size={15} /> Save mock</button
      >
    </div>
  </div>

  <MockRequestBar bind:editing onSave={saveMock} />

  <div class="mt-4 flex gap-1 border-b border-border" role="tablist">
    {#each EDITOR_TABS as t (t.id)}
      <button
        class="tab"
        class:tab-active={etab === t.id}
        role="tab"
        aria-selected={etab === t.id}
        onclick={() => (etab = t.id)}
      >
        {t.label}
        {#if t.id === 'headers' && editing.response.headers.length}
          <span class="tab-count">{editing.response.headers.length}</span>
        {/if}
        {#if t.id === 'ratelimit' && Number(editing.rateLimit.limit) > 0}
          <span class="tab-dot"></span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="mt-4 min-h-[22rem]">
    {#if etab === 'response'}
      <MockResponseTab bind:editing />
    {:else if etab === 'headers'}
      <MockHeadersTab bind:editing />
    {:else if etab === 'settings'}
      <MockSettingsTab bind:editing />
    {:else if etab === 'ratelimit'}
      <MockRateLimitTab bind:editing />
    {/if}
  </div>
{/if}

<style>
  .tab-count {
    margin-left: 0.375rem;
    border-radius: 999px;
    background: var(--surface-3, color-mix(in srgb, var(--fg) 12%, transparent));
    padding: 0 0.375rem;
    font-size: 0.625rem;
    font-weight: 600;
    color: var(--muted);
  }
  .tab-dot {
    display: inline-block;
    margin-left: 0.375rem;
    height: 0.4rem;
    width: 0.4rem;
    border-radius: 999px;
    background: var(--accent);
    vertical-align: middle;
  }
</style>
