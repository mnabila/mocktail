<script lang="ts">
  // Brand mark - the martini glass logo, mirrored from the marketing site
  // (website/src/routes/+page.svelte). Monochrome, inherits currentColor.
  let { size = 20, class: klass = '' }: { size?: number; class?: string } = $props();
</script>

<svg
  class={klass}
  width={size}
  height={size}
  viewBox="0 0 64 64"
  fill="none"
  stroke="currentColor"
  stroke-width="4"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  <path d="M13 15 L51 15 L32 38 Z" />
  <path d="M32 38 V51" />
  <path d="M22 51 H42" />
  <path d="M26 19 L21.5 24 L26 29" stroke-width="3.5" />
  <path d="M38 19 L42.5 24 L38 29" stroke-width="3.5" />
  <path d="M35 18 L29 30" stroke-width="3.5" />
  <circle cx="47" cy="12" r="3" fill="currentColor" stroke="none" />
</svg>
