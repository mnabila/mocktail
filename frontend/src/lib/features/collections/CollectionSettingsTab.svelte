<script lang="ts">
  import { goto } from '$app/navigation';
  import { createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import { apiErrorMessage, queryClient, queryKeys } from '$lib/api/query';
  import { toast } from '$lib/state/toast.svelte';
  import type { Collection } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  interface Props {
    wsSlug: string;
    wsId: string | null;
    coll: Collection;
  }

  let { wsSlug, wsId, coll }: Props = $props();

  let newSlug = $state('');
  let newBase = $state('');
  let newDesc = $state('');

  $effect(() => {
    if (!coll) return;
    newSlug = coll.slug;
    newBase = coll.basePath;
    newDesc = coll.description;
  });

  const saveSettingsMutation = createMutation(
    () => ({
      mutationFn: async () => {
        const targetSlug = newSlug || coll.slug;
        await api.patch(`/collections/${coll.id}`, { slug: targetSlug, basePath: newBase, description: newDesc });
        return targetSlug;
      },
      onSuccess: async (targetSlug) => {
        await queryClient.invalidateQueries({ queryKey: queryKeys.collections(wsId) });
        toast('Settings saved');
        goto(`/${wsSlug}/collections/${targetSlug}`);
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not save settings'), 'error');
      }
    }),
    () => queryClient
  );

  const deleteCollectionMutation = createMutation(
    () => ({
      mutationFn: () => api.del(`/collections/${coll.id}`),
      onSuccess: async () => {
        queryClient.removeQueries({ queryKey: queryKeys.collectionTree(coll.id) });
        await queryClient.invalidateQueries({ queryKey: queryKeys.collections(wsId) });
        goto(`/${wsSlug}/collections`);
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not delete collection'), 'error');
      }
    }),
    () => queryClient
  );

  function deleteCollection() {
    if (!confirm(`Delete collection "${coll.name}"? All its mock URLs stop working.`)) return;
    deleteCollectionMutation.mutate();
  }
</script>

<div class="mt-4 max-w-xl space-y-4">
  <div class="card space-y-4">
    <div class="field">
      <label class="label" for="s-slug"
        >Slug <span class="text-faint">(changes every served URL)</span></label
      >
      <input id="s-slug" class="input mono" bind:value={newSlug} />
    </div>
    <div class="field">
      <label class="label" for="s-base">Base path</label>
      <input id="s-base" class="input mono" bind:value={newBase} />
    </div>
    <div class="field">
      <label class="label" for="s-desc">Description</label>
      <textarea
        id="s-desc"
        class="input min-h-[100px] block w-full"
        bind:value={newDesc}
        placeholder="Collection description..."
      ></textarea>
    </div>
    <button
      class="btn btn-primary"
      disabled={saveSettingsMutation.isPending}
      onclick={() => saveSettingsMutation.mutate()}
    >
      <Icon name="check" size={16} />
      {saveSettingsMutation.isPending ? 'Saving…' : 'Save'}
    </button>
  </div>

  <div class="card space-y-3 border-[color-mix(in_srgb,var(--danger)_35%,transparent)]">
    <div>
      <h3 class="font-medium text-fg">Danger zone</h3>
      <p class="mt-0.5 text-sm text-muted">Deleting stops every mock URL in this collection.</p>
    </div>
    <button
      class="btn btn-danger"
      disabled={deleteCollectionMutation.isPending}
      onclick={deleteCollection}
    >
      <Icon name="trash" size={16} /> Delete collection
    </button>
  </div>
</div>
