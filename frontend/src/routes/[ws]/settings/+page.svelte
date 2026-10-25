<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { createMutation, createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import {
    apiErrorMessage,
    meQueryOptions,
    queryClient,
    queryKeys,
    workspacesQueryOptions
  } from '$lib/api/query';
  import { wsIdFromSlug } from '$lib/state/session.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import { slugify } from '$lib/utils/format';
  import type { Workspace, WorkspaceRole } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';

  let wsSlug = $derived(page.params.ws ?? '');
  const workspacesQuery = createQuery(
    () => workspacesQueryOptions(),
    () => queryClient
  );
  const meQuery = createQuery(
    () => meQueryOptions(),
    () => queryClient
  );

  let wsList = $derived(workspacesQuery.data ?? []);
  let ws = $derived<Workspace | null>(wsList.find((w) => w.slug === wsSlug) ?? null);
  let wsId = $derived(wsIdFromSlug(wsList, wsSlug));
  let myRole = $derived<WorkspaceRole | null>(
    (meQuery.data?.memberships ?? []).find((m) => m.workspaceId === wsId)?.role ?? null
  );
  let loading = $derived(workspacesQuery.isPending || meQuery.isPending);
  let error = $derived(
    workspacesQuery.error
      ? apiErrorMessage(workspacesQuery.error, 'failed to load workspace')
      : meQuery.error
        ? apiErrorMessage(meQuery.error, 'failed to load workspace')
        : ''
  );

  let name = $state('');
  let slug = $state('');

  let isOwner = $derived(myRole === 'owner');
  let slugPreview = $derived(slugify(slug, 'workspace'));

  $effect(() => {
    if (ws) {
      name = ws.name;
      slug = ws.slug;
    }
  });

  const saveWorkspaceMutation = createMutation(
    () => ({
      mutationFn: ({
        id,
        nextName,
        nextSlug
      }: {
        id: string;
        nextName: string;
        nextSlug: string;
      }) => api.patch<Workspace>(`/workspaces/${id}`, { name: nextName, slug: nextSlug }),
      onSuccess: async (updated) => {
        await queryClient.invalidateQueries({ queryKey: queryKeys.workspaces() });
        toast('Workspace saved');
        goto(`/${updated.slug}/settings`);
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not save'), 'error');
      }
    }),
    () => queryClient
  );

  const deleteWorkspaceMutation = createMutation(
    () => ({
      mutationFn: (id: string) => api.del(`/workspaces/${id}`),
      onSuccess: async () => {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: queryKeys.workspaces() }),
          queryClient.invalidateQueries({ queryKey: queryKeys.me() })
        ]);
        toast('Workspace deleted');
        goto('/');
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not delete'), 'error');
      }
    }),
    () => queryClient
  );

  let saving = $derived(saveWorkspaceMutation.isPending);

  function save() {
    if (!ws || !name.trim() || saving) return;
    saveWorkspaceMutation.mutate({ id: ws.id, nextName: name, nextSlug: slug });
  }

  function destroy() {
    if (!ws || deleteWorkspaceMutation.isPending) return;
    if (
      !confirm(
        `Delete workspace "${ws.name}"? Every collection, mock, and member is removed. This cannot be undone.`
      )
    )
      return;
    deleteWorkspaceMutation.mutate(ws.id);
  }
</script>

<div>
  <h1 class="text-2xl font-semibold tracking-tight">Workspace Management</h1>
  <p class="mt-0.5 text-sm text-muted">Configure this workspace. Owner only.</p>
</div>

{#if loading}
  <p class="mt-6 text-muted">loading…</p>
{:else if error}
  <p class="mt-6 flex items-center gap-1.5 text-sm text-danger">
    <Icon name="alert" size={15} />
    {error}
  </p>
{:else if !isOwner}
  <div class="card mt-6 flex items-center gap-3">
    <span class="text-faint"><Icon name="settings" size={20} /></span>
    <p class="text-sm text-muted">Only the workspace owner can manage these settings.</p>
  </div>
{:else if ws}
  <div class="mt-6 max-w-2xl space-y-5">
    <!-- general -->
    <section class="card space-y-4">
      <h2 class="font-medium text-fg">General</h2>
      <div class="field">
        <label class="label" for="ws-name">Workspace name</label>
        <input id="ws-name" class="input" bind:value={name} />
      </div>
      <div class="field">
        <label class="label" for="ws-slug">Subdomain slug</label>
        <input id="ws-slug" class="input mono" bind:value={slug} />
        <p class="text-xs text-faint">
          Mock URLs resolve at
          <span class="mono text-muted">
            {slugPreview}.{typeof location !== 'undefined' ? location.host : 'mocktail'}
          </span>. Changing this breaks existing mock URLs.
        </p>
      </div>
      <button class="btn btn-primary" disabled={!name.trim() || saving} onclick={save}>
        <Icon name="check" size={16} />
        {saving ? 'Saving…' : 'Save changes'}
      </button>
    </section>

    <!-- danger -->
    <section class="card space-y-3 border-[color-mix(in_srgb,var(--danger)_35%,transparent)]">
      <div>
        <h2 class="font-medium text-fg">Danger zone</h2>
        <p class="mt-0.5 text-sm text-muted">
          Deleting the workspace removes all its collections, mocks, logs, and memberships.
        </p>
      </div>
      <button class="btn btn-danger" disabled={deleteWorkspaceMutation.isPending} onclick={destroy}>
        <Icon name="trash" size={16} /> Delete workspace
      </button>
    </section>
  </div>
{/if}
