<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Logo from '$lib/components/ui/Logo.svelte';
  // SSO only. Full-page navigations to the Rust /auth routes.
  const providers: Array<{ id: string; label: string }> = [
    { id: 'google', label: 'Continue with Google' },
    { id: 'github', label: 'Continue with GitHub' },
    { id: 'gitlab', label: 'Continue with GitLab' }
  ];
</script>

<div class="relative flex h-full items-center justify-center overflow-hidden p-6">
  <!-- ambient accent glow -->
  <div class="glow" aria-hidden="true"></div>

  <div class="relative w-full max-w-sm">
    <div class="mb-6 flex flex-col items-center text-center">
      <span
        class="mb-3 flex h-12 w-12 items-center justify-center rounded-xl border border-border bg-surface text-accent"
      >
        <Logo size={26} />
      </span>
      <h1 class="text-2xl font-semibold tracking-tight">mocktail</h1>
      <p class="mt-1 text-sm text-muted">Mock APIs, fast.</p>
    </div>

    <div class="card space-y-2.5">
      {#each providers as p (p.id)}
        <a
          href={`/auth/${p.id}/login`}
          class="btn btn-secondary w-full justify-center gap-2.5 py-2.5"
        >
          <Icon name={p.id} size={18} />
          {p.label}
        </a>
      {/each}
    </div>

    <p class="mt-4 text-center text-xs text-faint">
      Only the providers your operator configured will work.
    </p>
  </div>
</div>

<style>
  .glow {
    position: absolute;
    top: 30%;
    left: 50%;
    width: 520px;
    height: 520px;
    transform: translate(-50%, -50%);
    background: radial-gradient(
      circle,
      color-mix(in srgb, var(--accent) 16%, transparent),
      transparent 62%
    );
    filter: blur(28px);
    pointer-events: none;
  }
</style>
