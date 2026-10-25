<script lang="ts">
  import '../app.css';
  import { onMount, type Snippet } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { createQuery, QueryClientProvider } from '@tanstack/svelte-query';
  import { meQueryOptions, queryClient, workspacesQueryOptions } from '$lib/api/query';
  import { uiState } from '$lib/state/session.svelte';
  import {
    sidebarState,
    initSidebar,
    startSidebarResize,
    closeMobileNav
  } from '$lib/state/sidebar.svelte';
  import AppNavbar from '$lib/components/layout/AppNavbar.svelte';
  import AppSidebar from '$lib/components/layout/AppSidebar.svelte';
  import NewCollectionModal from '$lib/features/collections/NewCollectionModal.svelte';
  import Toasts from '$lib/components/ui/Toasts.svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  let wsMenu = $state(false);
  let userMenu = $state(false);
  let menuFor = $state<string | null>(null);

  onMount(() => {
    initSidebar();
  });

  let isLoginRoute = $derived(page.url.pathname === '/login');
  const meQuery = createQuery(
    () => meQueryOptions(!isLoginRoute),
    () => queryClient
  );
  const workspacesQuery = createQuery(
    () => workspacesQueryOptions(!isLoginRoute),
    () => queryClient
  );

  $effect(() => {
    if (meQuery.data) uiState.me = meQuery.data;
  });

  $effect(() => {
    if (workspacesQuery.data) uiState.workspaces = workspacesQuery.data;
  });

  $effect(() => {
    const err = (meQuery.error ?? workspacesQuery.error) as { status?: number } | null;
    if (!isLoginRoute && err?.status === 401) {
      goto('/login');
    }
  });

  let ready = $derived(isLoginRoute || (!meQuery.isPending && !workspacesQuery.isPending));
  let wsList = $derived(workspacesQuery.data ?? uiState.workspaces);
  let me = $derived(meQuery.data ?? uiState.me);
  let currentWs = $derived(page.params.ws);
  let currentWsObj = $derived(wsList.find((w) => w.slug === currentWs));
  let currentWsName = $derived(currentWsObj?.name ?? wsList[0]?.name ?? 'Workspace');
  let displayName = $derived(me?.user?.displayName ?? 'account');
  let username = $derived(me?.user?.username ?? '');
  let currentRole = $derived(
    (me?.memberships ?? []).find((m) => m.workspaceId === currentWsObj?.id)?.role ?? null
  );

  function closeAll() {
    wsMenu = false;
    userMenu = false;
    menuFor = null;
  }
</script>

<svelte:window onclick={closeAll} />

<QueryClientProvider client={queryClient}>
  {#if !ready}
    <div class="flex h-full items-center justify-center gap-2 text-muted">
      <span class="loader"></span> loading…
    </div>
  {:else if page.url.pathname === '/login'}
    {@render children()}
  {:else}
    <div class="flex h-full bg-bg">
      <!-- sidebar: full height on the left (resizable, toggleable) -->
      {#if sidebarState.open}
        <aside
          class="relative hidden shrink-0 flex-col border-r border-border bg-surface md:flex"
          style="width:{sidebarState.width}px"
        >
          <AppSidebar {currentWs} {currentWsObj} bind:menuFor />
          <button
            type="button"
            tabindex="-1"
            class="resize-handle"
            class:resizing={sidebarState.resizing}
            onmousedown={startSidebarResize}
            aria-label="Resize sidebar"
          ></button>
        </aside>
      {/if}

      <!-- mobile drawer -->
      {#if sidebarState.mobileNav}
        <div class="fixed inset-0 z-50 md:hidden">
          <div
            class="absolute inset-0 bg-black/40"
            role="button"
            tabindex="-1"
            onclick={closeMobileNav}
            onkeydown={(e) => e.key === 'Escape' && closeMobileNav()}
          ></div>
          <aside class="absolute inset-y-0 left-0 flex w-72 flex-col bg-surface shadow-md">
            <AppSidebar {currentWs} {currentWsObj} bind:menuFor />
          </aside>
        </div>
      {/if}

      <!-- right column: navbar on top of the main content -->
      <div class="flex min-w-0 flex-1 flex-col">
        <AppNavbar
          {currentWs}
          {currentWsName}
          {wsList}
          {displayName}
          {username}
          {currentRole}
          bind:wsMenu
          bind:userMenu
        />

        <main class="min-w-0 flex-1 overflow-auto px-4 py-6 md:px-8">
          <div
            class={page.route.id?.startsWith('/[ws]/collections/[coll]') ? '' : 'mx-auto max-w-6xl'}
          >
            {@render children()}
          </div>
        </main>
      </div>
    </div>
  {/if}

  <NewCollectionModal {currentWs} {currentWsObj} />
  <Toasts />
</QueryClientProvider>

<style>
  :global(.navbtn) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 0.5rem;
    border: 1px solid transparent;
    padding: 0.3125rem 0.5rem;
    cursor: pointer;
    transition: background-color 150ms;
  }
  :global(.navbtn:hover) {
    background: var(--surface-hover);
  }
  .resize-handle {
    position: absolute;
    top: 0;
    right: -3px;
    z-index: 20;
    height: 100%;
    width: 6px;
    border: 0;
    padding: 0;
    cursor: col-resize;
    background: transparent;
    transition: background-color 150ms;
  }
  .resize-handle:hover,
  .resize-handle.resizing {
    background: color-mix(in srgb, var(--accent) 40%, transparent);
  }
  :global(.ws-card) {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 0.625rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--surface);
    padding: 0.5rem 0.625rem;
    cursor: pointer;
    transition: background-color 150ms;
  }
  :global(.ws-card:hover) {
    background: var(--surface-hover);
  }
  :global(.ws-avatar) {
    display: flex;
    height: 2rem;
    width: 2rem;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    border-radius: 0.5rem;
    background: #1c1c1e;
    color: #fff;
    font-size: 0.8rem;
    font-weight: 600;
  }
  :global(.kbd) {
    border-radius: 4px;
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    padding: 1px 5px;
    font-size: 10px;
    color: var(--faint);
  }
  :global(.nav-group) {
    padding: 0 0.75rem 0.375rem;
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--faint);
  }
  :global(.group-toggle) {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.375rem;
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--faint);
    cursor: pointer;
    border-radius: 0.375rem;
    transition: color 150ms;
  }
  :global(.group-toggle:hover) {
    color: var(--muted);
  }
  :global(.dropdown) {
    position: absolute;
    z-index: 50;
    overflow: hidden;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--surface);
    padding: 0.25rem;
    box-shadow: var(--shadow-md);
  }
  :global(.menu-item) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 0.375rem;
    padding: 0.5rem 0.625rem;
    font-size: 0.875rem;
    color: var(--fg);
    transition: background-color 150ms;
  }
  :global(.menu-item:hover) {
    background: var(--surface-hover);
  }
  :global(.side-link) {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    border-radius: 0.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--muted);
    transition: all 150ms;
  }
  :global(.side-link:hover) {
    background: var(--surface-hover);
    color: var(--fg);
  }
  :global(.side-link-active) {
    background: var(--accent-soft);
    color: var(--accent);
  }
  :global(.sub-link) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 0.5rem;
    padding: 0.375rem 0.75rem 0.375rem 1rem;
    font-size: 0.8125rem;
    color: var(--muted);
    transition: all 150ms;
  }
  :global(.sub-link:hover) {
    background: var(--surface-hover);
    color: var(--fg);
  }
  :global(.sub-link-active) {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 500;
  }
  :global(.sub-link .dot) {
    width: 5px;
    height: 5px;
    flex-shrink: 0;
    border-radius: 9999px;
    background: var(--border-strong);
  }
  :global(.sub-link-active .dot) {
    background: var(--accent);
  }
  :global(.tree-row) {
    display: flex;
    align-items: center;
    gap: 0.125rem;
    border-radius: 0.375rem;
    padding-right: 0.25rem;
    color: var(--muted);
  }
  :global(.tree-row:hover) {
    background: var(--surface-hover);
  }
  :global(.tree-active) {
    background: var(--accent-soft);
  }
  :global(.tree-active .tree-label) {
    color: var(--accent);
    font-weight: 500;
  }
  :global(.tree-row.drop-target) {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  :global(.tree-row.drop-target .tree-label) {
    color: var(--accent);
  }
  :global(.tree-caret) {
    display: flex;
    flex-shrink: 0;
    padding: 0.375rem 0.125rem;
    color: var(--faint);
    cursor: pointer;
  }
  :global(.tree-caret:hover) {
    color: var(--fg);
  }
  :global(.tree-label) {
    flex: 1;
    min-width: 0;
    padding: 0.375rem 0;
    font-size: 0.8125rem;
    color: inherit;
    cursor: pointer;
  }
  :global(.tree-row:hover .tree-label) {
    color: var(--fg);
  }
  :global(.tree-add) {
    display: flex;
    flex-shrink: 0;
    padding: 0.25rem;
    color: var(--faint);
    opacity: 0;
    cursor: pointer;
    transition: opacity 150ms;
  }
  :global(.tree-row:hover .tree-add) {
    opacity: 1;
  }
  :global(.tree-add:hover) {
    color: var(--fg);
  }
  :global(.tree-more) {
    display: flex;
    flex-shrink: 0;
    padding: 0.25rem;
    border-radius: 0.375rem;
    color: var(--muted);
    background: var(--surface-2);
    opacity: 1;
    cursor: pointer;
    transition: all 150ms;
  }
  :global(.tree-more:hover),
  :global(.tree-more-open) {
    color: var(--fg);
    background: var(--surface-hover);
  }
  :global(.req-row) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 0.375rem;
    padding: 0.3125rem 0.5rem;
    font-size: 0.75rem;
    color: var(--muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    cursor: pointer;
  }
  :global(.req-row:hover) {
    background: var(--surface-hover);
    color: var(--fg);
  }
  :global(.req-row.req-active) {
    background: var(--accent-soft);
    color: var(--fg);
    font-weight: 500;
  }
  :global(.req-row:active) {
    cursor: grabbing;
  }
  :global(.req-method) {
    width: 2.6rem;
    flex-shrink: 0;
    font-size: 0.625rem;
    font-weight: 700;
    letter-spacing: 0.02em;
  }
  .loader {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
