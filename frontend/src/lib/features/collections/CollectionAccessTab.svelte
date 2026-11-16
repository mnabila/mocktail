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

  let entries = $state<string[]>([]);

  $effect(() => {
    entries = [...(coll?.ipAllowlist ?? [])];
  });

  function addEntry() {
    entries.push('');
  }
  function removeEntry(i: number) {
    entries.splice(i, 1);
  }

  const saveMutation = createMutation(
    () => ({
      mutationFn: () => {
        const list = entries.map((s) => s.trim()).filter(Boolean);
        return api.patch<Collection>(`/collections/${coll.id}`, { ipAllowlist: list });
      },
      onSuccess: async () => {
        await queryClient.invalidateQueries({
          queryKey: wsId ? queryKeys.collections(wsId) : ['collections']
        });
        await onSaved?.();
        toast('IP allowlist saved');
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not save IP allowlist'), 'error');
      }
    }),
    () => queryClient
  );
</script>

<div class="card mt-4 max-w-xl space-y-4">
  <div class="flex items-center justify-between gap-2">
    <p class="text-sm font-medium text-fg">Allowed client IPs / CIDR ranges</p>
    <button class="btn btn-secondary btn-sm" onclick={addEntry}>
      <Icon name="plus" size={14} /> Add IP
    </button>
  </div>

  {#if entries.length}
    <div class="space-y-2">
      {#each entries as _entry, i (i)}
        <div class="flex items-center gap-2">
          <input
            class="input mono"
            placeholder="203.0.113.5 or 10.0.0.0/8"
            bind:value={entries[i]}
          />
          <button
            class="flex items-center justify-center px-2 text-faint hover:text-danger"
            onclick={() => removeEntry(i)}
            aria-label="Remove entry"
          >
            <Icon name="trash" size={15} />
          </button>
        </div>
      {/each}
    </div>
  {:else}
    <div
      class="flex flex-col items-center gap-2 rounded-lg border border-dashed border-border py-8 text-center"
    >
      <span class="text-faint"><Icon name="layers" size={22} /></span>
      <p class="text-sm text-muted">No entries - every client IP is allowed.</p>
    </div>
  {/if}

  <div
    class="flex gap-2.5 rounded-lg border border-border bg-[var(--surface-2)] px-3 py-2.5 text-sm text-fg"
    role="note"
  >
    <span class="mt-0.5 shrink-0 text-faint"><Icon name="alert" size={16} /></span>
    <p class="text-xs text-muted">
      Only requests to this collection's mock endpoints from a listed address are served; others get
      <strong>403</strong>. An <strong>empty list allows everyone</strong>. This does not affect the
      admin API. IPv4/IPv6 and CIDR ranges are accepted; invalid entries are rejected on save.
    </p>
  </div>

  <button
    class="btn btn-primary"
    disabled={saveMutation.isPending}
    onclick={() => saveMutation.mutate()}
  >
    <Icon name="check" size={16} />
    {saveMutation.isPending ? 'Saving…' : 'Save allowlist'}
  </button>
</div>
