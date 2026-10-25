<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { createMutation, createQueries, createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import {
    apiErrorMessage,
    collectionsQueryOptions,
    collectionTreeQueryOptions,
    queryClient,
    queryKeys
  } from '$lib/api/query';
  import { uiState } from '$lib/state/session.svelte';
  import { sidebarState, closeMobileNav } from '$lib/state/sidebar.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import type { Collection, CollectionTree, Folder, Mock, Workspace } from '$lib/types/domain';
  import { methodTextColor } from '$lib/utils/http';
  import Icon from '$lib/components/ui/Icon.svelte';

  interface Props {
    currentWs?: string | null;
    currentWsObj?: Workspace | null;
    menuFor?: string | null;
  }

  let { currentWs = null, currentWsObj = null, menuFor = $bindable(null) }: Props = $props();

  let path = $derived(page.url.pathname);

  // Collections for the current workspace, shown nested in the sidebar.
  const collectionsQuery = createQuery(
    () => collectionsQueryOptions(currentWsObj?.id),
    () => queryClient
  );
  let collections = $derived<Collection[]>(collectionsQuery.data ?? []);
  let filteredCollections = $derived(
    collections.filter((c) => {
      const q = uiState.searchQuery.trim().toLowerCase();
      return !q || c.name.toLowerCase().includes(q) || c.slug.toLowerCase().includes(q);
    })
  );

  // Postman-style tree: lazily load a collection's folders + requests on expand.
  let expandedCols = $state<Set<string>>(new Set());
  let expandedFolders = $state<Set<string>>(new Set());
  let expandedList = $derived([...expandedCols]);
  const treeQueries = createQueries(
    () => ({
      queries: expandedList.map((cid) => collectionTreeQueryOptions(cid))
    }),
    () => queryClient
  );
  let tree = $derived<Record<string, CollectionTree>>(
    Object.fromEntries(
      expandedList
        .map((cid, i) => [cid, treeQueries[i]?.data] as const)
        .filter((entry): entry is readonly [string, CollectionTree] => Boolean(entry[1]))
    )
  );

  const childFolders = (cid: string, parentId: string | null): Folder[] =>
    (tree[cid]?.folders ?? []).filter((f) => (f.parentId ?? null) === parentId);
  const mocksIn = (cid: string, folderId: string | null): Mock[] =>
    (tree[cid]?.mocks ?? []).filter((m) => (m.folderId ?? null) === folderId);

  interface DraggedMock {
    id: string;
    collectionId: string;
    folderId: string | null;
  }

  let draggingMock = $state<DraggedMock | null>(null);
  let dropTarget = $state<string | null>(null);
  const dragTarget = (c: Collection, folderId: string | null): string =>
    folderId ? `folder:${folderId}` : `collection:${c.id}`;
  const canDrop = (c: Collection, folderId: string | null): boolean =>
    Boolean(
      draggingMock &&
      draggingMock.collectionId === c.id &&
      (draggingMock.folderId ?? null) !== (folderId ?? null)
    );

  function startMockDrag(e: DragEvent, c: Collection, m: Mock) {
    draggingMock = {
      id: m.id,
      collectionId: c.id,
      folderId: m.folderId ?? null
    };
    dropTarget = null;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', m.id);
    }
  }
  function endMockDrag() {
    draggingMock = null;
    dropTarget = null;
  }
  function allowDrop(e: DragEvent, c: Collection, folderId: string | null) {
    if (!canDrop(c, folderId)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dropTarget = dragTarget(c, folderId);
  }
  function leaveDrop(e: DragEvent, key: string) {
    if (dropTarget !== key) return;
    if (
      e.relatedTarget instanceof Node &&
      e.currentTarget instanceof Node &&
      e.currentTarget.contains(e.relatedTarget)
    )
      return;
    dropTarget = null;
  }
  const moveMockMutation = createMutation(
    () => ({
      mutationFn: ({
        mockId,
        folderId
      }: {
        collectionId: string;
        mockId: string;
        folderId: string | null;
      }) => api.patch(`/mocks/${mockId}`, { folderId }),
      onMutate: async ({ collectionId, mockId, folderId }) => {
        const key = queryKeys.collectionTree(collectionId);
        await queryClient.cancelQueries({ queryKey: key });
        const previous = queryClient.getQueryData<CollectionTree>(key);
        if (previous?.mocks) {
          queryClient.setQueryData<CollectionTree>(key, {
            ...previous,
            mocks: previous.mocks.map((m) => (m.id === mockId ? { ...m, folderId } : m))
          });
        }
        return { previous, key };
      },
      onError: (err, _vars, ctx) => {
        if (ctx?.previous && ctx.key) queryClient.setQueryData(ctx.key, ctx.previous);
        toast(apiErrorMessage(err, 'Could not move request'), 'error');
      },
      onSettled: (_data, _err, { collectionId, mockId }) => {
        queryClient.invalidateQueries({
          queryKey: queryKeys.collectionTree(collectionId)
        });
        queryClient.invalidateQueries({ queryKey: queryKeys.mock(mockId) });
      }
    }),
    () => queryClient
  );

  const createFolderMutation = createMutation(
    () => ({
      mutationFn: ({
        collectionId,
        name,
        parentId
      }: {
        collectionId: string;
        name: string;
        parentId: string | null;
      }) =>
        api.post<Folder>(`/collections/${collectionId}/folders`, {
          name,
          parentId
        }),
      onSuccess: (_data, { collectionId, parentId }) => {
        expandedCols = new Set(expandedCols).add(collectionId);
        if (parentId) expandedFolders = new Set(expandedFolders).add(parentId);
        queryClient.invalidateQueries({
          queryKey: queryKeys.collectionTree(collectionId)
        });
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not create folder'), 'error');
      }
    }),
    () => queryClient
  );

  const deleteFolderMutation = createMutation(
    () => ({
      mutationFn: ({ collectionId, folderId }: { collectionId: string; folderId: string }) =>
        api.del(`/collections/${collectionId}/folders/${folderId}`),
      onSuccess: (_data, { collectionId }) => {
        queryClient.invalidateQueries({
          queryKey: queryKeys.collectionTree(collectionId)
        });
        toast('Folder deleted');
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not delete folder'), 'error');
      }
    }),
    () => queryClient
  );

  function dropMock(e: DragEvent, c: Collection, folderId: string | null) {
    if (!canDrop(c, folderId) || !draggingMock) return;
    e.preventDefault();
    const moved = draggingMock;
    draggingMock = null;
    dropTarget = null;
    if (folderId) expandedFolders = new Set(expandedFolders).add(folderId);
    moveMockMutation.mutate({ collectionId: c.id, mockId: moved.id, folderId });
  }

  function toggleCol(c: Collection) {
    const s = new Set(expandedCols);
    if (s.has(c.id)) s.delete(c.id);
    else s.add(c.id);
    expandedCols = s;
  }
  function toggleFolder(fid: string) {
    const s = new Set(expandedFolders);
    s.has(fid) ? s.delete(fid) : s.add(fid);
    expandedFolders = s;
  }

  // Auto-expand a collection and its folders when an import completes.
  let pendingFolderExpand = $state<string | null>(null);
  $effect(() => {
    const req = uiState.expandTreeRequest;
    if (!req?.collectionId) return;
    if (!expandedCols.has(req.collectionId)) {
      expandedCols = new Set(expandedCols).add(req.collectionId);
    }
    pendingFolderExpand = req.collectionId;
  });
  $effect(() => {
    if (!pendingFolderExpand) return;
    const data = tree[pendingFolderExpand];
    if (data?.folders) {
      const next = new Set(expandedFolders);
      for (const f of data.folders) next.add(f.id);
      expandedFolders = next;
      pendingFolderExpand = null;
      uiState.expandTreeRequest = null;
    }
  });

  function newFolder(c: Collection, parentId: string | null = null) {
    menuFor = null;
    const name = prompt(parentId ? 'Sub-folder name' : 'Folder name');
    if (!name) return;
    createFolderMutation.mutate({ collectionId: c.id, name, parentId });
  }
  function deleteFolderNode(c: Collection, f: Folder) {
    menuFor = null;
    if (!confirm(`Delete folder "${f.name}"? Requests inside will move to the collection root.`))
      return;
    deleteFolderMutation.mutate({ collectionId: c.id, folderId: f.id });
  }
  function openCollectionImport(c: Collection) {
    menuFor = null;
    closeMobileNav();
    goto(`/${currentWs}/collections/${c.slug}?tab=import`);
  }
  function newRequest(c: Collection, folderId: string | null = null) {
    menuFor = null;
    closeMobileNav();
    const q = folderId ? `?folder=${folderId}` : '';
    goto(`/${currentWs}/collections/${c.slug}/new${q}`);
  }
  function openNewCollection() {
    closeMobileNav();
    uiState.newCollectionOpen = true;
  }
</script>

{#snippet folderNode(c: Collection, f: Folder)}
  <div>
    <div
      class="tree-row relative"
      class:drop-target={dropTarget === dragTarget(c, f.id)}
      role="treeitem"
      aria-selected="false"
      tabindex="-1"
      ondragover={(e) => allowDrop(e, c, f.id)}
      ondragleave={(e) => leaveDrop(e, dragTarget(c, f.id))}
      ondrop={(e) => dropMock(e, c, f.id)}
    >
      <button class="tree-caret" onclick={() => toggleFolder(f.id)} aria-label="Toggle folder">
        <Icon name="chevron-down" size={12} class={expandedFolders.has(f.id) ? '' : '-rotate-90'} />
      </button>
      <button
        type="button"
        class="tree-label flex items-center gap-1.5 truncate text-left"
        onclick={() => toggleFolder(f.id)}
      >
        <Icon name="folder" size={13} />
        {f.name}
      </button>
      <button
        class="tree-more"
        class:tree-more-open={menuFor === f.id}
        onclick={(e) => {
          e.stopPropagation();
          menuFor = menuFor === f.id ? null : f.id;
        }}
        aria-haspopup="menu"
        aria-expanded={menuFor === f.id}
        aria-label="Folder actions"
      >
        <Icon name="more" size={15} />
      </button>
      {#if menuFor === f.id}
        <div class="dropdown right-1 top-full mt-1 w-44" role="menu">
          <button class="menu-item w-full" onclick={() => newRequest(c, f.id)} role="menuitem">
            <Icon name="zap" size={15} /> New request
          </button>
          <button class="menu-item w-full" onclick={() => newFolder(c, f.id)} role="menuitem">
            <Icon name="folder" size={15} /> New sub-folder
          </button>
          <div class="my-1 border-t border-border"></div>
          <button
            class="menu-item w-full text-danger"
            onclick={() => deleteFolderNode(c, f)}
            role="menuitem"
          >
            <Icon name="trash" size={15} /> Delete folder
          </button>
        </div>
      {/if}
    </div>
    {#if expandedFolders.has(f.id)}
      <div class="ml-4 pl-1">
        {#each childFolders(c.id, f.id) as sub (sub.id)}
          {@render folderNode(c, sub)}
        {/each}
        {#each mocksIn(c.id, f.id) as m (m.id)}
          {@render requestRow(c, m)}
        {/each}
        {#if !childFolders(c.id, f.id).length && !mocksIn(c.id, f.id).length}
          <p class="px-2 py-1 text-[11px] text-faint">empty</p>
        {/if}
      </div>
    {/if}
  </div>
{/snippet}

{#snippet requestRow(c: Collection, m: Mock)}
  {@const reqActive = path === `/${currentWs}/collections/${c.slug}/${m.id}`}
  <a
    href={`/${currentWs}/collections/${c.slug}/${m.id}`}
    class="req-row"
    class:req-active={reqActive}
    draggable="true"
    title="Drag to a folder"
    onclick={closeMobileNav}
    ondragstart={(e) => startMockDrag(e, c, m)}
    ondragend={endMockDrag}
  >
    <span class="req-method {methodTextColor(m.method)}">{m.method}</span>
    <span class="truncate">{m.pathTemplate}</span>
  </a>
{/snippet}

<!-- brand -->
<a
  href={currentWs ? `/${currentWs}/overview` : '/'}
  class="flex h-14 shrink-0 items-center gap-1.5 border-b border-border px-4 font-semibold tracking-tight text-fg"
  onclick={closeMobileNav}
>
  <span class="text-accent"><Icon name="martini" size={20} /></span> mocktail
</a>

<!-- search -->
<div class="px-3 pt-3">
  <div class="relative">
    <span class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint">
      <Icon name="search" size={15} />
    </span>
    <input
      class="input py-1.5 pl-8 pr-10 text-sm"
      placeholder="Search collections"
      bind:value={uiState.searchQuery}
    />
    <kbd class="kbd absolute right-2 top-1/2 -translate-y-1/2">⌘K</kbd>
  </div>
</div>

<!-- nav -->
<nav class="flex min-h-0 flex-1 flex-col px-3 py-4">
  {#if currentWs}
    <div class="flex shrink-0 items-center gap-1 pr-1">
      <button
        class="group-toggle"
        onclick={() => (sidebarState.openCollections = !sidebarState.openCollections)}
        aria-expanded={sidebarState.openCollections}
      >
        <Icon
          name="chevron-down"
          size={13}
          class={sidebarState.openCollections ? '' : '-rotate-90'}
        />
        <Icon name="layers" size={14} />
        Collections
        <span class="ml-1 text-faint">{filteredCollections.length}</span>
      </button>
      <button
        class="ml-auto text-faint hover:text-fg"
        aria-label="New collection"
        onclick={openNewCollection}
      >
        <Icon name="plus" size={15} />
      </button>
    </div>
    {#if sidebarState.openCollections}
      <div class="mt-1.5 min-h-0 flex-1 overflow-y-auto overflow-x-hidden" role="tree">
        {#each filteredCollections as c (c.id)}
          {@const colActive =
            path === `/${currentWs}/collections/${c.slug}` ||
            path.startsWith(`/${currentWs}/collections/${c.slug}/logs/`) ||
            (path.startsWith(`/${currentWs}/collections/${c.slug}/`) && !expandedCols.has(c.id))}
          <div class="relative">
            <div
              class="tree-row"
              class:tree-active={colActive}
              class:drop-target={dropTarget === dragTarget(c, null)}
              role="treeitem"
              aria-selected={colActive}
              tabindex="-1"
              ondragover={(e) => allowDrop(e, c, null)}
              ondragleave={(e) => leaveDrop(e, dragTarget(c, null))}
              ondrop={(e) => dropMock(e, c, null)}
            >
              <button
                class="tree-caret"
                onclick={() => toggleCol(c)}
                aria-label="Toggle"
                aria-expanded={expandedCols.has(c.id)}
              >
                <Icon
                  name="chevron-down"
                  size={13}
                  class={expandedCols.has(c.id) ? '' : '-rotate-90'}
                />
              </button>
              <a
                href={`/${currentWs}/collections/${c.slug}`}
                class="tree-label truncate"
                onclick={closeMobileNav}
              >
                {c.name}
              </a>
              <button
                class="tree-more"
                class:tree-more-open={menuFor === c.id}
                onclick={(e) => {
                  e.stopPropagation();
                  menuFor = menuFor === c.id ? null : c.id;
                }}
                aria-haspopup="menu"
                aria-expanded={menuFor === c.id}
                aria-label="Collection actions"
              >
                <Icon name="more" size={16} />
              </button>
              {#if menuFor === c.id}
                <div class="dropdown right-1 top-full mt-1 w-44" role="menu">
                  <button
                    class="menu-item w-full"
                    onclick={() => newRequest(c, null)}
                    role="menuitem"
                  >
                    <Icon name="zap" size={15} /> New request
                  </button>
                  <button
                    class="menu-item w-full"
                    onclick={() => newFolder(c, null)}
                    role="menuitem"
                  >
                    <Icon name="folder" size={15} /> New folder
                  </button>
                  <button
                    class="menu-item w-full"
                    onclick={() => openCollectionImport(c)}
                    role="menuitem"
                  >
                    <Icon name="upload" size={15} /> Import OpenAPI
                  </button>
                </div>
              {/if}
            </div>

            {#if expandedCols.has(c.id)}
              <div class="ml-4 pl-1">
                {#if tree[c.id]}
                  {#each childFolders(c.id, null) as f (f.id)}
                    {@render folderNode(c, f)}
                  {/each}
                  {#each mocksIn(c.id, null) as m (m.id)}
                    {@render requestRow(c, m)}
                  {/each}
                  {#if !tree[c.id].folders.length && !tree[c.id].mocks.length}
                    <p class="px-2 py-1 text-[11px] text-faint">No requests yet</p>
                  {/if}
                {:else}
                  <p class="px-2 py-1 text-[11px] text-faint">loading…</p>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
        {#if !filteredCollections.length}
          <p class="px-3 py-1 text-xs text-faint">
            {uiState.searchQuery.trim() ? 'No matches' : 'No collections yet'}
          </p>
        {/if}
      </div>
    {/if}
  {/if}
</nav>
