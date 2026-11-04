<script lang="ts">
	let {
		method,
		path,
		note,
		ms,
		status,
		match,
		request,
		response,
		open = false,
		onToggle
	} = $props<{
		method: string;
		path: string;
		note: string;
		ms: string;
		status: number;
		match: string[];
		request: string;
		response: string;
		open?: boolean;
		onToggle: () => void;
	}>();

	const ok = $derived(status < 400);
</script>

<div class="border-t border-dark-line">
	<button
		class="group grid w-full grid-cols-[22px_60px_minmax(0,4fr)_minmax(0,2fr)_64px_58px] items-center gap-4 py-3.5 pr-[clamp(16px,3vw,36px)] pl-[clamp(16px,3vw,36px)] text-left font-mono text-[13px] focus-visible:outline-2 focus-visible:-outline-offset-4 focus-visible:outline-accent max-md:grid-cols-[18px_52px_minmax(0,1fr)_52px] max-md:gap-2.5 max-md:px-3.5"
		type="button"
		onclick={onToggle}
		aria-expanded={open}
		aria-controls={`log-detail-${path}`}
	>
		<span class="text-dark-muted transition-colors group-hover:text-accent">
			{open ? '▾' : '▸'}
		</span>
		<span
			class="inline-flex min-h-[26px] items-center justify-center px-1.5 text-[12px] font-semibold tracking-[0.04em] {ok
				? 'bg-dark-ink/10 text-dark-ink'
				: 'bg-danger/20 text-danger'}"
		>
			{method}
		</span>
		<span class="truncate font-sans text-[15px] font-medium tracking-[-0.01em] text-dark-ink">
			{path}
		</span>
		<span class="truncate text-dark-muted max-md:hidden">{note}</span>
		<span class="text-right text-dark-muted tabular-nums max-md:hidden">{ms}</span>
		<span class="justify-self-end font-semibold tabular-nums {ok ? 'text-accent' : 'text-danger'}">
			{status}
		</span>
	</button>

	<div
		class="grid gap-4 px-[clamp(16px,3vw,36px)] pb-7 max-md:grid-cols-1 max-md:px-3.5 max-md:pb-5 md:grid-cols-2"
		id={`log-detail-${path}`}
		hidden={!open}
	>
		<p class="col-span-full font-mono text-[12px] tracking-[0.04em] text-dark-muted">
			<span class="text-accent">matched_by</span> = {match.join(' · ')}
		</p>
		<div>
			<p class="tag mb-2 text-dark-muted">request</p>
			<pre
				class="overflow-x-auto border border-dark-line bg-dark px-4 py-3.5 font-mono text-[12.5px] leading-[1.55] text-dark-ink max-md:p-3"><code
					>{request}</code
				></pre>
		</div>
		<div>
			<p class="tag mb-2 text-dark-muted">response</p>
			<pre
				class="overflow-x-auto border border-dark-line bg-dark px-4 py-3.5 font-mono text-[12.5px] leading-[1.55] text-dark-ink max-md:p-3"><code
					>{response}</code
				></pre>
		</div>
	</div>
</div>
