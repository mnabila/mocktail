<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { queryClient, workspacesQueryOptions } from '$lib/api/query';

  onMount(async () => {
    // Land on the first workspace, or onboarding if the user has none.
    try {
      const ws = await queryClient.fetchQuery(workspacesQueryOptions());
      if (ws.length) goto(`/${ws[0].slug}/overview`);
      else goto('/onboarding');
    } catch (e: unknown) {
      if ((e as { status?: number })?.status === 401) goto('/login');
    }
  });
</script>

<div class="text-muted">redirecting…</div>
