<script lang="ts">
  import { goto } from '$app/navigation';
  import { createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import { apiErrorMessage, queryClient, queryKeys } from '$lib/api/query';
  import { slugify } from '$lib/utils/format';
  import type { Workspace } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  let name = $state('');
  let error = $state('');

  // Live slug preview (DNS-label style), mirrors server derivation.
  let slugPreview = $derived(slugify(name, 'workspace'));

  const createWorkspaceMutation = createMutation(
    () => ({
      mutationFn: (workspaceName: string) =>
        api.post<Workspace>('/workspaces', { name: workspaceName }),
      onSuccess: async (ws) => {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: queryKeys.workspaces() }),
          queryClient.invalidateQueries({ queryKey: queryKeys.me() })
        ]);
        goto(`/${ws.slug}/overview`);
      },
      onError: (err) => {
        error = apiErrorMessage(err, 'failed');
      }
    }),
    () => queryClient
  );

  let busy = $derived(createWorkspaceMutation.isPending);

  function create() {
    if (!name.trim() || busy) return;
    error = '';
    createWorkspaceMutation.mutate(name);
  }
</script>

<div class="mx-auto max-w-md pt-6">
  <span
    class="mb-4 flex h-11 w-11 items-center justify-center rounded-xl border border-border bg-surface text-accent"
  >
    <Icon name="zap" size={22} />
  </span>
  <h1 class="text-2xl font-semibold tracking-tight">Create your workspace</h1>
  <p class="mt-1.5 text-sm text-muted">The name derives a slug used as your mock subdomain.</p>

  <div class="card mt-6 space-y-4">
    <div class="field">
      <label class="label" for="ws-name">Workspace name</label>
      <input
        id="ws-name"
        class="input"
        placeholder="Acme Inc."
        bind:value={name}
        onkeydown={(e) => e.key === 'Enter' && name && !busy && create()}
      />
      <p class="text-xs text-faint">
        Subdomain: <span class="mono text-muted">{slugPreview}</span>.mocktail
      </p>
    </div>

    {#if error}
      <p class="flex items-center gap-1.5 text-sm text-danger">
        <Icon name="alert" size={15} />
        {error}
      </p>
    {/if}

    <button class="btn btn-primary w-full" disabled={!name || busy} onclick={create}>
      {busy ? 'Creating…' : 'Create workspace'}
    </button>
  </div>
</div>
