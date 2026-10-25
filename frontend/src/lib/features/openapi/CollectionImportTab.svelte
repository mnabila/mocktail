<script lang="ts">
  import { createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import { apiErrorMessage, queryClient, queryKeys } from '$lib/api/query';
  import { uiState } from '$lib/state/session.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import type {
    Collection,
    ImportCommitResponse,
    ImportPreviewItem,
    ImportPreviewResponse
  } from '$lib/types/domain';
  import { methodClass, statusTone } from '$lib/utils/http';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { SAMPLE_SWAGGER_SPEC } from './sampleSpec';

  interface Props {
    wsId: string | null;
    coll: Collection;
    onImported?: () => void | Promise<void>;
  }

  let { wsId, coll, onImported = undefined }: Props = $props();

  let specText = $state('');
  let specFileName = $state('');
  let draggingFile = $state(false);
  let createFolders = $state(true);
  let deleteRemoved = $state(false);
  let preview = $state<ImportPreviewResponse | null>(null);
  let selectedKeys = $state<Set<string>>(new Set());
  let collapsedPreviewFolders = $state<Set<string>>(new Set());
  let fileInputEl = $state<HTMLInputElement | null>(null);

  let importableItems = $derived<ImportPreviewItem[]>(
    (preview?.items ?? []).filter((it) => it.action !== 'removed')
  );
  let removedItems = $derived<ImportPreviewItem[]>(
    (preview?.items ?? []).filter((it) => it.action === 'removed')
  );

  interface PreviewFolderGroup {
    key: string;
    tag: string;
    segments: string[];
    isNewFolder: boolean;
    items: ImportPreviewItem[];
  }

  // Group importable items by tag folder path (or '' for collection root).
  let groupedPreview = $derived.by<PreviewFolderGroup[]>(() => {
    if (!preview) return [];
    const folderMeta = new Map((preview.folders ?? []).map((f) => [f.path.toLowerCase(), f]));
    const groups = new Map<string, PreviewFolderGroup>();
    for (const it of importableItems) {
      const folderKey = createFolders && it.tag ? it.tag : '';
      if (!groups.has(folderKey)) {
        const meta = folderKey ? folderMeta.get(folderKey.toLowerCase()) : null;
        groups.set(folderKey, {
          key: folderKey || '__root__',
          tag: folderKey,
          segments: folderKey ? folderKey.split('/').filter(Boolean) : [],
          isNewFolder: meta ? meta.action === 'added' : Boolean(folderKey),
          items: []
        });
      }
      groups.get(folderKey)!.items.push(it);
    }
    return [...groups.values()].sort((a, b) => {
      if (!a.tag && b.tag) return 1;
      if (a.tag && !b.tag) return -1;
      return a.tag.localeCompare(b.tag);
    });
  });

  let selectedFolderCount = $derived.by(() => {
    if (!createFolders || !preview) return 0;
    const foldersWithSelected = new Set<string>();
    for (const it of importableItems) {
      if (selectedKeys.has(it.key) && it.tag) {
        const segs = it.tag
          .split('/')
          .map((s) => s.trim())
          .filter(Boolean);
        let cur = '';
        for (const seg of segs) {
          cur = cur ? `${cur}/${seg}` : seg;
          foldersWithSelected.add(cur.toLowerCase());
        }
      }
    }
    return foldersWithSelected.size;
  });

  async function handleFileSelect(file?: File) {
    if (!file) return;
    specFileName = file.name;
    specText = await file.text();
    await doPreview();
  }

  function onFileDrop(e: DragEvent) {
    e.preventDefault();
    draggingFile = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) handleFileSelect(file);
  }

  async function loadSampleSpec() {
    specFileName = 'petstore.openapi.json';
    specText = SAMPLE_SWAGGER_SPEC;
    await doPreview();
  }

  function clearSpec() {
    specText = '';
    specFileName = '';
    preview = null;
    selectedKeys = new Set();
  }

  function toggleItemKey(key: string) {
    const next = new Set(selectedKeys);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    selectedKeys = next;
  }

  function toggleFolderGroup(group: PreviewFolderGroup) {
    const next = new Set(selectedKeys);
    const allSelected = group.items.every((it) => next.has(it.key));
    for (const it of group.items) {
      if (allSelected) next.delete(it.key);
      else next.add(it.key);
    }
    selectedKeys = next;
  }

  function togglePreviewFolderCollapse(groupKey: string) {
    const next = new Set(collapsedPreviewFolders);
    if (next.has(groupKey)) next.delete(groupKey);
    else next.add(groupKey);
    collapsedPreviewFolders = next;
  }

  function selectByPreset(mode: 'all' | 'none' | '2xx') {
    if (!preview) return;
    if (mode === 'all') {
      selectedKeys = new Set(importableItems.map((it) => it.key));
    } else if (mode === 'none') {
      selectedKeys = new Set();
    } else if (mode === '2xx') {
      selectedKeys = new Set(
        importableItems.filter((it) => it.status >= 200 && it.status < 300).map((it) => it.key)
      );
    }
  }

  const previewMutation = createMutation(
    () => ({
      mutationFn: (rawSpec: string) =>
        api.postRaw<ImportPreviewResponse>(`/collections/${coll.id}/import/preview`, rawSpec),
      onSuccess: (res) => {
        preview = res;
        selectedKeys = new Set(
          (res.items ?? [])
            .filter((it) => it.selected && it.action !== 'removed')
            .map((it) => it.key)
        );
        collapsedPreviewFolders = new Set();
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Failed to parse OpenAPI/Swagger specification'), 'error');
      }
    }),
    () => queryClient
  );

  const importMutation = createMutation(
    () => ({
      mutationFn: () =>
        api.post<ImportCommitResponse>(`/collections/${coll.id}/import`, {
          spec: specText,
          accept: [...selectedKeys],
          deleteRemoved,
          createFolders
        }),
      onSuccess: async (res) => {
        preview = null;
        specText = '';
        specFileName = '';
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: queryKeys.collectionTree(coll.id) }),
          queryClient.invalidateQueries({ queryKey: queryKeys.collections(wsId) }),
          queryClient.invalidateQueries({ queryKey: ['mock'] })
        ]);
        uiState.expandTreeRequest = { collectionId: coll.id, ts: Date.now() };
        const folderMsg = res.foldersCreated
          ? `, ${res.foldersCreated} folder${res.foldersCreated > 1 ? 's' : ''} created`
          : '';
        const delMsg = res.deleted ? `, ${res.deleted} removed` : '';
        toast(`Imported: ${res.created} created, ${res.updated} updated${folderMsg}${delMsg}`);
        await onImported?.();
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Import failed'), 'error');
      }
    }),
    () => queryClient
  );

  let previewing = $derived(previewMutation.isPending);
  let importing = $derived(importMutation.isPending);

  function doPreview() {
    if (!specText.trim()) return;
    previewMutation.mutate(specText);
  }

  function doImport() {
    if (!specText.trim() || (!selectedKeys.size && !deleteRemoved)) return;
    importMutation.mutate();
  }
</script>

<div class="mt-4 space-y-4">
  <div class="card space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <div>
        <h2 class="text-sm font-semibold text-fg">Import OpenAPI / Swagger Specification</h2>
        <p class="mt-0.5 text-xs text-muted">
          Drop a JSON or YAML specification (OpenAPI 3.x or Swagger 2.0), or paste the raw document
          below.
        </p>
      </div>
      <div class="flex items-center gap-2">
        <button type="button" class="btn btn-secondary btn-sm" onclick={loadSampleSpec}>
          <Icon name="zap" size={14} /> Load sample with tags
        </button>
        {#if specText}
          <button type="button" class="btn btn-ghost btn-sm" onclick={clearSpec}>Clear</button>
        {/if}
      </div>
    </div>

    <input
      bind:this={fileInputEl}
      type="file"
      accept=".json,.yaml,.yml,application/json,text/yaml,application/x-yaml"
      class="hidden"
      onchange={(e) => handleFileSelect(e.currentTarget.files?.[0])}
    />
    <div
      class="dropzone"
      class:dropzone-active={draggingFile}
      role="button"
      tabindex="0"
      ondragover={(e) => {
        e.preventDefault();
        draggingFile = true;
      }}
      ondragleave={() => (draggingFile = false)}
      ondrop={onFileDrop}
      onclick={() => fileInputEl?.click()}
      onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && fileInputEl?.click()}
    >
      <span
        class="flex h-10 w-10 items-center justify-center rounded-full bg-accent-soft text-accent"
      >
        <Icon name="upload" size={18} />
      </span>
      {#if specFileName}
        <div class="text-center">
          <p class="flex items-center justify-center gap-1.5 text-sm font-medium text-fg">
            <Icon name="file-text" size={15} class="text-accent" />
            <span class="mono">{specFileName}</span>
          </p>
          <p class="mt-0.5 text-xs text-muted">Click or drop another file to replace</p>
        </div>
      {:else}
        <div class="text-center">
          <p class="text-sm font-medium text-fg">
            Drop your OpenAPI / Swagger file here, or <span class="text-accent underline"
              >browse</span
            >
          </p>
          <p class="mt-0.5 text-xs text-faint">
            Supports .json, .yaml, .yml (Swagger 2.0 &amp; OpenAPI 3.x)
          </p>
        </div>
      {/if}
    </div>

    <div class="field">
      <label class="label" for="imp-spec">Specification content (JSON / YAML)</label>
      <textarea
        id="imp-spec"
        class="textarea mono h-48 text-xs"
        placeholder="openapi: 3.0.0 … or swagger: &quot;2.0&quot; …"
        bind:value={specText}></textarea>
    </div>

    <div
      class="flex flex-wrap items-center justify-between gap-4 rounded-lg border border-border bg-surface-2 px-4 py-3"
    >
      <div class="flex flex-wrap items-center gap-5">
        <label class="flex cursor-pointer items-center gap-2.5 text-sm text-fg">
          <input type="checkbox" class="cbx" bind:checked={createFolders} />
          <span>
            <span class="font-medium">Auto-create folders from tags</span>
            <span class="block text-xs text-muted">
              Group endpoints into folders based on Swagger/OpenAPI operation <code class="mono"
                >tags</code
              >
            </span>
          </span>
        </label>

        <label class="flex cursor-pointer items-center gap-2.5 text-sm text-fg">
          <input type="checkbox" class="cbx" bind:checked={deleteRemoved} />
          <span>
            <span class="font-medium">Prune removed endpoints</span>
            <span class="block text-xs text-muted">
              Delete existing mocks in this collection that are absent from the spec
            </span>
          </span>
        </label>
      </div>

      <button
        class="btn btn-secondary"
        disabled={!specText.trim() || previewing}
        onclick={doPreview}
      >
        <Icon name="search" size={16} />
        {previewing ? 'Analyzing…' : 'Preview Import'}
      </button>
    </div>
  </div>

  {#if preview}
    <div class="card space-y-4">
      <div class="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-3">
        <div>
          <div class="flex flex-wrap items-center gap-2">
            <h3 class="text-sm font-semibold text-fg">
              {preview.spec?.title || 'Specification Preview'}
            </h3>
            {#if preview.spec?.version}
              <span class="badge mono">v{preview.spec.version}</span>
            {/if}
            {#if preview.spec?.basePath}
              <span class="badge mono">{preview.spec.basePath}</span>
            {/if}
          </div>
          <div class="mt-1.5 flex flex-wrap items-center gap-3 text-xs">
            <span class="font-medium text-accent">+{preview.summary.added} added</span>
            <span class="font-medium text-[var(--m-put)]">~{preview.summary.changed} changed</span>
            <span class="text-muted">={preview.summary.unchanged} unchanged</span>
            {#if preview.summary.removed > 0}
              <span class="font-medium text-danger">−{preview.summary.removed} removed</span>
            {/if}
            {#if createFolders && (preview.folders?.length ?? 0) > 0}
              <span
                class="inline-flex items-center gap-1 rounded-md bg-accent-soft px-2 py-0.5 font-medium text-accent"
              >
                <Icon name="folder" size={13} />
                {preview.folders.length} tag folder{preview.folders.length === 1 ? '' : 's'}
                {#if preview.summary.foldersAdded > 0}
                  ({preview.summary.foldersAdded} new)
                {/if}
              </span>
            {/if}
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <div class="flex items-center gap-1 text-xs">
            <button type="button" class="btn btn-ghost btn-sm" onclick={() => selectByPreset('2xx')}
              >2xx only</button
            >
            <button type="button" class="btn btn-ghost btn-sm" onclick={() => selectByPreset('all')}
              >Select all</button
            >
            <button
              type="button"
              class="btn btn-ghost btn-sm"
              onclick={() => selectByPreset('none')}>Select none</button
            >
          </div>
          <button
            class="btn btn-primary"
            disabled={importing || (!selectedKeys.size && !deleteRemoved)}
            onclick={doImport}
          >
            <Icon name="check" size={16} />
            {#if importing}
              Importing…
            {:else}
              Import {selectedKeys.size} endpoint{selectedKeys.size === 1 ? '' : 's'}
              {#if createFolders && selectedFolderCount > 0}
                into {selectedFolderCount} folder{selectedFolderCount === 1 ? '' : 's'}
              {/if}
            {/if}
          </button>
        </div>
      </div>

      <div class="space-y-2.5">
        {#each groupedPreview as group (group.key)}
          {@const groupSelectedCount = group.items.filter((it) => selectedKeys.has(it.key)).length}
          {@const allGroupSelected =
            group.items.length > 0 && groupSelectedCount === group.items.length}
          {@const isCollapsed = collapsedPreviewFolders.has(group.key)}
          <div class="overflow-hidden rounded-lg border border-border bg-surface">
            <div class="flex items-center gap-2.5 bg-surface-2 px-3 py-2">
              <input
                type="checkbox"
                class="cbx"
                checked={allGroupSelected}
                indeterminate={groupSelectedCount > 0 && !allGroupSelected}
                onchange={() => toggleFolderGroup(group)}
                aria-label="Select folder {group.tag || 'root'}"
              />
              <button
                type="button"
                class="flex flex-1 items-center gap-2 text-left text-xs font-semibold text-fg"
                onclick={() => togglePreviewFolderCollapse(group.key)}
              >
                <Icon
                  name="chevron-down"
                  size={14}
                  class={isCollapsed ? '-rotate-90 text-faint' : 'text-faint'}
                />
                {#if group.tag}
                  <span class="text-accent"><Icon name="folder" size={15} /></span>
                  <span class="flex items-center gap-1">
                    {#each group.segments as seg, idx (idx)}
                      {#if idx > 0}<span class="text-faint">/</span>{/if}
                      <span>{seg}</span>
                    {/each}
                  </span>
                  {#if group.isNewFolder}
                    <span
                      class="rounded bg-accent-soft px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-accent"
                    >
                      new folder
                    </span>
                  {:else}
                    <span
                      class="rounded border border-border px-1.5 py-0.5 text-[10px] font-medium text-muted"
                    >
                      existing folder
                    </span>
                  {/if}
                {:else}
                  <span class="text-muted"><Icon name="layers" size={14} /></span>
                  <span class="text-muted">Collection root {createFolders ? '(untagged)' : ''}</span
                  >
                {/if}
              </button>
              <span class="mono text-xs text-muted">
                {groupSelectedCount}/{group.items.length} selected
              </span>
            </div>

            {#if !isCollapsed}
              <div class="divide-y divide-border">
                {#each group.items as it (it.key)}
                  <label
                    class="flex cursor-pointer items-center justify-between gap-3 px-3.5 py-2 transition-colors hover:bg-surface-hover"
                  >
                    <div class="flex min-w-0 items-center gap-2.5">
                      <input
                        type="checkbox"
                        class="cbx"
                        checked={selectedKeys.has(it.key)}
                        onchange={() => toggleItemKey(it.key)}
                      />
                      <span class={methodClass(it.method)}>{it.method}</span>
                      <span class="mono truncate text-xs font-medium text-fg"
                        >{it.pathTemplate}</span
                      >
                      <span class="mono text-xs font-semibold {statusTone(it.status)}"
                        >→ {it.status}</span
                      >
                      {#if it.summary || it.operationId}
                        <span class="hidden truncate text-xs text-muted sm:inline">
                          · {it.summary || it.operationId}
                        </span>
                      {/if}
                    </div>
                    <div class="flex shrink-0 items-center gap-2">
                      {#if !createFolders && it.tag}
                        <span class="badge mono text-[10px]">{it.tag}</span>
                      {/if}
                      {#if it.source}
                        <span class="badge text-[10px]">{it.source}</span>
                      {/if}
                      <span
                        class="mono text-[11px] font-medium {it.action === 'added'
                          ? 'text-accent'
                          : it.action === 'changed'
                            ? 'text-[var(--m-put)]'
                            : 'text-faint'}"
                      >
                        {it.action}
                      </span>
                    </div>
                  </label>
                {/each}
              </div>
            {/if}
          </div>
        {/each}

        {#if removedItems.length > 0}
          <div
            class="overflow-hidden rounded-lg border border-[color-mix(in_srgb,var(--danger)_35%,transparent)] bg-surface"
          >
            <div class="flex items-center justify-between bg-surface-2 px-3 py-2 text-xs">
              <span class="font-semibold text-danger">
                Absent from spec ({removedItems.length}) — {deleteRemoved
                  ? 'will be deleted'
                  : 'kept unless Prune is enabled'}
              </span>
            </div>
            <div class="divide-y divide-border">
              {#each removedItems as it (it.key)}
                <div class="flex items-center justify-between px-3.5 py-2 text-xs text-muted">
                  <span class="mono">{it.key}</span>
                  <span class="mono text-danger">removed</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
