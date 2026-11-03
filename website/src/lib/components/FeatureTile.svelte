<script lang="ts">
	let {
		title,
		description,
		bgColor = 'bg-surface',
		delay = '0ms'
	} = $props<{
		title: string;
		description: string;
		bgColor?: string;
		delay?: string;
	}>();
</script>

<div class="reveal {bgColor} p-6 max-md:p-5" style="--d: {delay}">
	<h3 class="mb-2 font-display text-body font-bold">{title}</h3>
	<p class="text-sm text-muted">{description}</p>
</div>
