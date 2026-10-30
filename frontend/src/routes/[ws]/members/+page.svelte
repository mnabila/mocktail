<script lang="ts">
  import { page } from '$app/state';
  import { createMutation, createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import {
    apiErrorMessage,
    meQueryOptions,
    membersQueryOptions,
    queryClient,
    queryKeys,
    workspacesQueryOptions
  } from '$lib/api/query';
  import { wsIdFromSlug } from '$lib/state/session.svelte';
  import { toast } from '$lib/state/toast.svelte';
  import type { WorkspaceMember, WorkspaceRole } from '$lib/types/domain';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select from '$lib/components/ui/Select.svelte';

  let wsSlug = $derived(page.params.ws ?? '');
  const workspacesQuery = createQuery(
    () => workspacesQueryOptions(),
    () => queryClient
  );
  const meQuery = createQuery(
    () => meQueryOptions(),
    () => queryClient
  );
  let wsId = $derived(wsIdFromSlug(workspacesQuery.data ?? [], wsSlug));

  const membersQuery = createQuery(
    () => membersQueryOptions(wsId),
    () => queryClient
  );

  let members = $derived<WorkspaceMember[]>(membersQuery.data ?? []);
  let meId = $derived<string | null>(meQuery.data?.user.id ?? null);
  let myRole = $derived<WorkspaceRole | null>(
    (meQuery.data?.memberships ?? []).find((m) => m.workspaceId === wsId)?.role ?? null
  );
  let loading = $derived(
    workspacesQuery.isPending || meQuery.isPending || Boolean(wsId && membersQuery.isPending)
  );
  let error = $derived(
    membersQuery.error
      ? apiErrorMessage(membersQuery.error, 'failed to load members')
      : workspacesQuery.error
        ? apiErrorMessage(workspacesQuery.error, 'failed to load workspace')
        : ''
  );

  const ROLES: WorkspaceRole[] = ['owner', 'editor', 'viewer'];
  const roleHint: Record<WorkspaceRole, string> = {
    owner: 'Full control: manage members, collections, settings.',
    editor: 'Create and edit mocks and collections.',
    viewer: 'Read-only access.'
  };

  let username = $state('');
  let inviteRole = $state<WorkspaceRole>('editor');
  let inviteError = $state('');

  let isOwner = $derived(myRole === 'owner');

  const inviteMutation = createMutation(
    () => ({
      mutationFn: ({ targetUsername, role }: { targetUsername: string; role: WorkspaceRole }) =>
        api.post(`/workspaces/${wsId}/members`, { username: targetUsername, role }),
      onSuccess: async () => {
        username = '';
        toast('Member added');
        await queryClient.invalidateQueries({ queryKey: queryKeys.members(wsId) });
      },
      onError: (err) => {
        inviteError = apiErrorMessage(err, 'Could not add member');
      }
    }),
    () => queryClient
  );

  const changeRoleMutation = createMutation(
    () => ({
      mutationFn: ({ memberId, role }: { memberId: string; role: string }) =>
        api.patch(`/workspaces/${wsId}/members/${memberId}`, { role }),
      onSuccess: async () => {
        toast('Role updated');
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: queryKeys.members(wsId) }),
          queryClient.invalidateQueries({ queryKey: queryKeys.me() })
        ]);
      },
      onError: async (err) => {
        toast(apiErrorMessage(err, 'Could not update role'), 'error');
        await queryClient.invalidateQueries({ queryKey: queryKeys.members(wsId) });
      }
    }),
    () => queryClient
  );

  const removeMemberMutation = createMutation(
    () => ({
      mutationFn: (memberId: string) => api.del(`/workspaces/${wsId}/members/${memberId}`),
      onSuccess: async () => {
        toast('Member removed');
        await queryClient.invalidateQueries({ queryKey: queryKeys.members(wsId) });
      },
      onError: (err) => {
        toast(apiErrorMessage(err, 'Could not remove member'), 'error');
      }
    }),
    () => queryClient
  );

  let inviting = $derived(inviteMutation.isPending);

  function invite() {
    if (!wsId || !username.trim() || inviting) return;
    inviteError = '';
    inviteMutation.mutate({ targetUsername: username.trim(), role: inviteRole });
  }

  function changeRole(m: WorkspaceMember, role: string) {
    if (!wsId) return;
    changeRoleMutation.mutate({ memberId: m.id, role });
  }

  function remove(m: WorkspaceMember) {
    if (!wsId) return;
    if (!confirm(`Remove ${m.displayName} from this workspace?`)) return;
    removeMemberMutation.mutate(m.id);
  }
</script>

<div>
  <h1 class="text-2xl font-semibold tracking-tight">User Management</h1>
  <p class="mt-0.5 text-sm text-muted">
    Invite registered users and manage their roles in this workspace.
  </p>
</div>

{#if error}
  <p class="mt-4 flex items-center gap-1.5 text-sm text-danger">
    <Icon name="alert" size={15} />
    {error}
  </p>
{/if}

<!-- invite (owner only) -->
{#if isOwner}
  <div class="card mt-5">
    <h2 class="flex items-center gap-1.5 font-medium text-fg">
      <Icon name="user" size={16} /> Invite a member
    </h2>
    <p class="mt-0.5 text-sm text-muted">
      Enter the username of someone who already has a mocktail account — invitations don't create
      accounts.
    </p>
    <div class="mt-4 flex flex-col gap-3 sm:flex-row sm:items-end">
      <div class="field flex-1">
        <label class="label" for="inv-username">Username</label>
        <input
          id="inv-username"
          type="text"
          class="input"
          placeholder="@username"
          bind:value={username}
          onkeydown={(e) => e.key === 'Enter' && username && !inviting && invite()}
        />
      </div>
      <div class="field sm:w-44">
        <label class="label" for="inv-role">Role</label>
        <Select
          id="inv-role"
          bind:value={inviteRole}
          options={ROLES.filter((r) => r !== 'owner')}
        />
      </div>
      <button class="btn btn-primary" disabled={!username.trim() || inviting} onclick={invite}>
        <Icon name="plus" size={16} />
        {inviting ? 'Adding…' : 'Add member'}
      </button>
    </div>
    {#if inviteError}
      <p class="mt-3 flex items-center gap-1.5 text-sm text-danger">
        <Icon name="alert" size={15} />
        {inviteError}
      </p>
    {/if}
    <p class="mt-2 text-xs text-faint">{roleHint[inviteRole]}</p>
  </div>
{/if}

<!-- members table -->
<div class="card mt-5 overflow-hidden p-0">
  <div class="overflow-x-auto">
  <table class="tbl">
    <thead>
      <tr>
        <th>Member</th>
        <th class="w-40">Role</th>
        <th class="w-16"></th>
      </tr>
    </thead>
    <tbody>
      {#each members as m (m.id)}
        <tr>
          <td>
            <div class="flex items-center gap-3">
              <span
                class="flex h-8 w-8 items-center justify-center rounded-full bg-surface-2 text-xs font-semibold text-fg"
              >
                {(m.displayName ?? '?').slice(0, 1).toUpperCase()}
              </span>
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span class="truncate font-medium text-fg">{m.displayName}</span>
                  {#if m.userId === meId}<span class="badge">you</span>{/if}
                </div>
                <div class="truncate text-xs text-muted">{m.email}</div>
              </div>
            </div>
          </td>
          <td>
            {#if isOwner}
              <Select size="sm" value={m.role} options={ROLES} onchange={(v) => changeRole(m, v)} />
            {:else}
              <span class="badge">{m.role}</span>
            {/if}
          </td>
          <td class="text-right">
            {#if isOwner}
              <button
                class="btn btn-ghost btn-sm text-faint hover:text-danger"
                onclick={() => remove(m)}
                aria-label="Remove"
              >
                <Icon name="trash" size={15} />
              </button>
            {/if}
          </td>
        </tr>
      {/each}
      {#if loading}
        <tr><td colspan="3" class="py-8 text-center text-faint">loading…</td></tr>
      {:else if !members.length}
        <tr><td colspan="3" class="py-8 text-center text-faint">No members.</td></tr>
      {/if}
    </tbody>
  </table>
  </div>
</div>

{#if !loading && !isOwner}
  <p class="mt-3 text-xs text-faint">Only workspace owners can invite members or change roles.</p>
{/if}
