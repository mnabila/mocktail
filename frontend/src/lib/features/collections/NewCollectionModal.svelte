<script lang="ts">
  import { goto } from '$app/navigation';
  import { createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import { apiErrorMessage, queryClient, queryKeys } from '$lib/api/query';
  import { uiState } from '$lib/state/session.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import type {
    Collection,
    CollectionSource,
    ImportCommitResponse,
    Workspace
  } from '$lib/types/domain';
  import { extractSpecMeta } from '$lib/utils/format';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select from '$lib/components/ui/Select.svelte';

  interface Props {
    currentWs?: string | null;
    currentWsObj?: Workspace | null;
  }

  let { currentWs = null, currentWsObj = null }: Props = $props();

  let ncName = $state('');
  let ncSlug = $state('');
  let ncBasePath = $state('');
  let ncSource = $state<CollectionSource>('manual');
  let ncSpecText = $state('');
  let ncSpecFileName = $state('');
  let ncCreateFolders = $state(true);
  let ncDraggingFile = $state(false);
  let ncFileInput = $state<HTMLInputElement | null>(null);
  let ncError = $state('');

  // Reset form state each time the modal opens.
  $effect(() => {
    if (uiState.newCollectionOpen) {
      ncName = '';
      ncSlug = '';
      ncBasePath = '';
      ncError = '';
      ncSpecText = '';
      ncSpecFileName = '';
      ncSource = 'manual';
      ncCreateFolders = true;
    }
  });

  function tryAutofillFromSpec(text: string) {
    const meta = extractSpecMeta(text);
    if (!ncName && meta.title) ncName = meta.title;
    if (!ncBasePath && meta.basePath) ncBasePath = meta.basePath;
  }

  async function handleNcFile(file?: File) {
    if (!file) return;
    ncSpecFileName = file.name;
    ncSpecText = await file.text();
    tryAutofillFromSpec(ncSpecText);
    if (!ncName) {
      ncName = file.name.replace(/\.(openapi|swagger)?\.(json|ya?ml)$/i, '');
    }
  }

  function onNcDrop(e: DragEvent) {
    e.preventDefault();
    ncDraggingFile = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) handleNcFile(file);
  }

  function close() {
    uiState.newCollectionOpen = false;
  }

  const createCollectionMutation = createMutation(
    () => ({
      mutationFn: async () => {
        if (!currentWsObj) throw { error: 'workspace not loaded' };
        const body: Record<string, string> = {
          name: ncName,
          source: ncSource,
          basePath: ncBasePath
        };
        if (ncSlug) body.slug = ncSlug;
        const coll = await api.post<Collection>(`/workspaces/${currentWsObj.id}/collections`, body);
        let importRes: ImportCommitResponse | null = null;
        if (ncSource === 'openapi' && ncSpecText.trim()) {
          importRes = await api.post<ImportCommitResponse>(`/collections/${coll.id}/import`, {
            spec: ncSpecText,
            createFolders: ncCreateFolders
          });
        }
        return { coll, importRes, wsId: currentWsObj.id };
      },
      onSuccess: async ({ coll, importRes, wsId }) => {
        close();
        await queryClient.invalidateQueries({ queryKey: queryKeys.collections(wsId) });
        if (importRes) {
          await queryClient.invalidateQueries({ queryKey: queryKeys.collectionTree(coll.id) });
          uiState.expandTreeRequest = { collectionId: coll.id, ts: Date.now() };
          const folderMsg = importRes.foldersCreated
            ? ` into ${importRes.foldersCreated} folder${importRes.foldersCreated > 1 ? 's' : ''}`
            : '';
          toast(
            `Imported ${importRes.created} endpoint${importRes.created === 1 ? '' : 's'}${folderMsg}`
          );
          goto(`/${currentWs}/collections/${coll.slug}`);
        } else if (ncSource === 'openapi') {
          goto(`/${currentWs}/collections/${coll.slug}?tab=import`);
        } else {
          goto(`/${currentWs}/collections/${coll.slug}`);
        }
      },
      onError: (err) => {
        ncError = apiErrorMessage(err, 'failed');
      }
    }),
    () => queryClient
  );

  let ncCreating = $derived(createCollectionMutation.isPending);

  function createCollection() {
    if (!currentWsObj) return;
    ncError = '';
    createCollectionMutation.mutate();
  }
</script>

{#if uiState.newCollectionOpen}
  <Modal title="New collection" size={ncSource === 'openapi' ? 'lg' : 'md'} onClose={close} overflowVisible>
    {#if ncError}
      <p class="mb-4 flex items-center gap-1.5 text-sm text-danger">
        <Icon name="alert" size={15} />
        {ncError}
      </p>
    {/if}
    <div class="grid gap-4 sm:grid-cols-2">
      <div class="field">
        <label class="label" for="nc-name">Name</label>
        <input id="nc-name" class="input" placeholder="Todo API" bind:value={ncName} />
      </div>
      <div class="field">
        <label class="label" for="nc-slug">Slug <span class="text-faint">(optional)</span></label>
        <input id="nc-slug" class="input mono" placeholder="todo" bind:value={ncSlug} />
      </div>
      <div class="field">
        <label class="label" for="nc-base">Base path</label>
        <input id="nc-base" class="input mono" placeholder="/api/v1" bind:value={ncBasePath} />
      </div>
      <div class="field">
        <label class="label" for="nc-src">Start from</label>
        <Select
          id="nc-src"
          bind:value={ncSource}
          options={[
            { value: 'manual', label: 'Scratch (manual)' },
            { value: 'openapi', label: 'OpenAPI / Swagger import' }
          ]}
        />
      </div>
    </div>

    {#if ncSource === 'openapi'}
      <div class="mt-4 space-y-3 border-t border-border pt-4">
        <input
          bind:this={ncFileInput}
          type="file"
          accept=".json,.yaml,.yml,application/json,text/yaml,application/x-yaml"
          class="hidden"
          onchange={(e) => handleNcFile(e.currentTarget.files?.[0])}
        />
        <div
          class="dropzone !py-4"
          class:dropzone-active={ncDraggingFile}
          role="button"
          tabindex="0"
          ondragover={(e) => {
            e.preventDefault();
            ncDraggingFile = true;
          }}
          ondragleave={() => (ncDraggingFile = false)}
          ondrop={onNcDrop}
          onclick={() => ncFileInput?.click()}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && ncFileInput?.click()}
        >
          <span
            class="flex h-8 w-8 items-center justify-center rounded-full bg-accent-soft text-accent"
          >
            <Icon name="upload" size={16} />
          </span>
          {#if ncSpecFileName}
            <p class="flex items-center gap-1.5 text-xs font-medium text-fg">
              <Icon name="file-text" size={14} class="text-accent" />
              <span class="mono">{ncSpecFileName}</span>
            </p>
          {:else}
            <p class="text-xs text-muted">
              Drop a Swagger / OpenAPI file (<code class="mono">.json</code>,
              <code class="mono">.yaml</code>) or
              <span class="text-accent underline">browse</span> to auto-fill &amp; import
            </p>
          {/if}
        </div>

        <div class="field">
          <label class="label" for="nc-spec">
            Or paste specification
            <span class="text-faint"
              >(optional — you can also preview in the Import tab after creation)</span
            >
          </label>
          <textarea
            id="nc-spec"
            class="textarea mono h-32 text-xs"
            placeholder="openapi: 3.0.0 … or swagger: &quot;2.0&quot; …"
            bind:value={ncSpecText}
            oninput={() => tryAutofillFromSpec(ncSpecText)}></textarea>
        </div>

        <label class="flex cursor-pointer items-center gap-2 text-xs text-fg">
          <input type="checkbox" class="cbx" bind:checked={ncCreateFolders} />
          <span>Auto-create folders based on Swagger / OpenAPI <code class="mono">tags</code></span>
        </label>
      </div>
    {/if}

    {#snippet footer()}
      <button class="btn btn-primary" disabled={!ncName || ncCreating} onclick={createCollection}>
        {#if ncCreating}
          {ncSource === 'openapi' && ncSpecText.trim() ? 'Importing…' : 'Creating…'}
        {:else}
          {ncSource === 'openapi' && ncSpecText.trim() ? 'Create & Import' : 'Create collection'}
        {/if}
      </button>
      <button class="btn btn-ghost" onclick={close}>Cancel</button>
    {/snippet}
  </Modal>
{/if}
