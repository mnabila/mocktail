<script lang="ts">
	import './layout.css';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		href="https://fonts.googleapis.com/css2?family=Archivo:wght@400;500;600;700&family=Bricolage+Grotesque:opsz,wght@12..96,500..800&family=IBM+Plex+Mono:wght@400;500;600&display=swap"
		rel="stylesheet"
	/>
</svelte:head>

{@render children()}
