<script lang="ts">
  import { createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import { apiErrorMessage, queryClient, queryKeys } from '$lib/api/query';
  import { toast } from '$lib/state/toast.svelte';
  import type { Collection } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  interface Props {
    wsId?: string | null;
    coll: Collection;
    onSaved?: () => void | Promise<void>;
  }

  let { wsId = null, coll, onSaved = undefined }: Props = $props();

  let allowOrigins = $state('*');
  let allowCredentials = $state(false);
  let maxAgeSecs = $state(600);

  $effect(() => {
    if (!coll?.cors) return;
    allowOrigins = Array.isArray(coll.cors.allowOrigins) ? coll.cors.allowOrigins.join(', ') : '*';
    allowCredentials = Boolean(coll.cors.allowCredentials);
    maxAgeSecs = coll.cors.maxAgeSecs ?? 600;
  });

  const saveCorsMutation = createMutation(
    () => ({
      mutationFn: () => {
        const origins = allowOrigins.trim();
        const parsedOrigins =
          origins === '*'
            ? '*'
            : origins
                .split(',')
                .map((s) => s.trim())
                .filter(Boolean);
        return api.patch<Collection>(`/collections/${coll.id}`, {
          cors: {
            allowOrigins: parsedOrigins,
            allowCredentials,
            maxAgeSecs: Number(maxAgeSecs)
          }
        });
      },
      onSuccess: async () => {
        await queryClient.invalidateQueries({
          queryKey: wsId ? queryKeys.collections(wsId) : ['collections']
        });
        await onSaved?.();
        toast('CORS policy saved');
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not save CORS policy'), 'error');
      }
    }),
    () => queryClient
  );
</script>

<div class="card mt-4 max-w-xl space-y-4">
  <div class="field">
    <label class="label" for="c-origins">Allowed origins</label>
    <input
      id="c-origins"
      class="input mono"
      placeholder="* or https://a.com, https://b.com"
      bind:value={allowOrigins}
    />
    <p class="text-xs text-faint">
      <span class="mono">*</span> for any, or a comma-separated list.
    </p>
  </div>
  <label class="flex cursor-pointer items-center gap-2 text-sm text-fg">
    <input type="checkbox" class="accent-[var(--accent)]" bind:checked={allowCredentials} />
    Allow credentials
  </label>
  <div class="field max-w-[12rem]">
    <label class="label" for="c-maxage">Preflight max-age (s)</label>
    <input id="c-maxage" type="number" class="input" bind:value={maxAgeSecs} />
  </div>
  <button
    class="btn btn-primary"
    disabled={saveCorsMutation.isPending}
    onclick={() => saveCorsMutation.mutate()}
  >
    <Icon name="check" size={16} />
    {saveCorsMutation.isPending ? 'Saving…' : 'Save CORS'}
  </button>
</div>
