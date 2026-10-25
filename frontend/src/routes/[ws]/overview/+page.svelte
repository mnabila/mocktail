<script lang="ts">
  import { page } from '$app/state';
  import { createQuery } from '@tanstack/svelte-query';
  import {
    collectionsQueryOptions,
    membersQueryOptions,
    queryClient,
    workspacesQueryOptions
  } from '$lib/api/query';
  import { wsIdFromSlug } from '$lib/state/session.svelte';
  import { mountUrl } from '$lib/utils/format';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/state/toast.svelte';

  let wsSlug = $derived(page.params.ws ?? '');
  const workspacesQuery = createQuery(
    () => workspacesQueryOptions(),
    () => queryClient
  );
  let wsId = $derived(wsIdFromSlug(workspacesQuery.data ?? [], wsSlug));

  const collectionsQuery = createQuery(
    () => collectionsQueryOptions(wsId),
    () => queryClient
  );
  const membersQuery = createQuery(
    () => membersQueryOptions(wsId, { silent: true }),
    () => queryClient
  );

  let collections = $derived(collectionsQuery.data ?? []);
  let members = $derived(membersQuery.data ?? []);
  let loading = $derived(
    workspacesQuery.isPending ||
      Boolean(wsId && (collectionsQuery.isPending || membersQuery.isPending))
  );

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    toast('URL copied to clipboard');
  }
</script>

<div class="space-y-8">
  <!-- Hero Header -->
  <header class="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
    <div class="space-y-1">
      <div class="text-[10px] font-bold uppercase tracking-[0.2em] text-muted/60">
        Workspace Context
      </div>
      <h1 class="text-4xl font-bold tracking-tighter text-fg sm:text-5xl">{wsSlug}</h1>
      <p class="text-sm text-muted max-w-md">
        Control center for your mock API surfaces. Orchestrate endpoints and manage collaborator access.
      </p>
    </div>
    <div class="flex gap-3">
      <a 
        href="/{wsSlug}/collections" 
        class="btn btn-ghost btn-sm h-9 px-4"
      >
        Collections
      </a>
      <a 
        href="/{wsSlug}/collections" 
        class="btn btn-primary btn-sm h-9 px-4 shadow-sm shadow-accent/20"
      >
        <Icon name="plus" size={14} />
        New Collection
      </a>
    </div>
  </header>

  <!-- High-Density Status Grid -->
  <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
    <!-- Stat Card: Total -->
    <div class="group relative overflow-hidden rounded-xl border border-border bg-surface p-5 transition-all hover:border-accent/30">
      <div class="flex items-start justify-between">
        <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-surface-2 text-muted group-hover:text-accent transition-colors">
          <Icon name="layers" size={18} />
        </div>
        <span class="text-[10px] font-mono uppercase text-muted/50">Total</span>
      </div>
      <div class="mt-4 flex items-baseline gap-2">
        <span class="text-3xl font-bold tracking-tighter text-fg">{collections.length}</span>
        <span class="text-xs font-medium text-muted">Collections</span>
      </div>
    </div>

    <!-- Stat Card: OpenAPI -->
    <div class="group relative overflow-hidden rounded-xl border border-border bg-surface p-5 transition-all hover:border-accent/30">
      <div class="flex items-start justify-between">
        <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-surface-2 text-muted group-hover:text-accent transition-colors">
          <Icon name="download" size={18} />
        </div>
        <span class="text-[10px] font-mono uppercase text-muted/50">Imported</span>
      </div>
      <div class="mt-4 flex items-baseline gap-2">
        <span class="text-3xl font-bold tracking-tighter text-accent">{collections.filter(c => c.source === 'openapi').length}</span>
        <span class="text-xs font-medium text-muted">OpenAPI</span>
      </div>
    </div>

    <!-- Stat Card: Manual -->
    <div class="group relative overflow-hidden rounded-xl border border-border bg-surface p-5 transition-all hover:border-accent/30">
      <div class="flex items-start justify-between">
        <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-surface-2 text-muted group-hover:text-accent transition-colors">
          <Icon name="pencil" size={18} />
        </div>
        <span class="text-[10px] font-mono uppercase text-muted/50">Native</span>
      </div>
      <div class="mt-4 flex items-baseline gap-2">
        <span class="text-3xl font-bold tracking-tighter text-fg">{collections.filter(c => c.source !== 'openapi').length}</span>
        <span class="text-xs font-medium text-muted">Manual</span>
      </div>
    </div>

    <!-- Stat Card: Team -->
    <div class="group relative overflow-hidden rounded-xl border border-border bg-surface p-5 transition-all hover:border-accent/30">
      <div class="flex items-start justify-between">
        <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-surface-2 text-muted group-hover:text-accent transition-colors">
          <Icon name="users" size={18} />
        </div>
        <span class="text-[10px] font-mono uppercase text-muted/50">Team</span>
      </div>
      <div class="mt-4 flex items-baseline gap-2">
        <span class="text-3xl font-bold tracking-tighter text-fg">{members.length}</span>
        <span class="text-xs font-medium text-muted">Members</span>
      </div>
    </div>
  </div>

  <div class="grid gap-8 lg:grid-cols-3">
    <!-- Main Content: Collections -->
    <section class="lg:col-span-2 space-y-8">
      <div class="space-y-4">
        <div class="flex items-center justify-between px-1">
          <h2 class="text-sm font-bold uppercase tracking-widest text-muted/80">Primary Collections</h2>
          <a href="/{wsSlug}/collections" class="text-xs font-medium text-accent hover:underline">View All →</a>
        </div>

        <div class="grid gap-3">
          {#each collections.slice(0, 5) as c (c.id)}
            <div class="group relative flex items-center gap-4 rounded-xl border border-border bg-surface p-4 transition-all hover:border-accent/40 hover:bg-surface-hover active:scale-[0.99]">
              <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-surface-2 text-muted transition-colors group-hover:bg-accent/10 group-hover:text-accent">
                <Icon name="layers" size={22} />
              </div>
              
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="truncate text-base font-semibold tracking-tight text-fg">{c.name}</span>
                  <span class="pill text-[9px] font-bold uppercase tracking-tighter px-1.5 py-0.5 rounded bg-surface-2 text-muted border border-border" class:pill-openapi={c.source === 'openapi'}>
                    {c.source}
                  </span>
                </div>
                <div class="flex items-center gap-2 mt-0.5">
                  <span class="mono text-xs text-muted/60 truncate">{mountUrl(wsSlug, c.slug, c.basePath)}</span>
                </div>
              </div>

              <div class="flex gap-1 opacity-0 transition-all translate-x-2 group-hover:opacity-100 group-hover:translate-x-0">
                <button 
                  class="btn btn-ghost btn-sm h-9 w-9 p-0 rounded-lg" 
                  onclick={() => copyToClipboard(mountUrl(wsSlug, c.slug, c.basePath))}
                  title="Copy Base URL"
                  aria-label="Copy Base URL"
                >
                  <Icon name="copy" size={16} />
                </button>
                <a href={`/${wsSlug}/collections/${c.slug}`} class="btn btn-ghost btn-sm h-9 w-9 p-0 rounded-lg" title="Manage" aria-label="Manage Collection">
                  <Icon name="external-link" size={16} />
                </a>
              </div>
            </div>
          {:else}
            <div class="flex h-40 items-center justify-center rounded-xl border border-dashed border-border bg-surface-hover/30">
              <div class="text-center space-y-2">
                <p class="text-sm font-medium text-muted">
                  {loading ? 'Synchronizing collections...' : 'No collections defined'}
                </p>
                {#if !loading}
                  <p class="text-xs text-muted/60">Start by importing a spec or creating a manual collection.</p>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    </section>

    <!-- Sidebar: Workspace Context -->
    <aside class="space-y-6">
      <div class="rounded-xl border border-border bg-surface p-6 shadow-sm">
        <div class="mb-5 flex items-center justify-between">
          <h3 class="text-xs font-bold uppercase tracking-widest text-muted/80">Collaborators</h3>
          <Icon name="users" size={14} class="text-muted/40" />
        </div>
        <div class="space-y-4">
          {#each members.slice(0, 5) as m (m.id)}
            <div class="flex items-center justify-between group">
              <div class="flex items-center gap-3 overflow-hidden">
                <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-accent/10 text-[10px] font-bold text-accent border border-accent/20">
                  {(m.displayName ?? '?').slice(0, 1).toUpperCase()}
                </div>
                <span class="truncate text-xs font-medium text-fg group-hover:text-accent transition-colors">{m.displayName}</span>
              </div>
              <span class="text-[9px] font-mono uppercase px-1.5 py-0.5 rounded bg-surface-2 text-muted border border-border">{m.role}</span>
            </div>
          {:else}
            <p class="text-xs text-muted italic">No members assigned.</p>
          {/each}
        </div>
        {#if members.length > 5}
          <a href={`/${wsSlug}/members`} class="mt-6 block text-center text-xs font-medium text-accent hover:text-accent/80 transition-colors">
            Manage Access →
          </a>
        {/if}
      </div>

      <div class="rounded-xl border border-border bg-surface-hover/30 p-6">
        <h3 class="mb-4 text-xs font-bold uppercase tracking-widest text-muted/80">Workspace Toolkit</h3>
        <div class="flex flex-col gap-3">
          <a href={`/${wsSlug}/collections`} class="group flex items-center justify-between p-2 rounded-lg transition-colors hover:bg-surface">
            <div class="flex items-center gap-3 text-xs text-muted group-hover:text-fg transition-colors">
              <Icon name="layers" size={14} />
              All Collections
            </div>
            <Icon name="chevron-right" size={12} class="text-muted/40 group-hover:text-fg transition-colors" />
          </a>
          <a href={`/${wsSlug}/members`} class="group flex items-center justify-between p-2 rounded-lg transition-colors hover:bg-surface">
            <div class="flex items-center gap-3 text-xs text-muted group-hover:text-fg transition-colors">
              <Icon name="users" size={14} />
              Access Control
            </div>
            <Icon name="chevron-right" size={12} class="text-muted/40 group-hover:text-fg transition-colors" />
          </a>
        </div>
      </div>
    </aside>
  </div>
</div>
