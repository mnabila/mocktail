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
</script>

<div class="transition-colors {open ? 'bg-accent text-ink' : 'bg-dark-2 hover:bg-[#353531]'}">
	<button
		class="grid min-h-[72px] w-full grid-cols-[58px_minmax(0,4fr)_minmax(0,2fr)_72px_66px] items-center gap-4 px-[clamp(16px,3vw,36px)] text-left text-inherit focus-visible:outline-2 focus-visible:outline-offset-[-4px] max-md:min-h-16 max-md:grid-cols-[50px_minmax(0,1fr)_50px] max-md:gap-2.5 max-md:px-3.5 {open ? 'focus-visible:outline-ink' : 'focus-visible:outline-accent'}"
		type="button"
		onclick={onToggle}
		aria-expanded={open}
		aria-controls={`log-detail-${path}`}
	>
		<span class="inline-flex min-h-[30px] items-center justify-center px-1.5 font-display text-[13px] font-semibold tracking-[0.03em] max-md:min-h-[26px] {open ? 'bg-ink text-dark-ink' : 'bg-[#3d3d38] text-dark-ink'}">{method}</span>
		<span class="truncate font-display text-lead font-semibold tracking-[-0.025em]">{path}</span>
		<span class="truncate font-display text-sm font-medium max-md:hidden {open ? 'text-ink opacity-[0.66]' : 'text-dark-muted'}">{note}</span>
		<span class="text-right font-display text-sm tabular-nums max-md:hidden {open ? 'text-ink opacity-[0.66]' : 'text-dark-muted'}">{ms}</span>
		<span class="inline-flex h-[30px] items-center justify-center bg-tile font-display text-sm font-semibold text-ink tabular-nums max-md:h-[26px] {status < 400 && open ? 'bg-ink text-dark-ink' : status < 400 ? 'bg-accent text-ink' : 'bg-tile text-ink'}">{status}</span>
	</button>
	<div
		class="bg-accent px-[clamp(16px,3vw,36px)] pb-7 text-ink max-md:px-3.5 max-md:pb-5"
		id={`log-detail-${path}`}
		hidden={!open}
	>
		<p class="pb-[18px] font-display text-sm leading-[1.4] max-md:pb-3.5">
			<span class="font-semibold">Matched by</span>
			{match.join(' · ')}
		</p>
		<div class="grid grid-cols-2 gap-4 max-md:grid-cols-1 max-md:gap-3">
			<div>
				<h3 class="mb-2 font-display text-[13px] leading-[1.2] font-semibold tracking-normal">Request</h3>
				<pre class="overflow-x-auto bg-tile px-4 py-3.5 font-mono text-[13px] leading-[1.5] text-ink max-md:p-3"><code>{request}</code></pre>
			</div>
			<div>
				<h3 class="mb-2 font-display text-[13px] leading-[1.2] font-semibold tracking-normal">Response</h3>
				<pre class="overflow-x-auto bg-tile px-4 py-3.5 font-mono text-[13px] leading-[1.5] text-ink max-md:p-3"><code>{response}</code></pre>
			</div>
		</div>
	</div>
</div>
