<script lang="ts">
  import { toast } from '$lib/state/toast.svelte';
  import type { Collection } from '$lib/types/domain';
  import { mountUrl } from '$lib/utils/format';
  import Icon from '$lib/components/ui/Icon.svelte';

  interface Props {
    wsSlug: string;
    coll: Collection;
  }

  let { wsSlug, coll }: Props = $props();

  function copyMountUrl() {
    navigator.clipboard?.writeText(mountUrl(wsSlug, coll.slug, coll.basePath));
    toast('Mount URL copied');
  }
</script>

<div class="flex flex-wrap items-start justify-between gap-3">
  <div>
    <h1 class="text-xl font-semibold tracking-tight">{coll.name}</h1>
    <button
      class="group mt-1 inline-flex items-center gap-1.5 text-muted transition-colors hover:text-fg"
      onclick={copyMountUrl}
      title="Copy mount URL"
    >
      <span class="mono text-xs">{mountUrl(wsSlug, coll.slug, coll.basePath)}</span>
      <span class="opacity-0 transition-opacity group-hover:opacity-100"
        ><Icon name="copy" size={13} /></span
      >
    </button>
  </div>
  <div class="flex gap-2">
    <a class="btn btn-secondary btn-sm" href={`/__admin/collections/${coll.id}/export?format=json`}>
      <Icon name="download" size={15} /> JSON
    </a>
    <a class="btn btn-secondary btn-sm" href={`/__admin/collections/${coll.id}/export?format=yaml`}>
      <Icon name="download" size={15} /> YAML
    </a>
  </div>
</div>
