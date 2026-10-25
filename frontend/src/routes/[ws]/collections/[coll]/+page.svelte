<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { createQuery } from '@tanstack/svelte-query';
  import {
    apiErrorMessage,
    collectionsQueryOptions,
    queryClient,
    workspacesQueryOptions
  } from '$lib/api/query';
  import { wsIdFromSlug } from '$lib/state/session.svelte';
  import type { Collection } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';
  import CollectionHeader from '$lib/features/collections/CollectionHeader.svelte';
  import CollectionCorsTab from '$lib/features/collections/CollectionCorsTab.svelte';
  import CollectionSettingsTab from '$lib/features/collections/CollectionSettingsTab.svelte';
  import CollectionImportTab from '$lib/features/openapi/CollectionImportTab.svelte';
  import CollectionLogsTab from '$lib/features/logs/CollectionLogsTab.svelte';

  const TABS = ['logs', 'import', 'cors', 'settings'] as const;

  let wsSlug = $derived(page.params.ws ?? '');
  let collSlug = $derived(page.params.coll ?? '');

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

  let loading = $derived(workspacesQuery.isPending || Boolean(wsId && collectionsQuery.isPending));
  let error = $derived.by(() => {
    const rawErr = workspacesQuery.error ?? collectionsQuery.error;
    if (rawErr) {
      if ((rawErr as { status?: number })?.status === 401) return 'unauthorized';
      return apiErrorMessage(rawErr, 'failed');
    }
    if (!loading && !coll) return 'collection not found';
    return '';
  });

  let tab = $state(page.url.searchParams.get('tab') ?? 'logs');

  // Sync URL ?tab=... parameter or legacy deep-links when navigating.
  $effect(() => {
    if (!coll) return;
    const sp = page.url.searchParams;
    const urlTab = sp.get('tab');
    if (urlTab && (TABS as readonly string[]).includes(urlTab) && urlTab !== tab) {
      tab = urlTab;
    }
    if (sp.get('newMock') === '1') {
      const folder = sp.get('folder');
      goto(`/${wsSlug}/collections/${collSlug}/new${folder ? `?folder=${folder}` : ''}`);
    } else if (sp.get('mock')) {
      goto(`/${wsSlug}/collections/${collSlug}/${sp.get('mock')}`);
    }
  });
</script>

{#if error && !coll}
  <p class="flex items-center gap-1.5 text-sm text-danger">
    <Icon name="alert" size={15} />
    {error}
  </p>
{/if}

{#if coll}
  <CollectionHeader {wsSlug} {coll} />

  <div class="mt-5 flex gap-1 border-b border-border" role="tablist">
    {#each TABS as t (t)}
      <button
        class="tab"
        class:tab-active={tab === t}
        role="tab"
        aria-selected={tab === t}
        onclick={() => (tab = t)}
      >
        {t}
      </button>
    {/each}
  </div>

  {#if tab === 'logs'}
    <CollectionLogsTab {wsSlug} {wsId} {collSlug} {coll} />
  {:else if tab === 'cors'}
    <CollectionCorsTab {wsId} {coll} />
  {:else if tab === 'settings'}
    <CollectionSettingsTab {wsSlug} {wsId} {coll} />
  {/if}
{/if}
