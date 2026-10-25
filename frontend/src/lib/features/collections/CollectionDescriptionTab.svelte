<script lang="ts">
  import { createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import { apiErrorMessage, queryClient, queryKeys } from '$lib/api/query';
  import { toast } from '$lib/state/toast.svelte';
  import type { Collection } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  interface Props {
    wsId: string | null;
    coll: Collection;
  }

  let { wsId, coll }: Props = $props();

  let description = $state(coll.description);

  $effect(() => {
    description = coll.description;
  });

  const saveDescriptionMutation = createMutation(
    () => ({
      mutationFn: async () => {
        await api.patch(`/collections/${coll.id}`, { description });
      },
      onSuccess: async () => {
        await queryClient.invalidateQueries({ queryKey: queryKeys.collections(wsId) });
        toast('Description saved');
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not save description'), 'error');
      }
    }),
    () => queryClient
  );
</script>

<div class="mt-4 max-w-2xl space-y-4">
  <div class="card space-y-4">
    <div class="field">
      <label class="label" for="c-desc">Description</label>
      <textarea
        id="c-desc"
        class="input min-h-[150px] block w-full"
        bind:value={description}
        placeholder="Describe this collection..."
      ></textarea>
    </div>
    <button
      class="btn btn-primary"
      disabled={saveDescriptionMutation.isPending}
      onclick={() => saveDescriptionMutation.mutate()}
    >
      <Icon name="check" size={16} />
      {saveDescriptionMutation.isPending ? 'Saving…' : 'Save'}
    </button>
  </div>
</div>
