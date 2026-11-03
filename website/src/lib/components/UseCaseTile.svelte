<script lang="ts">
	let { title, description, icon } = $props<{
		title: string;
		description: string;
		icon: 'zap' | 'shield' | 'cycle';
	}>();

	// Stroke-SVG icons in the logo mark's language (fill none, round joins).
	const paths: Record<string, string> = {
		zap: 'M13 3 4 14h7l-1 7 10-11h-7l1-7z',
		shield: 'M12 3 5 6v6c0 5 4 8 7 9 3-1 7-4 7-9V6l-7-3z',
		cycle: 'M20 7H9a5 5 0 0 0 0 10h2M4 17h11a5 5 0 0 0 0-10h-2 M17 4l3 3-3 3 M7 20l-3-3 3-3'
	};
</script>

<div class="reveal bg-tile p-6 max-md:p-5">
	<div class="mb-4 flex size-10 items-center justify-center bg-accent text-ink">
		<svg
			viewBox="0 0 24 24"
			class="size-5"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<path d={paths[icon]} />
		</svg>
	</div>
	<h3 class="mb-2 font-display text-body font-bold">{title}</h3>
	<p class="text-sm leading-relaxed text-muted">{description}</p>
</div>
