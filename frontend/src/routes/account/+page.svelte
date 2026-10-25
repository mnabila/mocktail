<script lang="ts">
  import { createMutation, createQuery } from '@tanstack/svelte-query';
  import { meQueryOptions, queryClient, apiErrorMessage, queryKeys } from '$lib/api/query';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/state/toast.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';

  const meQuery = createQuery(
    () => meQueryOptions(),
    () => queryClient
  );
  let user = $derived(meQuery.data?.user ?? null);
  const providers = ['google', 'github', 'gitlab'] as const;

  let linked = $derived(new Set((meQuery.data?.providers ?? []).map((p) => p.provider)));

  let editing = $state(false);
  let username = $state('');
  let displayName = $state('');
  let updateError = $state('');

  const updateMutation = createMutation(
    () => ({
      mutationFn: (data: { username: string; displayName: string }) => api.patch('/me', data),
      onSuccess: async () => {
        toast('Profile updated');
        await queryClient.invalidateQueries({ queryKey: queryKeys.me() });
      },
      onError: (err) => {
        updateError = apiErrorMessage(err, 'Could not update profile');
      }
    }),
    () => queryClient
  );

  function save() {
    updateError = '';
    updateMutation.mutate({ username, displayName });
  }

  function cancel() {
    editing = false;
    username = user?.username ?? '';
    displayName = user?.displayName ?? '';
  }
</script>

<div class="mx-auto max-w-2xl">
  <h1 class="text-xl font-semibold tracking-tight">Account</h1>

  {#if user}
    <div class="mt-5 flex items-center gap-3">
      <span
        class="flex h-11 w-11 items-center justify-center rounded-full bg-surface-2 text-base font-semibold text-fg"
      >
        {user.displayName.slice(0, 1).toUpperCase()}
      </span>
      <div>
        <div class="font-medium text-fg">{user.displayName}</div>
        <div class="text-sm text-muted">@{user.username}</div>
      </div>
    </div>

    <section class="mt-8">
      <div class="flex items-center justify-between">
        <h2 class="text-xs font-medium uppercase tracking-wide text-faint">Profile</h2>
      </div>

      <div class="card mt-2">
        {#if editing}
          <div class="flex flex-col gap-4 p-4">
            <div class="field">
              <label class="label" for="prof-username">Username</label>
              <input
                id="prof-username"
                type="text"
                class="input"
                bind:value={username}
                placeholder="@username"
              />
            </div>
            <div class="field">
              <label class="label" for="prof-displayname">Display Name</label>
              <input
                id="prof-displayname"
                type="text"
                class="input"
                bind:value={displayName}
                placeholder="Your Name"
              />
            </div>
            {#if updateError}
              <p class="flex items-center gap-1.5 text-sm text-danger">
                <Icon name="alert" size={15} />
                {updateError}
              </p>
            {/if}
            <div class="flex justify-end gap-2">
              <button class="btn btn-ghost btn-sm" onclick={cancel}>Cancel</button>
              <button
                class="btn btn-primary btn-sm"
                disabled={updateMutation.isPending}
                onclick={save}
              >
                {updateMutation.isPending ? 'Saving...' : 'Save changes'}
              </button>
            </div>
          </div>
        {:else}
          <div class="p-4">
            <div class="flex items-center justify-between">
              <div>
                <div class="text-sm text-muted">Username</div>
                <div class="font-medium text-fg">@{user.username}</div>
              </div>
              <div>
                <div class="text-sm text-muted">Display Name</div>
                <div class="font-medium text-fg">{user.displayName}</div>
              </div>
              <button
                class="btn btn-ghost btn-sm"
                onclick={() => {
                  editing = true;
                  username = user.username;
                  displayName = user.displayName;
                }}
              >
                Edit
              </button>
            </div>
          </div>
        {/if}
      </div>
    </section>

    <section class="mt-8">
      <h2 class="text-xs font-medium uppercase tracking-wide text-faint">Connected Providers</h2>
      <div class="mt-3 flex flex-wrap items-center gap-x-6 gap-y-4">
        {#each providers as provider}
          {@const isLinked = linked.has(provider)}
          <div class="flex items-center gap-2">
            {#if isLinked}
              <div
                class="relative flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-border transition-all bg-green-500 text-white border-green-600 shadow-sm shadow-green-500/20"
              >
                <Icon name={provider} size={16} />
                <div
                  class="absolute -right-1 -top-1 flex h-3 w-3 items-center justify-center rounded-full bg-white text-green-600 shadow-sm"
                >
                  <Icon name="check" size={6} />
                </div>
              </div>
              <span class="text-xs font-medium text-fg">connected</span>
            {:else}
              <a
                href={`/auth/${provider}/connect`}
                class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-border bg-surface-1 text-faint transition-all hover:border-border-strong hover:bg-surface-2 hover:text-fg"
                title="Connect {provider}"
              >
                <Icon name={provider} size={16} />
              </a>
              <span class="text-xs text-faint">disconnected</span>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  {:else}
    <div class="mt-6 text-muted">loading…</div>
  {/if}
</div>
