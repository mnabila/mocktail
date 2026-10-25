<script lang="ts">
  import { goto } from '$app/navigation';
  import type { Role, Workspace } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toggleSidebar } from '$lib/state/sidebar.svelte';

  interface Props {
    currentWs?: string | null;
    currentWsName?: string;
    wsList?: Workspace[];
    displayName?: string;
    username?: string;
    currentRole?: Role | null;
    wsMenu?: boolean;
    userMenu?: boolean;
  }

  let {
    currentWs = null,
    currentWsName = 'Workspace',
    wsList = [],
    displayName = 'account',
    username = 'username',
    currentRole = null,
    wsMenu = $bindable(false),
    userMenu = $bindable(false)
  }: Props = $props();

  function pickWs(slug: string) {
    wsMenu = false;
    goto(`/${slug}/collections`);
  }

  function logout() {
    const f = document.createElement('form');
    f.method = 'POST';
    f.action = '/auth/logout';
    document.body.appendChild(f);
    f.submit();
  }
</script>

<header class="flex h-14 shrink-0 items-center gap-2 border-b border-border bg-surface px-3">
  <button class="btn btn-ghost btn-sm" onclick={toggleSidebar} aria-label="Toggle sidebar">
    <Icon name="sidebar" size={18} />
  </button>
  <div class="mx-1 h-5 w-px bg-border"></div>

  <!-- workspace switcher -->
  <div class="relative">
    <button
      class="navbtn"
      onclick={(e) => {
        e.stopPropagation();
        wsMenu = !wsMenu;
        userMenu = false;
      }}
      aria-haspopup="menu"
      aria-expanded={wsMenu}
    >
      <span class="ws-avatar !h-6 !w-6 !rounded-md !text-[11px]"
        >{currentWsName.slice(0, 1).toUpperCase()}</span
      >
      <span class="max-w-[16ch] truncate text-sm font-medium text-fg">{currentWsName}</span>
      <Icon name="chevron-down" size={15} class="text-faint" />
    </button>
    {#if wsMenu}
      <div class="dropdown left-0 top-full mt-1 w-56" role="menu">
        {#each wsList as w (w.id)}
          <button
            class="menu-item w-full"
            class:text-accent={w.slug === currentWs}
            onclick={() => pickWs(w.slug)}
            role="menuitem"
          >
            <span class="ws-avatar !h-5 !w-5 !rounded-md !text-[10px]"
              >{w.name.slice(0, 1).toUpperCase()}</span
            >
            <span class="truncate">{w.name}</span>
            {#if w.slug === currentWs}<span class="ml-auto"><Icon name="check" size={14} /></span
              >{/if}
          </button>
        {/each}
        <div class="my-1 border-t border-border"></div>
        <a class="menu-item" href="/onboarding" role="menuitem"
          ><Icon name="plus" size={15} /> New workspace</a
        >
      </div>
    {/if}
  </div>

  <!-- profile (right) -->
  <div class="relative ml-auto">
    <button
      class="navbtn"
      onclick={(e) => {
        e.stopPropagation();
        userMenu = !userMenu;
        wsMenu = false;
      }}
      aria-haspopup="menu"
      aria-expanded={userMenu}
    >
      <span
        class="ws-avatar !h-6 !w-6 !rounded-full !bg-accent !text-[11px]"
        style="color:var(--accent-fg)"
      >
        {displayName.slice(0, 1).toUpperCase()}
      </span>
      <span class="hidden max-w-[16ch] truncate text-sm font-medium text-fg sm:inline"
        >{displayName}</span
      >
      <Icon name="chevron-down" size={15} class="text-faint" />
    </button>
    {#if userMenu}
      <div class="dropdown right-0 top-full mt-1 w-56" role="menu">
        <div class="border-b border-border px-3 py-2 flex items-start gap-3">
          <span class="ws-avatar !h-8 !w-8 !rounded-full !bg-accent !text-[12px]" style="color:var(--accent-fg)">
            {displayName.slice(0, 1).toUpperCase()}
          </span>
          <div class="flex flex-col min-w-0">
            <div class="truncate text-sm font-medium text-fg">{displayName}</div>
            <div class="truncate text-xs text-muted">@{username}</div>
          </div>
        </div>
        <a class="menu-item" href="/account" role="menuitem"
          ><Icon name="user" size={15} /> Account</a
        >
        {#if currentWs}
          <div class="my-1 border-t border-border"></div>
          <div
            class="px-3 pb-1 pt-1.5 text-[11px] font-semibold uppercase tracking-wide text-faint"
          >
            Management
          </div>
          <a class="menu-item" href={`/${currentWs}/members`} role="menuitem">
            <Icon name="users" size={15} /> User Management
          </a>
          {#if currentRole === 'owner'}
            <a class="menu-item" href={`/${currentWs}/settings`} role="menuitem">
              <Icon name="settings" size={15} /> Workspace Settings
            </a>
          {/if}
        {/if}
        <div class="my-1 border-t border-border"></div>
        <button class="menu-item w-full text-danger" onclick={logout} role="menuitem">
          <Icon name="log-out" size={15} /> Sign out
        </button>
      </div>
    {/if}
  </div>
</header>
