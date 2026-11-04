<script lang="ts">
	import { reveal } from '#lib/actions/reveal';
	import LogEntry from '#lib/components/LogEntry.svelte';

	type LogRow = {
		method: string;
		path: string;
		note: string;
		ms: string;
		status: number;
		match: string[];
		request: string;
		response: string;
	};

	const logRows: LogRow[] = [
		{
			method: 'GET',
			path: '/v1/users/42',
			note: 'users.get',
			ms: '12 ms',
			status: 200,
			match: ['method GET', 'path /v1/users/:id', 'header x-mock-env = dev'],
			request: 'GET /v1/users/42?expand=team\nx-mock-env: dev',
			response: '{\n  "id": "42",\n  "name": "Ada Lovelace",\n  "team": "analytics",\n  "plan": "pro"\n}'
		},
		{
			method: 'POST',
			path: '/v1/orders',
			note: 'orders.create',
			ms: '8 ms',
			status: 201,
			match: ['method POST', 'path /v1/orders', 'body.sku matches ^MOCK-'],
			request: 'POST /v1/orders\ncontent-type: application/json\n\n{ "sku": "MOCK-01", "qty": 2 }',
			response: '{\n  "id": "ord_1042",\n  "status": "created",\n  "total": 84.00\n}'
		},
		{
			method: 'GET',
			path: '/v1/users/42/team',
			note: 'users.team',
			ms: '24 ms',
			status: 200,
			match: ['method GET', 'path /v1/users/:id/team', 'query.expand = members'],
			request: 'GET /v1/users/42/team?expand=members',
			response: '{\n  "members": [\n    "ada",\n    "grace",\n    "linus"\n  ]\n}'
		},
		{
			method: 'GET',
			path: '/v1/admin',
			note: 'unmatched',
			ms: '3 ms',
			status: 401,
			match: ['no collection matched', 'fallback response'],
			request: 'GET /v1/admin\nx-mock-env: dev',
			response: '{\n  "error": "no mock matched",\n  "request_id": "req_8f2a"\n}'
		},
		{
			method: 'POST',
			path: '/v1/checkout',
			note: 'fault injected',
			ms: '802 ms',
			status: 503,
			match: ['method POST', 'path /v1/checkout', 'fault: delay 800 ms, then 503'],
			request: 'POST /v1/checkout\ncontent-type: application/json\n\n{ "cart": "cart_771" }',
			response: '{\n  "error": "upstream unavailable",\n  "retry_after": 30\n}'
		},
		{
			method: 'GET',
			path: '/v1/openapi.json',
			note: 'spec export',
			ms: '6 ms',
			status: 200,
			match: ['method GET', 'path /v1/openapi.json', 'export collection'],
			request: 'GET /v1/openapi.json',
			response: '{\n  "openapi": "3.1.0",\n  "paths": { "...": "..." }\n}'
		},
		{
			method: 'PATCH',
			path: '/v1/orders/1042',
			note: 'orders.patch',
			ms: '17 ms',
			status: 200,
			match: ['method PATCH', 'path /v1/orders/:id', 'body.status = shipped'],
			request: 'PATCH /v1/orders/1042\n\n{ "status": "shipped" }',
			response: '{\n  "id": "1042",\n  "status": "shipped",\n  "updated_at": "{{now}}"\n}'
		}
	];

	const capabilities = [
		{ id: 'CAP-01', name: 'OpenAPI 2.0 / 3.x', spec: 'import · diff · commit · export' },
		{ id: 'CAP-02', name: 'Request matching', spec: 'method · path · query · header · body' },
		{ id: 'CAP-03', name: 'Dynamic templates', spec: '{{path}} · {{query}} · {{header}} · {{body}}' },
		{ id: 'CAP-04', name: 'Fault injection', spec: 'delay · malformed · connection reset' },
		{ id: 'CAP-05', name: 'Request logs', spec: 'matched · unmatched · full transcript' },
		{ id: 'CAP-06', name: 'Workspaces & RBAC', spec: 'host-routed · owner / editor / viewer · SSO' }
	];

	const useCases = [
		{
			id: '01',
			title: 'Frontend parallelism',
			icon: 'M13 3 4 14h7l-1 7 10-11h-7l1-7z',
			body: 'Stop waiting for the backend. Import the OpenAPI spec and build your UI against realistic, dynamic mocks from day one.'
		},
		{
			id: '02',
			title: 'Resilience testing',
			icon: 'M12 3 5 6v6c0 5 4 8 7 9 3-1 7-4 7-9V6l-7-3z',
			body: 'Inject faults on purpose. Simulate 503s, 800 ms lag, or intermittent timeouts to prove your app fails gracefully.'
		},
		{
			id: '03',
			title: 'Deterministic CI',
			icon: 'M20 7H9a5 5 0 0 0 0 10h2M4 17h11a5 5 0 0 0 0-10h-2 M17 4l3 3-3 3 M7 20l-3-3 3-3',
			body: 'Replace flaky staging APIs with a local Mocktail instance in your pipeline. Fast, private, consistent every run.'
		}
	];

	const ticker = [
		'MATCHING',
		'TEMPLATING',
		'OPENAPI 3.1',
		'FAULT INJECTION',
		'REQUEST LOGS',
		'RBAC / SSO',
		'SQLITE',
		'CLOUDFLARE D1',
		'SINGLE BINARY',
		'EDGE READY'
	];

	let openRow = $state(4);
</script>

{#snippet mark()}
	<svg
		viewBox="0 0 64 64"
		class="size-full"
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
{/snippet}

{#snippet sectionTag(num: string, label: string)}
	<div class="flex items-center gap-3 text-ink">
		<span class="tag text-accent">{num}</span>
		<span class="h-px w-6 bg-ink/25"></span>
		<span class="tag">{label}</span>
	</div>
{/snippet}

<svelte:head>
	<title>Mocktail — Self-hostable API mocking in Rust</title>
	<meta
		name="description"
		content="Define matches, template responses, import OpenAPI, and inspect every request. Self-hostable API mocking in one Rust binary, on SQLite or Cloudflare D1."
	/>
	<meta property="og:title" content="Mocktail — Self-hostable API mocking in Rust" />
	<meta
		property="og:description"
		content="Define matches, template responses, import OpenAPI, and inspect every request. One binary, SQLite or D1."
	/>
	<meta property="og:type" content="website" />
</svelte:head>

<div class="relative z-[1]" id="top">
	<!-- Status bar -->
	<header
		class="sticky top-0 z-50 border-b border-ink/15 bg-page/85 backdrop-blur-md"
	>
		<div
			class="mx-auto flex h-14 max-w-[1180px] items-center gap-4 px-4 font-mono text-[12px] tracking-[0.04em] max-md:h-12 max-md:gap-3"
		>
			<a href="#top" class="flex items-center gap-2.5 text-ink" aria-label="Mocktail, back to top">
				<span class="block size-6 text-accent">{@render mark()}</span>
				<span class="font-semibold tracking-[0.02em]">mocktail</span>
			</a>
			<span class="text-ink/30 max-md:hidden">·</span>
			<span class="text-muted max-md:hidden">v{__APP_VERSION__} · self-hostable</span>
			<nav
				class="ml-auto flex items-center gap-6 text-muted max-lg:gap-4 max-md:gap-3"
				aria-label="Main"
			>
				<a class="transition-colors hover:text-ink max-md:hidden" href="#capabilities">capabilities</a>
				<a class="transition-colors hover:text-ink max-md:hidden" href="#log">log</a>
				<a class="transition-colors hover:text-ink" href="#deploy">deploy</a>
				<a
					class="inline-flex items-center gap-1.5 bg-ink px-3 py-1.5 font-semibold text-tile transition-colors hover:bg-accent hover:text-ink"
					href="https://github.com/mnabila/mocktail">GitHub ↗</a
				>
			</nav>
		</div>
	</header>

	<main class="mx-auto max-w-[1180px] border-x border-ink/15">
		<!-- Hero -->
		<section
			class="grid grid-cols-[1.05fr_minmax(0,0.95fr)] max-lg:grid-cols-1"
			aria-labelledby="hero-title"
		>
			<div
				use:reveal
				class="flex flex-col gap-7 border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-[clamp(40px,6vw,88px)] max-lg:border-r-0 lg:border-r"
				style="--d: 0ms"
			>
				<div class="tag flex items-center gap-2 text-muted">
					<span class="inline-block size-1.5 bg-accent"></span>
					API mocking · Rust edition 2024
				</div>
				<h1
					id="hero-title"
					class="text-giant leading-[0.86] font-extrabold tracking-[-0.045em]"
				>
					Mock any API.<br />
					See <span class="text-accent">every byte.</span>
				</h1>
				<p class="max-w-[46ch] text-lead leading-[1.3] text-muted">
					Self-hostable API mocking in one Rust binary. Match requests, template responses, import
					OpenAPI, inject faults, and inspect every call. SQLite or Cloudflare D1.
				</p>
				<div class="flex flex-wrap items-center gap-5">
					<a
						class="inline-flex min-h-11 items-center gap-2 bg-ink px-5 font-mono text-[13px] font-semibold tracking-[0.04em] text-tile transition-[background-color,color,transform] hover:bg-accent hover:text-ink active:scale-[0.98]"
						href="https://github.com/mnabila/mocktail#quickstart"
					>
						$ read the quickstart
					</a>
					<a
						class="inline-flex items-center gap-1.5 font-mono text-[13px] tracking-[0.04em] text-ink underline decoration-ink/30 decoration-1 underline-offset-4 transition-colors hover:decoration-accent"
						href="#capabilities">what's inside ↓</a
					>
				</div>
				<div class="mt-auto flex flex-wrap gap-x-5 gap-y-2 pt-2 font-mono text-[12px] text-muted">
					<span>GPL-3.0</span><span class="text-ink/25">/</span>
					<span>Native</span><span class="text-ink/25">/</span>
					<span>Docker</span><span class="text-ink/25">/</span>
					<span>Cloudflare Workers</span>
				</div>
			</div>

			<!-- Live transcript -->
			<div
				use:reveal
				class="relative overflow-hidden border-b border-ink/15 bg-dark p-[clamp(16px,2vw,24px)] text-dark-ink max-lg:min-h-[420px]"
				style="--d: 120ms"
				aria-hidden="true"
			>
				<div
					class="pointer-events-none absolute inset-x-0 top-0 z-[2] h-16 bg-gradient-to-b from-accent/15 to-transparent [.js_&]:animate-scan"
				></div>
				<div class="relative z-[1] flex h-full flex-col font-mono text-[13px]">
					<div class="flex items-center justify-between pb-4 text-dark-muted">
						<span class="flex items-center gap-1.5">
							<i class="size-2 rounded-full bg-danger/70"></i>
							<i class="size-2 rounded-full bg-accent/70"></i>
							<i class="size-2 rounded-full bg-dark-ink/30"></i>
						</span>
						<span class="tracking-[0.04em]">acme.mocktail.dev / users</span>
						<span class="flex items-center gap-1.5">
							<i class="size-[7px] animate-live rounded-full bg-accent"></i> live
						</span>
					</div>

					<div class="flex flex-col gap-1 border-t border-dark-line pt-4 leading-[1.7]">
						<p style="animation-delay: 220ms" class="[.js_&]:animate-rise">
							<span class="text-accent">→</span>
							<span class="mr-2 bg-dark-ink/10 px-1.5 text-dark-ink">GET</span>/v1/users/42?expand=team
						</p>
						<p style="animation-delay: 360ms" class="text-dark-muted [.js_&]:animate-rise">
							&nbsp;&nbsp;x-mock-env: dev
						</p>
						<p style="animation-delay: 540ms" class="pt-2 text-dark-muted [.js_&]:animate-rise">
							<span class="text-accent">·</span> matched 3 rules · 12 ms
						</p>
						<p style="animation-delay: 680ms" class="[.js_&]:animate-rise">
							<span class="text-accent">←</span> 200 OK
						</p>
					</div>

					<pre
						style="animation-delay: 820ms"
						class="mt-3 min-h-0 flex-1 overflow-hidden border border-dark-line bg-dark-2 px-4 py-3.5 leading-[1.6] [.js_&]:animate-rise"><code
							>&#123;
  <span class="text-dark-muted">"id"</span>: <span class="text-accent">"42"</span>,
  <span class="text-dark-muted">"name"</span>: <span class="text-accent">"Ada Lovelace"</span>,
  <span class="text-dark-muted">"team"</span>: <span class="text-accent">"analytics"</span>,
  <span class="text-dark-muted">"plan"</span>: <span class="text-accent">"pro"</span>
&#125;<span class="animate-caret text-accent">▊</span></code
						></pre>
				</div>
			</div>
		</section>

		<!-- Ticker -->
		<div class="flex overflow-hidden border-b border-ink/15 bg-ink py-2.5 text-tile select-none">
			<div class="flex shrink-0 animate-marquee items-center font-mono text-[12px] tracking-[0.18em]">
				{#each [...ticker, ...ticker] as token}
					<span class="flex items-center">
						<span class="px-6">{token}</span>
						<span class="text-accent">◆</span>
					</span>
				{/each}
			</div>
		</div>

		<!-- 01 Why -->
		<section
			use:reveal
			class="grid grid-cols-[auto_minmax(0,1fr)] gap-[clamp(24px,4vw,64px)] border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-[clamp(40px,5vw,80px)] max-md:grid-cols-1 max-md:gap-6"
			id="why"
			aria-labelledby="why-title"
		>
			<div class="max-md:mb-2">{@render sectionTag('01', 'Why Mocktail')}</div>
			<div class="max-w-[62ch]">
				<h2 id="why-title" class="mb-8 text-head font-extrabold">
					Between static files and<br class="max-md:hidden" />
					<span class="text-accent">rented cloud mocks.</span>
				</h2>
				<div class="space-y-5 text-body leading-[1.6] text-muted">
					<p>
						Mocking is usually a trade-off: static fixtures too dumb for real testing, or managed
						services that add latency, privacy concerns, and a monthly bill.
					</p>
					<p>
						Mocktail fills the gap. A mocking tool should be as portable as a single binary, as
						private as a local SQLite file, and as capable as a hand-written server.
					</p>
					<p class="text-ink">
						No YAML wrangling, no guessing why a request didn't match. A fast, self-hostable engine
						with total control over behavior and full visibility into every byte.
					</p>
				</div>
			</div>
		</section>

		<!-- 02 Capabilities -->
		<section id="capabilities" aria-labelledby="cap-title">
			<div
				use:reveal
				class="flex flex-wrap items-end justify-between gap-5 border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-[clamp(28px,4vw,52px)]"
			>
				<div class="flex flex-col gap-5">
					{@render sectionTag('02', 'Capabilities')}
					<h2 id="cap-title" class="max-w-[16ch] text-head font-extrabold">
						Everything to simulate a production API.
					</h2>
				</div>
				<p class="max-w-[34ch] font-mono text-[13px] leading-[1.6] text-muted">
					One server, no sidecars. Matching, templating, import, faults and logs — all built in.
				</p>
			</div>

			<!-- Three specimen panels -->
			<div class="grid grid-cols-3 border-b border-ink/15 max-md:grid-cols-1">
				<!-- matching -->
				<article
					use:reveal
					class="flex flex-col gap-4 border-ink/15 p-[clamp(18px,2vw,28px)] max-md:border-b md:border-r"
					style="--d: 0ms"
				>
					<p class="tag text-muted">specimen · match</p>
					<div class="flex items-center gap-2.5 font-mono text-[14px]">
						<span class="bg-ink px-1.5 py-0.5 text-[12px] font-semibold text-tile">GET</span>
						<span class="font-sans font-semibold">/v1/users/:id</span>
					</div>
					<dl class="flex flex-col font-mono text-[12.5px]">
						{#each [['query', 'expand = team'], ['header', 'x-env = dev'], ['body', '{ "v": "1" }']] as [k, v]}
							<div class="flex items-baseline gap-4 border-t border-ink/10 py-2">
								<dt class="w-14 shrink-0 text-muted">{k}</dt>
								<dd class="font-semibold">{v}</dd>
							</div>
						{/each}
					</dl>
					<h3 class="mt-auto pt-2 font-display text-[17px] font-bold">Request matching</h3>
				</article>

				<!-- templates -->
				<article
					use:reveal
					class="flex flex-col gap-4 border-ink/15 p-[clamp(18px,2vw,28px)] max-md:border-b md:border-r"
					style="--d: 90ms"
				>
					<p class="tag text-muted">specimen · response.json</p>
					<pre
						class="flex-1 font-mono text-[13px] leading-[1.7] whitespace-pre"><code>&#123;
  "id": "<mark class="bg-accent px-0.5 text-ink">&#123;&#123;path.id&#125;&#125;</mark>",
  "team": "<mark class="bg-accent px-0.5 text-ink">&#123;&#123;query.team&#125;&#125;</mark>",
  "seen": "<mark class="bg-accent px-0.5 text-ink">&#123;&#123;now&#125;&#125;</mark>"
&#125;</code></pre>
					<h3 class="mt-auto pt-2 font-display text-[17px] font-bold">Dynamic templates</h3>
				</article>

				<!-- faults -->
				<article
					use:reveal
					class="flex flex-col gap-4 p-[clamp(18px,2vw,28px)]"
					style="--d: 180ms"
				>
					<p class="tag text-muted">specimen · fault</p>
					<div class="flex flex-col gap-3 font-mono text-[12.5px]">
						<div class="flex items-center justify-between">
							<span class="text-muted">delay</span><strong class="text-[14px]">800 ms</strong>
						</div>
						<div class="h-1.5 overflow-hidden bg-ink/10">
							<span class="block h-full bg-accent" style="width: 82%"></span>
						</div>
						<div class="flex items-center justify-between border-t border-ink/10 pt-3">
							<span class="text-muted">fail 1 in 5</span>
							<span class="relative block h-5 w-9 bg-ink">
								<i class="absolute top-[3px] right-[3px] size-3.5 bg-accent"></i>
							</span>
						</div>
						<div class="flex items-center justify-between">
							<span class="text-muted">status</span><strong class="text-danger text-[14px]">503</strong>
						</div>
					</div>
					<h3 class="mt-auto pt-2 font-display text-[17px] font-bold">Faults on demand</h3>
				</article>
			</div>

			<!-- Datasheet table -->
			<div use:reveal class="font-mono">
				<div
					class="hidden grid-cols-[120px_minmax(0,1fr)_minmax(0,1.2fr)] gap-6 border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-3 text-[11px] tracking-[0.18em] text-muted uppercase md:grid"
				>
					<span>id</span><span>capability</span><span>spec</span>
				</div>
				{#each capabilities as cap}
					<div
						class="group grid grid-cols-[120px_minmax(0,1fr)_minmax(0,1.2fr)] items-center gap-6 border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-4 transition-colors hover:bg-surface max-md:grid-cols-[1fr] max-md:gap-1"
					>
						<span class="text-[12px] text-accent">{cap.id}</span>
						<span class="font-sans text-[16px] font-semibold tracking-[-0.01em]">{cap.name}</span>
						<span class="text-[13px] text-muted">{cap.spec}</span>
					</div>
				{/each}
			</div>
		</section>

		<!-- 03 Request log -->
		<section class="bg-dark text-dark-ink" id="log" aria-labelledby="log-title">
			<div
				use:reveal
				class="flex flex-col gap-5 border-b border-dark-line px-[clamp(20px,4vw,52px)] py-[clamp(32px,4vw,64px)]"
			>
				<div class="flex items-center gap-3">
					<span class="tag text-accent">03</span>
					<span class="h-px w-6 bg-dark-ink/25"></span>
					<span class="tag text-dark-muted">Request log</span>
				</div>
				<h2 id="log-title" class="max-w-[16ch] text-head font-extrabold text-dark-ink">
					Every request, accounted for.
				</h2>
				<p class="max-w-[58ch] text-body text-dark-muted">
					Matched, unmatched, delayed or failed. Open a row to see the exact request and the
					response Mocktail returned.
				</p>
			</div>

			<div class="pb-6">
				{#each logRows as row, i (row.path)}
					<LogEntry {...row} open={openRow === i} onToggle={() => (openRow = i)} />
				{/each}
			</div>

			<div
				class="flex justify-between border-t border-dark-line px-[clamp(20px,4vw,52px)] py-4 font-mono text-[12px] text-dark-muted"
			>
				<span>8 requests</span><span>1 unmatched</span>
			</div>
		</section>

		<!-- 04 Deploy -->
		<section aria-labelledby="deploy-title" id="deploy">
			<div
				use:reveal
				class="flex flex-col gap-5 border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-[clamp(32px,4vw,56px)]"
			>
				{@render sectionTag('04', 'Deploy')}
				<h2 id="deploy-title" class="max-w-[18ch] text-head font-extrabold">
					Your own metal, or the edge.
				</h2>
			</div>

			<div class="grid grid-cols-2 border-b border-ink/15 max-md:grid-cols-1">
				{#each [{ kind: 'Native / Docker', accent: false, steps: [['git clone', 'Grab the source from GitHub.'], ['make dev', 'Compile and start the server.'], ['localhost:8787', 'Open the dashboard, start mocking.']] }, { kind: 'Cloudflare Workers', accent: true, steps: [['wrangler login', 'Authenticate with Cloudflare.'], ['d1 migrations apply', 'Create the D1-backed store.'], ['wrangler deploy', 'Go live globally on the edge.']] }] as col}
					<div
						use:reveal
						class="flex flex-col gap-7 border-ink/15 p-[clamp(22px,3vw,44px)] max-md:border-b md:[&:first-child]:border-r"
						style="--d: 80ms"
					>
						<p class="tag text-muted">{col.kind}</p>
						<ol class="flex flex-col gap-6">
							{#each col.steps as [cmd, desc], si}
								<li class="flex gap-4">
									<span
										class="flex size-7 flex-none items-center justify-center font-mono text-[12px] font-bold {col.accent
											? 'bg-accent text-ink'
											: 'bg-ink text-tile'}">{si + 1}</span
									>
									<div class="flex flex-col gap-1">
										<code class="w-fit bg-surface px-1.5 py-0.5 text-[13px]">{cmd}</code>
										<p class="text-[14px] text-muted">{desc}</p>
									</div>
								</li>
							{/each}
						</ol>
					</div>
				{/each}
			</div>
		</section>

		<!-- 05 Use cases -->
		<section aria-labelledby="uc-title">
			<div use:reveal class="border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-[clamp(28px,4vw,48px)]">
				{@render sectionTag('05', 'Use cases')}
				<h2 id="uc-title" class="sr-only">Use cases</h2>
			</div>
			<div class="grid grid-cols-3 border-b border-ink/15 max-md:grid-cols-1">
				{#each useCases as uc, i}
					<article
						use:reveal
						class="flex flex-col gap-4 border-ink/15 p-[clamp(20px,2.5vw,36px)] max-md:border-b {i <
						2
							? 'md:border-r'
							: ''}"
						style="--d: {i * 90}ms"
					>
						<div class="flex items-center justify-between">
							<span class="flex size-10 items-center justify-center bg-ink text-tile">
								<svg
									viewBox="0 0 24 24"
									class="size-5"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"><path d={uc.icon} /></svg
								>
							</span>
							<span class="tag text-accent">{uc.id}</span>
						</div>
						<h3 class="font-display text-[20px] font-bold tracking-[-0.02em]">{uc.title}</h3>
						<p class="text-[15px] leading-[1.55] text-muted">{uc.body}</p>
					</article>
				{/each}
			</div>
		</section>

		<!-- CTA -->
		<section
			use:reveal
			class="grid grid-cols-[1.4fr_minmax(0,1fr)] border-b border-ink/15 max-md:grid-cols-1"
			aria-labelledby="cta-title"
		>
			<div
				class="flex flex-col items-start justify-center gap-6 bg-accent p-[clamp(28px,4vw,64px)] text-ink max-md:border-b max-md:border-ink/15 md:border-r md:border-ink/15"
			>
				<div class="tag flex items-center gap-2">
					<span class="inline-block size-1.5 bg-ink"></span> Ready when you are
				</div>
				<h2 id="cta-title" class="max-w-[14ch] text-head font-extrabold">
					Run it on your own metal.
				</h2>
				<p class="max-w-[48ch] text-body leading-[1.5]">
					No account, no hosted service. Clone the repo, run one command, and point your client at
					localhost.
				</p>
				<div class="flex flex-wrap items-center gap-5">
					<a
						class="inline-flex min-h-11 items-center gap-2 bg-ink px-5 font-mono text-[13px] font-semibold tracking-[0.04em] text-tile transition-[background-color,color,transform] hover:bg-tile hover:text-ink active:scale-[0.98]"
						href="https://github.com/mnabila/mocktail#quickstart">$ read the quickstart</a
					>
					<a
						class="inline-flex items-center gap-1.5 font-mono text-[13px] tracking-[0.04em] underline decoration-ink/40 decoration-1 underline-offset-4 transition-opacity hover:opacity-70"
						href="https://github.com/mnabila/mocktail">star on GitHub ↗</a
					>
				</div>
			</div>
			<div class="flex flex-col gap-7 bg-dark p-[clamp(22px,3vw,40px)] text-dark-ink">
				<div class="flex items-center justify-between">
					<span class="block size-12 text-accent">{@render mark()}</span>
					<span class="flex items-center gap-1.5 font-mono text-[12px] text-dark-muted">
						<i class="size-[7px] animate-live rounded-full bg-accent"></i> healthy
					</span>
				</div>
				<pre
					class="flex-1 font-mono text-[13px] leading-[1.7] text-dark-ink"><code
						>$ make dev
<span class="text-accent">mocktail</span>  listening on :8787
<span class="text-dark-muted">sqlite</span>    mocktail.db
<span class="text-dark-muted">assets</span>    frontend/build

$ npx wrangler deploy
<span class="text-accent">worker</span>    acme.workers.dev
<span class="text-dark-muted">d1</span>        mocktail
<span class="text-dark-muted">assets</span>    42 files uploaded

<span class="text-dark-muted"># point your client at the host</span>
$ curl acme.mocktail.dev/users/42
<span class="text-accent">200</span> <span class="text-dark-muted">·</span> 12 ms <span class="text-dark-muted">·</span> matched 3 rules<span
							class="animate-caret text-accent">▊</span
						></code
					></pre>
			</div>
		</section>

		<!-- Credits -->
		<section id="credits" aria-labelledby="credits-title">
			<div use:reveal class="border-b border-ink/15 px-[clamp(20px,4vw,52px)] py-[clamp(28px,4vw,48px)]">
				{@render sectionTag('06', 'Inspired by')}
				<h2 id="credits-title" class="sr-only">Inspired by</h2>
				<p class="mt-5 max-w-[58ch] text-body text-muted">
					Mocktail stands on the tools that shaped how developers mock and ship APIs.
				</p>
			</div>
			<div class="grid grid-cols-3 border-b border-ink/15 max-md:grid-cols-1">
				{#each [{ tag: 'Palette', name: 'Cloudflare', body: 'The accent orange nods to Cloudflare, the edge Workers and D1 deploy to.', href: 'https://www.cloudflare.com' }, { tag: 'Portal UX', name: 'Postman', body: 'The portal, especially the collection sidebar, follows patterns Postman made familiar.', href: 'https://www.postman.com' }, { tag: 'Prior art', name: 'WireMock', body: 'A long-time reference for self-hosted API mocking. Mocktail is an independent take, not built on it.', href: 'https://wiremock.org' }] as c, i}
					<a
						use:reveal
						href={c.href}
						class="flex flex-col gap-3 border-ink/15 p-[clamp(20px,2.5vw,36px)] transition-colors hover:bg-surface max-md:border-b {i <
						2
							? 'md:border-r'
							: ''}"
						style="--d: {i * 80}ms"
					>
						<span class="tag text-muted">{c.tag}</span>
						<h3 class="font-display text-[22px] font-bold tracking-[-0.02em]">{c.name}</h3>
						<p class="text-[14px] leading-[1.55] text-muted">{c.body}</p>
					</a>
				{/each}
			</div>
		</section>

		<!-- Footer -->
		<footer class="bg-dark text-dark-ink">
			<div class="flex flex-col gap-12 p-[clamp(28px,4vw,64px)] max-md:gap-9">
				<div class="flex items-center gap-3">
					<span class="block size-9 text-accent">{@render mark()}</span>
					<span class="font-display text-2xl font-bold tracking-[-0.03em]">Mocktail</span>
				</div>

				<div class="grid grid-cols-4 gap-8 max-md:grid-cols-2 max-md:gap-x-6 max-md:gap-y-8">
					{#each [{ h: 'Product', links: [['Capabilities', '#capabilities'], ['Request log', '#log'], ['Deploy', '#deploy'], ['Credits', '#credits']] }, { h: 'Docs', links: [['Quickstart', 'https://github.com/mnabila/mocktail#quickstart'], ['Configuration', 'https://github.com/mnabila/mocktail#configuration'], ['OpenAPI', 'https://github.com/mnabila/mocktail#features']] }, { h: 'Source', links: [['GitHub', 'https://github.com/mnabila/mocktail'], ['License', 'https://github.com/mnabila/mocktail/blob/main/LICENSE'], ['Issues', 'https://github.com/mnabila/mocktail/issues']] }] as col}
						<div class="flex flex-col items-start gap-2.5">
							<h3 class="tag mb-1 text-dark-muted">{col.h}</h3>
							{#each col.links as [label, href]}
								<a
									class="font-mono text-[13px] text-dark-ink/80 transition-colors hover:text-accent"
									{href}>{label}</a
								>
							{/each}
						</div>
					{/each}
					<div class="flex flex-col items-start gap-2.5">
						<h3 class="tag mb-1 text-dark-muted">Built with</h3>
						<p class="font-mono text-[13px] leading-[1.6] text-dark-ink/80">
							Rust · SQLite · SvelteKit · Cloudflare Workers
						</p>
					</div>
				</div>

				<div
					class="flex items-center justify-between gap-5 border-t border-dark-line pt-8 font-mono text-[12px] text-dark-muted"
				>
					<p>© 2026 Mocktail · GPL-3.0</p>
					<a class="whitespace-nowrap text-dark-ink underline underline-offset-4" href="#top"
						>back to top ↑</a
					>
				</div>
			</div>
		</footer>
	</main>
</div>
