<script lang="ts">
	import { reveal } from '#lib/actions/reveal';
	import UseCaseTile from '#lib/components/UseCaseTile.svelte';
	import FeatureTile from '#lib/components/FeatureTile.svelte';
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
			response:
				'{\n  "id": "42",\n  "name": "Ada Lovelace",\n  "team": "analytics",\n  "plan": "pro"\n}'
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
			method: 'GET',
			path: '/v1/users/99',
			note: 'no match',
			ms: '4 ms',
			status: 404,
			match: ['path /v1/users/:id', 'template guarded'],
			request: 'GET /v1/users/99',
			response: '{\n  "error": "user not found",\n  "id": "99"\n}'
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

<div class="overflow-hidden p-2 max-md:p-1.5" id="top">
	<header
		class="mb-2 grid h-[60px] grid-cols-[60px_minmax(0,1fr)_auto] gap-2 max-md:mb-1.5 max-md:h-[52px] max-md:grid-cols-[52px_minmax(0,1fr)_auto] max-md:gap-1.5"
	>
		<a
			class="flex items-center justify-center bg-dark text-dark-ink"
			href="#top"
			aria-label="Mocktail, back to top"
		>
			<span class="block size-9 max-md:size-8">{@render mark()}</span>
		</a>
		<nav
			class="flex items-center gap-[clamp(18px,3vw,40px)] bg-surface px-[22px] font-display text-sm font-medium text-muted max-lg:gap-[18px] max-lg:px-4 max-md:gap-4 max-md:px-3"
			aria-label="Main"
		>
			<a class="transition-colors hover:text-ink" href="#capabilities">Capabilities</a>
			<a class="transition-colors hover:text-ink" href="#requests">Requests</a>
			<a class="transition-colors hover:text-ink max-md:hidden" href="#deploy">Deploy</a>
			<a
				class="transition-colors hover:text-ink max-md:hidden"
				href="https://github.com/mnabila/mocktail#readme">Docs</a
			>
		</nav>
		<a
			class="flex items-center justify-center gap-2 bg-dark px-[22px] font-display text-sm font-semibold text-dark-ink transition-colors hover:bg-accent hover:text-ink max-lg:px-4 max-md:px-3.5"
			href="https://github.com/mnabila/mocktail"
		>
			GitHub <span aria-hidden="true">↗</span>
		</a>
	</header>

	<main>
		<section
			class="grid min-h-[calc(100svh-84px)] grid-cols-2 grid-rows-[minmax(0,1fr)_minmax(0,1fr)] gap-2 max-md:min-h-0 max-md:grid-cols-1 max-md:grid-rows-[auto] max-md:gap-1.5"
			aria-labelledby="hero-title"
		>
			<div
				use:reveal
				class="flex flex-col items-start gap-[clamp(16px,2vw,24px)] bg-tile p-[clamp(20px,2.4vw,32px)] max-md:min-h-[300px] max-md:p-[22px]"
				style="--d: 0ms"
			>
				<h1
					id="hero-title"
					class="text-[clamp(64px,9.5vw,150px)] leading-[0.84] tracking-[-0.055em] max-md:text-[clamp(56px,18vw,96px)]"
				>
					Mocktail
				</h1>
				<p class="max-w-[32ch] text-lead leading-[1.25] max-md:max-w-[28ch] max-md:text-lg">
					Self-hostable API mocking in Rust. One binary, SQLite or D1, and every request visible.
				</p>
				<div class="mt-auto flex flex-wrap items-center gap-6 max-md:mt-3 max-md:gap-4">
					<a
						class="inline-flex min-h-11 items-center justify-center bg-ink px-[18px] font-display text-sm font-semibold text-tile transition-[background-color,color,transform] hover:bg-accent hover:text-ink active:scale-[0.98]"
						href="https://github.com/mnabila/mocktail#quickstart"
					>
						Read the quickstart
					</a>
					<a
						class="inline-flex items-center gap-1.5 font-display text-sm font-medium underline decoration-1 underline-offset-4 transition-colors hover:text-muted"
						href="#capabilities">See what's inside <span aria-hidden="true">↓</span></a
					>
				</div>
				<p class="font-display text-sm text-muted">
					GPL-3.0 · Native · Docker · Cloudflare Workers
				</p>
			</div>

			<div
				use:reveal
				class="overflow-hidden bg-surface p-[clamp(14px,1.6vw,20px)] max-md:h-[320px] max-md:p-3 [.js_&]:animate-shutter"
				style="--d: 90ms"
				aria-hidden="true"
			>
				<div
					class="flex h-full w-full flex-col gap-3.5 overflow-hidden border border-[#e3e3de] bg-tile p-[clamp(14px,1.4vw,20px)] font-display [.js_&]:animate-settle"
					data-ui
				>
					<div class="flex items-center justify-between gap-3 text-[13px] font-medium text-muted">
						<span>acme / users</span>
						<span class="inline-flex items-center gap-1.5 text-[13px] font-medium"
							><i class="size-[7px] animate-live rounded-full bg-accent"></i> live</span
						>
					</div>
					<div class="flex min-w-0 items-center gap-3">
						<span
							class="inline-flex h-7 min-w-12 items-center justify-center bg-ink px-2 text-[13px] font-semibold tracking-[0.02em] text-tile"
							>GET</span
						>
						<span class="truncate text-sm font-medium">/v1/users/42?expand=team</span>
						<span
							class="ml-auto inline-flex h-7 min-w-12 items-center justify-center bg-accent px-2 text-[13px] font-semibold text-ink"
							>200</span
						>
					</div>
					<div class="text-sm text-muted">users.get · matched 3 rules · 12 ms</div>
					<pre
						class="min-h-0 flex-1 overflow-hidden bg-surface px-4 py-3.5 font-mono text-[13px] leading-[1.5]"><code
							>&#123;
  "id": "42",
  "name": "Ada Lovelace",
  "team": "analytics",
  "plan": "pro"
&#125;</code
						></pre>
					<div class="flex justify-between gap-3 text-[13px] text-muted">
						<span>x-mock-env: dev</span>
						<span>query.expand = team</span>
					</div>
				</div>
			</div>

			<div
				use:reveal
				class="overflow-hidden bg-dark p-[clamp(14px,1.6vw,20px)] text-dark-ink max-md:h-[320px] max-md:p-3"
				style="--d: 180ms"
				aria-hidden="true"
			>
				<div
					class="flex h-full w-full flex-col overflow-hidden bg-dark-2 p-[clamp(14px,1.4vw,20px)] font-display"
					data-ui
				>
					<div
						class="flex items-center justify-between gap-3 pb-3 text-[13px] font-medium text-dark-muted"
					>
						<span>Request log</span>
						<span class="inline-flex items-center gap-1.5 text-[13px] font-medium"
							><i class="size-[7px] animate-live rounded-full bg-accent"></i> live</span
						>
					</div>
					<div class="flex min-h-[46px] items-center gap-3 border-b border-dark-line text-sm">
						<span
							class="inline-flex h-6 min-w-11 items-center justify-center bg-[#3a3a36] px-2 text-[13px] font-semibold tracking-[0.02em] text-dark-ink"
							>GET</span
						>
						<span class="truncate text-sm font-medium">/v1/users/42</span>
						<span
							class="ml-auto inline-flex h-6 min-w-[42px] items-center justify-center bg-accent px-2 text-[13px] font-semibold text-ink"
							>200</span
						>
					</div>
					<div class="flex min-h-[46px] items-center gap-3 border-b border-dark-line text-sm">
						<span
							class="inline-flex h-6 min-w-11 items-center justify-center bg-[#3a3a36] px-2 text-[13px] font-semibold tracking-[0.02em] text-dark-ink"
							>POST</span
						>
						<span class="truncate text-sm font-medium">/v1/orders</span>
						<span
							class="ml-auto inline-flex h-6 min-w-[42px] items-center justify-center bg-accent px-2 text-[13px] font-semibold text-ink"
							>201</span
						>
					</div>
					<div class="flex min-h-[46px] items-center gap-3 border-b border-dark-line text-sm">
						<span
							class="inline-flex h-6 min-w-11 items-center justify-center bg-[#3a3a36] px-2 text-[13px] font-semibold tracking-[0.02em] text-dark-ink"
							>GET</span
						>
						<span class="truncate text-sm font-medium">/v1/admin</span>
						<span
							class="ml-auto inline-flex h-6 min-w-[42px] items-center justify-center bg-[#46463f] px-2 text-[13px] font-semibold text-dark-ink"
							>401</span
						>
					</div>
					<div class="flex min-h-[46px] items-center gap-3 border-b border-dark-line text-sm">
						<span
							class="inline-flex h-6 min-w-11 items-center justify-center bg-[#3a3a36] px-2 text-[13px] font-semibold tracking-[0.02em] text-dark-ink"
							>PATCH</span
						>
						<span class="truncate text-sm font-medium">/v1/orders/1042</span>
						<span
							class="ml-auto inline-flex h-6 min-w-[42px] items-center justify-center bg-accent px-2 text-[13px] font-semibold text-ink"
							>200</span
						>
					</div>
					<div class="mt-auto flex justify-between pt-3.5 text-[13px] text-dark-muted">
						<span>8 requests</span>
						<span>1 unmatched</span>
					</div>
				</div>
			</div>

			<div class="grid grid-cols-2 gap-2 max-md:gap-1.5">
				<a
					use:reveal
					class="flex flex-col justify-between gap-6 bg-dark p-[clamp(18px,2vw,26px)] text-dark-ink transition-colors hover:bg-ink hover:text-dark-ink max-md:min-h-[140px] max-md:justify-center max-md:p-[18px]"
					href="#deploy"
					style="--d: 270ms"
				>
					<p
						class="max-w-[16ch] font-display text-lead leading-[1.12] font-semibold tracking-[-0.025em] max-md:max-w-[14ch] max-md:after:content-['_↓']"
					>
						Run it on your own metal, or at the edge.
					</p>
					<span class="font-display text-sm font-medium text-dark-muted max-md:hidden"
						>Deploy <span aria-hidden="true">↓</span></span
					>
				</a>
				<a
					use:reveal
					class="flex flex-col justify-between gap-6 bg-accent p-[clamp(18px,2vw,26px)] text-ink transition-colors hover:bg-tile hover:text-ink max-md:min-h-[140px] max-md:justify-center max-md:p-[18px]"
					href="#capabilities"
					style="--d: 360ms"
				>
					<p
						class="max-w-[16ch] font-display text-lead leading-[1.12] font-semibold tracking-[-0.025em] max-md:max-w-[14ch] max-md:after:content-['_↓']"
					>
						OpenAPI in. Mocks out.
					</p>
					<span class="font-display text-sm font-medium text-ink max-md:hidden"
						>OpenAPI <span aria-hidden="true">↓</span></span
					>
				</a>
			</div>
		</section>

		<section
			use:reveal
			class="mt-2 grid grid-cols-12 gap-2 max-md:mt-1.5 max-md:gap-1.5"
			aria-labelledby="why-title"
		>
			<div
				class="col-span-8 bg-dark p-[clamp(32px,4vw,64px)] text-dark-ink max-lg:col-span-12 max-md:p-6"
				style="--d: 0ms"
			>
				<h2 id="why-title" class="mb-6 text-head">Why Mocktail?</h2>
				<div class="max-w-[60ch] space-y-6 text-body leading-relaxed opacity-90">
					<p>
						API mocking is usually a trade-off between two extremes:
						<strong class="text-accent">static files</strong> that are too simple for real-world
						testing, and <strong class="text-accent">managed cloud services</strong> that introduce latency,
						privacy concerns, and monthly subscriptions.
					</p>
					<p>
						Mocktail exists to fill that gap. We believe a developer's mocking tool should be as
						portable as a single binary, as private as a local SQLite file, and as powerful as a
						custom-coded server.
					</p>
					<p>
						No more fighting with complex configuration YAMLs or wondering why a request didn't
						match. Just a fast, self-hostable engine that gives you total control over your API's
						behavior and full visibility into every byte that hits your endpoint.
					</p>
				</div>
			</div>
			<div class="col-span-4 grid grid-cols-1 gap-2 max-lg:col-span-12 max-md:col-span-12">
				<FeatureTile
					title="Local-First"
					description="Your mocks, logs, and data stay on your hardware. Zero vendor lock-in."
					delay="100ms"
				/>
				<FeatureTile
					title="Zero Friction"
					description="One Rust binary. No runtime to install, no heavy dependencies to manage."
					bgColor="bg-tile"
					delay="200ms"
				/>
			</div>
		</section>

		<section
			use:reveal
			class="mt-2 grid grid-cols-1 gap-2 max-md:mt-1.5 max-md:gap-1.5 md:grid-cols-3"
			aria-labelledby="use-cases-title"
		>
			<UseCaseTile
				title="Frontend Parallelism"
				icon="zap"
				description="Stop waiting for the backend team. Import your OpenAPI spec and start building your UI against realistic, dynamic mocks from day one."
			/>
			<UseCaseTile
				title="Resilience Testing"
				icon="shield"
				description="Inject faults on purpose. Simulate 503 errors, 800ms network lag, or intermittent timeouts to ensure your app handles failure gracefully."
			/>
			<UseCaseTile
				title="Deterministic CI/CD"
				icon="cycle"
				description="Replace flaky external staging APIs with a local Mocktail instance in your pipeline. Get consistent, fast, and private test results every time."
			/>
		</section>

		<section
			use:reveal
			class="mt-2 grid grid-cols-12 gap-2 max-md:mt-1.5 max-md:gap-1.5"
			aria-labelledby="features-title"
		>
			<div class="col-span-4 bg-surface p-6 max-lg:col-span-12 max-md:p-5">
				<h2 id="features-title" class="mb-4 text-head">Core Capabilities</h2>
				<p class="mb-6 text-body text-muted">
					Everything you need to simulate a production API, from spec import to deep observability.
				</p>
				<div class="flex flex-col gap-3">
					<div class="flex items-start gap-3 text-sm">
						<span class="mt-1 size-1.5 flex-none rounded-full bg-accent"></span>
						<span
							><strong>OpenAPI 2.0 / 3.x:</strong> Import Swagger or OpenAPI specifications to bootstrap
							your entire mock server instantly.</span
						>
					</div>
					<div class="flex items-start gap-3 text-sm">
						<span class="mt-1 size-1.5 flex-none rounded-full bg-accent"></span>
						<span
							><strong>Workspaces & RBAC:</strong> Multi-tenant host-routed environments with granular
							roles (Owner, Editor, Viewer) and SSO integration.</span
						>
					</div>
					<div class="flex items-start gap-3 text-sm">
						<span class="mt-1 size-1.5 flex-none rounded-full bg-accent"></span>
						<span
							><strong>Advanced Matching:</strong> Precision targeting based on HTTP method, path patterns,
							query strings, headers, and request body content.</span
						>
					</div>
					<div class="flex items-start gap-3 text-sm">
						<span class="mt-1 size-1.5 flex-none rounded-full bg-accent"></span>
						<span
							><strong>Dynamic Templates:</strong> Go beyond static JSON. Use request data to generate
							realistic, variable responses.</span
						>
					</div>
					<div class="flex items-start gap-3 text-sm">
						<span class="mt-1 size-1.5 flex-none rounded-full bg-accent"></span>
						<span
							><strong>Fault Injection:</strong> Test the untestable. Simulate network lag, timeouts,
							and specific HTTP error codes on demand.</span
						>
					</div>
					<div class="flex items-start gap-3 text-sm">
						<span class="mt-1 size-1.5 flex-none rounded-full bg-accent"></span>
						<span
							><strong>Request Logs:</strong> Complete observability. Inspect every incoming request and
							the exact response Mocktail returned.</span
						>
					</div>
				</div>
			</div>
			<div class="col-span-8 grid grid-cols-2 gap-2 max-lg:col-span-12 max-md:col-span-12">
				<FeatureTile
					title="SQLite / D1"
					description="Fast, local storage that's easy to backup, version, and migrate."
					bgColor="bg-tile"
					delay="100ms"
				/>
				<FeatureTile
					title="Single Binary"
					description="No JVM, no Node.js, no Docker required. Just one native Rust binary."
					delay="200ms"
				/>
				<FeatureTile
					title="Edge Ready"
					description="Deploy to Cloudflare Workers for global, low-latency mocking."
					delay="300ms"
				/>
				<FeatureTile
					title="GPL-3.0"
					description="Open source and free for developers who value transparency."
					bgColor="bg-tile"
					delay="400ms"
				/>
			</div>
		</section>

		<section
			use:reveal
			class="mt-2 grid auto-rows-[104px] grid-cols-12 gap-2 max-md:mt-1.5 max-md:auto-rows-[28vw] max-md:grid-cols-2 max-md:gap-1.5"
			id="capabilities"
			aria-labelledby="capabilities-title"
		>
			<div
				class="col-span-6 row-span-2 flex flex-col justify-between gap-6 bg-surface p-6 max-lg:col-span-12 max-md:col-span-2 max-md:p-5"
				style="--d: 0ms"
			>
				<h2 id="capabilities-title" class="max-w-[12ch] text-head">What's in the box</h2>
				<p class="max-w-[44ch] text-body">
					One server, no sidecars. Matching, templating, import, faults and logs are all built in.
				</p>
			</div>

			<article
				use:reveal
				class="col-span-6 row-span-2 flex flex-col bg-tile max-lg:col-span-12 max-lg:row-span-3 max-md:col-span-2 max-md:row-span-3"
				style="--d: 80ms"
			>
				<div
					class="min-h-0 flex-1 overflow-hidden px-5 pt-4 font-display max-lg:px-4 max-lg:pt-3.5 max-md:px-3 max-md:pt-3"
					data-ui
					aria-hidden="true"
				>
					<div
						class="flex justify-between gap-3 pb-2.5 text-[13px] font-medium text-muted max-md:pb-1.5"
					>
						<span>Match</span><span>all rules</span>
					</div>
					<div class="mb-4 flex min-w-0 items-center gap-3">
						<span
							class="inline-flex h-[34px] min-w-14 items-center justify-center bg-ink px-2 text-[13px] font-semibold tracking-[0.02em] text-tile"
							>GET</span
						>
						<span class="text-lead font-semibold tracking-[-0.025em] whitespace-nowrap"
							>/v1/users/:id</span
						>
					</div>
					<div
						class="flex items-baseline gap-4 border-t border-line py-2.5 max-md:gap-2 max-md:py-1.5"
					>
						<span class="min-w-[46px] text-[13px] text-muted max-md:min-w-9">query</span>
						<strong class="text-body font-medium whitespace-nowrap">expand = team</strong>
					</div>
					<div
						class="flex items-baseline gap-4 border-t border-line py-2.5 max-md:gap-2 max-md:py-1.5"
					>
						<span class="min-w-[46px] text-[13px] text-muted max-md:min-w-9">header</span>
						<strong class="text-body font-medium whitespace-nowrap">x-env = dev</strong>
					</div>
					<div
						class="flex items-baseline gap-4 border-t border-line py-2.5 max-md:gap-2 max-md:py-1.5"
					>
						<span class="min-w-[46px] text-[13px] text-muted max-md:min-w-9">body</span>
						<strong class="text-body font-medium whitespace-nowrap"
							>&#123; "version": "v1" &#125;</strong
						>
					</div>
				</div>
				<div
					class="relative z-[1] px-5 pt-3.5 pb-[18px] max-lg:px-4 max-lg:pt-3 max-lg:pb-4 max-md:px-3 max-md:pt-3 max-md:pb-3.5"
				>
					<h3 class="text-body leading-[1.1] font-semibold tracking-[-0.02em]">Request matching</h3>
					<p class="mt-[5px] text-sm leading-[1.35]">Method, path, query, headers, body.</p>
				</div>
			</article>

			<article
				use:reveal
				class="col-span-3 row-span-4 flex flex-col bg-surface max-lg:col-span-6 max-lg:row-span-3 max-md:col-span-2 max-md:row-span-3"
				style="--d: 160ms"
			>
				<div
					class="min-h-0 flex-1 overflow-hidden px-5 pt-3.5 font-display max-lg:px-4 max-md:px-3 max-md:pt-3"
					data-ui
					aria-hidden="true"
				>
					<div
						class="flex justify-between gap-3 pb-2.5 text-[13px] font-medium text-muted max-md:pb-1.5"
					>
						<span>response.json</span><span>200</span>
					</div>
					<pre class="mt-2 font-mono text-sm leading-[1.55] whitespace-pre max-md:text-[13px]"><code
							>&#123;
  "id": "<mark class="bg-accent px-0.5 text-ink">&#123;&#123;path.id&#125;&#125;</mark>",
  "team": "<mark class="bg-accent px-0.5 text-ink">&#123;&#123;query.team&#125;&#125;</mark>",
  "seen": "<mark class="bg-accent px-0.5 text-ink">&#123;&#123;now&#125;&#125;</mark>"
&#125;</code
						></pre>
				</div>
				<div
					class="relative z-[1] px-5 pt-3.5 pb-[18px] max-lg:px-4 max-lg:pt-3 max-lg:pb-4 max-md:px-3 max-md:pt-3 max-md:pb-3.5"
				>
					<h3 class="text-body leading-[1.1] font-semibold tracking-[-0.02em]">
						Dynamic templates
					</h3>
					<p class="mt-[5px] text-sm leading-[1.35]">Responses built from the request.</p>
				</div>
			</article>

			<article
				use:reveal
				class="col-span-3 row-span-2 flex flex-col bg-tile max-lg:col-span-6 max-lg:row-span-3 max-md:col-span-1 max-md:row-span-3"
				style="--d: 240ms"
			>
				<div
					class="min-h-0 flex-1 overflow-hidden px-5 pt-4 font-display max-lg:px-4 max-lg:pt-3.5 max-md:px-3 max-md:pt-3"
					data-ui
					aria-hidden="true"
				>
					<div
						class="flex justify-between gap-3 pb-2.5 text-[13px] font-medium text-muted max-md:pb-1.5"
					>
						<span>orders.yaml</span><span>OpenAPI 3.1</span>
					</div>
					<div class="text-lead leading-none font-bold tracking-[-0.03em]">48 operations</div>
					<div class="mt-3.5 flex flex-col">
						<div
							class="flex justify-between gap-2.5 border-t border-line py-[7px] text-[13px] max-md:py-[5px]"
						>
							<span>GET /orders</span><span class="text-muted">ready</span>
						</div>
						<div
							class="flex justify-between gap-2.5 border-t border-line py-[7px] text-[13px] max-md:py-[5px]"
						>
							<span>POST /orders</span><span class="text-muted">ready</span>
						</div>
						<div
							class="flex justify-between gap-2.5 border-t border-line py-[7px] text-[13px] max-md:py-[5px]"
						>
							<span>PATCH /orders/:id</span><span class="text-muted">ready</span>
						</div>
					</div>
					<div class="mt-3 text-[13px] font-medium text-muted max-md:mt-1.5">
						12 added · 2 changed
					</div>
				</div>
				<div
					class="relative z-[1] px-5 pt-3.5 pb-[18px] max-lg:px-4 max-lg:pt-3 max-lg:pb-4 max-md:px-3 max-md:pt-3 max-md:pb-3.5"
				>
					<h3 class="text-body leading-[1.1] font-semibold tracking-[-0.02em]">
						OpenAPI 2.0 / 3.x
					</h3>
					<p class="mt-[5px] text-sm leading-[1.35]">Import, diff, then export.</p>
				</div>
			</article>

			<article
				use:reveal
				class="col-span-3 row-span-2 flex flex-col bg-accent text-ink max-lg:col-span-6 max-lg:row-span-3 max-md:col-span-1 max-md:row-span-3"
				style="--d: 320ms"
			>
				<div
					class="min-h-0 flex-1 overflow-hidden px-5 pt-3.5 font-display max-lg:px-4 max-md:px-3 max-md:pt-3"
					data-ui
					aria-hidden="true"
				>
					<div
						class="flex justify-between gap-3 pb-2.5 text-[13px] font-medium text-ink opacity-80 max-md:pb-1.5"
					>
						<span>Fault injection</span><span>on</span>
					</div>
					<div class="flex items-center justify-between gap-3 py-[9px] text-[13px] max-md:py-1.5">
						<span>Delay</span><strong class="text-body font-semibold">800 ms</strong>
					</div>
					<div class="h-1.5 overflow-hidden bg-ink/15">
						<span class="block h-full bg-ink" style="width: 82%"></span>
					</div>
					<div class="flex items-center justify-between gap-3 py-[9px] text-[13px] max-md:py-1.5">
						<span>Fail 1 in 5</span>
						<span class="relative block h-5 w-[34px] bg-ink"
							><i class="absolute top-[3px] right-[3px] size-3.5 bg-accent"></i></span
						>
					</div>
					<div class="flex items-center justify-between gap-3 py-[9px] text-[13px] max-md:py-1.5">
						<span>Status</span><strong class="text-body font-semibold">503</strong>
					</div>
				</div>
				<div
					class="relative z-[1] px-5 pt-3.5 pb-[18px] max-lg:px-4 max-lg:pt-3 max-lg:pb-4 max-md:px-3 max-md:pt-3 max-md:pb-3.5"
				>
					<h3 class="text-body leading-[1.1] font-semibold tracking-[-0.02em]">Faults on demand</h3>
					<p class="mt-[5px] text-sm leading-[1.35]">Delays and failures, on purpose.</p>
				</div>
			</article>

			<article
				use:reveal
				class="col-span-3 row-span-4 flex flex-col bg-tile max-lg:col-span-6 max-lg:row-span-3 max-md:col-span-2 max-md:row-span-3"
				style="--d: 400ms"
			>
				<div
					class="min-h-0 flex-1 overflow-hidden px-5 pt-3.5 font-display max-lg:px-4 max-md:px-3 max-md:pt-3"
					data-ui
					aria-hidden="true"
				>
					<div
						class="flex justify-between gap-3 pb-2.5 text-[13px] font-medium text-muted max-md:pb-1.5"
					>
						<span>Request log</span><span>live</span>
					</div>
					<div
						class="flex items-center gap-2 border-t border-line py-2 text-[13px] whitespace-nowrap max-md:py-[7px]"
					>
						<span
							class="inline-flex h-6 min-w-[38px] items-center justify-center bg-ink px-1 text-[13px] font-semibold tracking-[0.02em] text-tile"
							>GET</span
						><span class="truncate">/v1/users/42</span><span
							class="ml-auto text-[13px] font-semibold">200</span
						>
					</div>
					<div
						class="flex items-center gap-2 border-t border-line py-2 text-[13px] whitespace-nowrap max-md:py-[7px]"
					>
						<span
							class="inline-flex h-6 min-w-[38px] items-center justify-center bg-ink px-1 text-[13px] font-semibold tracking-[0.02em] text-tile"
							>POST</span
						><span class="truncate">/v1/orders</span><span class="ml-auto text-[13px] font-semibold"
							>201</span
						>
					</div>
					<div
						class="flex items-center gap-2 border-t border-line py-2 text-[13px] whitespace-nowrap max-md:py-[7px]"
					>
						<span
							class="inline-flex h-6 min-w-[38px] items-center justify-center bg-ink px-1 text-[13px] font-semibold tracking-[0.02em] text-tile"
							>GET</span
						><span class="truncate">/v1/admin</span><span
							class="ml-auto text-[13px] font-semibold text-danger">401</span
						>
					</div>
					<div
						class="flex items-center gap-2 border-t border-line py-2 text-[13px] whitespace-nowrap max-md:py-[7px]"
					>
						<span
							class="inline-flex h-6 min-w-[38px] items-center justify-center bg-ink px-1 text-[13px] font-semibold tracking-[0.02em] text-tile"
							>PATCH</span
						><span class="truncate">/v1/orders/1042</span><span
							class="ml-auto text-[13px] font-semibold">200</span
						>
					</div>
				</div>
				<div
					class="relative z-[1] px-5 pt-3.5 pb-[18px] max-lg:px-4 max-lg:pt-3 max-lg:pb-4 max-md:px-3 max-md:pt-3 max-md:pb-3.5"
				>
					<h3 class="text-body leading-[1.1] font-semibold tracking-[-0.02em]">Request logs</h3>
					<p class="mt-[5px] text-sm leading-[1.35]">Every matched and unmatched call.</p>
				</div>
			</article>

			<article
				use:reveal
				class="reveal col-span-3 row-span-4 flex flex-col bg-surface max-lg:col-span-12 max-lg:row-span-3 max-md:col-span-2 max-md:row-span-3"
				style="--d: 480ms"
			>
				<div
					class="min-h-0 flex-1 overflow-hidden px-5 pt-4 font-display max-lg:px-4 max-lg:pt-3.5 max-md:px-3 max-md:pt-3"
					data-ui
					aria-hidden="true"
				>
					<div
						class="flex justify-between gap-3 pb-2.5 text-[13px] font-medium text-muted max-md:pb-1.5"
					>
						<span>Workspaces</span><span>3 environments</span>
					</div>
					<div
						class="flex items-center gap-3 border-t border-line-strong py-3 text-body max-md:gap-2 max-md:py-[9px]"
					>
						<span class="size-2 flex-none rounded-full border border-ink bg-accent"></span>
						<span class="truncate font-medium">acme.mocktail.dev</span>
						<span class="ml-auto text-[13px] text-muted">Owner</span>
						<span
							class="inline-flex size-[30px] flex-none items-center justify-center rounded-full bg-ink text-[13px] font-semibold text-tile max-md:size-[26px]"
							>AL</span
						>
					</div>
					<div
						class="flex items-center gap-3 border-t border-line-strong py-3 text-body max-md:gap-2 max-md:py-[9px]"
					>
						<span class="size-2 flex-none rounded-full border border-ink bg-accent"></span>
						<span class="truncate font-medium">staging.mocktail.dev</span>
						<span class="ml-auto text-[13px] text-muted">Editor</span>
						<span
							class="inline-flex size-[30px] flex-none items-center justify-center rounded-full bg-ink text-[13px] font-semibold text-tile max-md:size-[26px]"
							>MK</span
						>
					</div>
					<div
						class="flex items-center gap-3 border-t border-line-strong py-3 text-body max-md:gap-2 max-md:py-[9px]"
					>
						<span class="size-2 flex-none rounded-full border border-ink bg-accent"></span>
						<span class="truncate font-medium">sandbox.mocktail.dev</span>
						<span class="ml-auto text-[13px] text-muted">Viewer</span>
						<span
							class="inline-flex size-[30px] flex-none items-center justify-center rounded-full bg-ink text-[13px] font-semibold text-tile max-md:size-[26px]"
							>JD</span
						>
					</div>
					<div class="border-t border-line-strong pt-3.5 text-[13px] text-muted max-md:pt-2">
						SSO · GitHub, Google, GitLab
					</div>
				</div>
				<div
					class="relative z-[1] px-5 pt-3.5 pb-[18px] max-lg:px-4 max-lg:pt-3 max-lg:pb-4 max-md:px-3 max-md:pt-3 max-md:pb-3.5"
				>
					<h3 class="text-body leading-[1.1] font-semibold tracking-[-0.02em]">
						Workspaces &amp; RBAC
					</h3>
					<p class="mt-[5px] text-sm leading-[1.35]">Host-routed environments with roles.</p>
				</div>
			</article>

			<article
				use:reveal
				class="col-span-12 row-span-4 flex flex-col bg-dark text-dark-ink max-lg:row-span-3 max-md:col-span-2 max-md:row-span-3"
				style="--d: 560ms"
			>
				<div
					class="min-h-0 flex-1 overflow-hidden px-5 pt-4 font-display max-lg:px-4 max-lg:pt-3.5 max-md:px-3 max-md:pt-3"
					data-ui
					aria-hidden="true"
				>
					<div
						class="flex justify-between gap-3 pb-2.5 text-[13px] font-medium text-dark-muted max-md:pb-1.5"
					>
						<span>Deploy</span><span>healthy</span>
					</div>
					<pre
						class="font-mono text-body leading-[1.6] whitespace-pre text-dark-ink max-lg:text-[15px] max-md:text-body max-md:leading-[1.55]"><code
							>$ make dev
  mocktail   listening on :8787
  sqlite     mocktail.db
  assets     frontend/build

$ npx wrangler deploy
  worker     acme.workers.dev
  d1         mocktail</code
						></pre>
				</div>
				<div
					class="relative z-[1] px-5 pt-3.5 pb-[18px] max-lg:px-4 max-lg:pt-3 max-lg:pb-4 max-md:px-3 max-md:pt-3 max-md:pb-3.5"
				>
					<h3 class="text-body leading-[1.1] font-semibold tracking-[-0.02em]">
						Self-hosted or edge
					</h3>
					<p class="mt-[5px] text-sm leading-[1.35] text-dark-ink">
						Native binary, Docker, or Workers + D1.
					</p>
				</div>
			</article>
		</section>

		<section
			use:reveal
			class="mt-2 bg-dark pt-[clamp(56px,7vw,96px)] text-dark-ink max-md:mt-1.5 max-md:pt-14"
			id="requests"
			aria-labelledby="requests-title"
		>
			<div
				class="flex flex-col gap-[18px] px-[clamp(16px,3vw,36px)] pb-[clamp(32px,4vw,56px)] max-md:gap-3 max-md:px-4 max-md:pb-7"
			>
				<h2 id="requests-title" class="max-w-[15ch] text-head">Every request, accounted for.</h2>
				<p class="max-w-[58ch] text-body">
					Matched, unmatched, delayed or failed. Open a row to see the exact request and the
					response Mocktail returned.
				</p>
			</div>

			<div class="reveal flex flex-col gap-1 pb-6 max-md:pb-4" style="--d: 100ms">
				{#each logRows as row, i (row.path)}
					<LogEntry {...row} open={openRow === i} onToggle={() => (openRow = i)} />
				{/each}
			</div>
		</section>

		<section
			use:reveal
			class="mt-2 grid grid-cols-1 gap-2 max-md:gap-1.5 md:grid-cols-2"
			aria-labelledby="deploy-guide-title"
		>
			<div class="bg-surface p-[clamp(24px,3vw,40px)] max-md:p-6">
				<h2 id="deploy-guide-title" class="mb-6 text-head">Get started in minutes</h2>
				<div class="flex flex-col gap-8">
					<div class="flex gap-4">
						<span
							class="flex size-7 flex-none items-center justify-center bg-ink font-display text-sm font-bold text-tile"
							>1</span
						>
						<div>
							<h3 class="font-display text-body font-semibold">Clone the repo</h3>
							<p class="text-sm text-muted">
								Get the source code from GitHub to your local machine.
							</p>
						</div>
					</div>
					<div class="flex gap-4">
						<span
							class="flex size-7 flex-none items-center justify-center bg-ink font-display text-sm font-bold text-tile"
							>2</span
						>
						<div>
							<h3 class="font-display text-body font-semibold">Build and run</h3>
							<p class="text-sm text-muted">
								Use <code class="border border-line bg-tile px-1">make dev</code> to compile and
								start the server.
							</p>
						</div>
					</div>
					<div class="flex gap-4">
						<span
							class="flex size-7 flex-none items-center justify-center bg-ink font-display text-sm font-bold text-tile"
							>3</span
						>
						<div>
							<h3 class="font-display text-body font-semibold">Start mocking</h3>
							<p class="text-sm text-muted">
								Access the dashboard at <code class="border border-line bg-tile px-1"
									>localhost:8787</code
								>.
							</p>
						</div>
					</div>
				</div>
				<h3 class="mt-8 font-display text-body font-bold">Local Deployment</h3>
			</div>

			<div
				class="reveal bg-tile p-[clamp(24px,3vw,40px)] max-md:p-6"
				style="--d: 100ms"
			>
				<div class="flex flex-col gap-8">
					<div class="flex gap-4">
						<span
							class="flex size-7 flex-none items-center justify-center bg-accent font-display text-sm font-bold text-ink"
							>1</span
						>
						<div>
							<h3 class="font-display text-body font-semibold">Wrangler Login</h3>
							<p class="text-sm text-muted">
								Authenticate with <code class="border border-line bg-surface px-1"
									>npx wrangler login</code
								>.
							</p>
						</div>
					</div>
					<div class="flex gap-4">
						<span
							class="flex size-7 flex-none items-center justify-center bg-accent font-display text-sm font-bold text-ink"
							>2</span
						>
						<div>
							<h3 class="font-display text-body font-semibold">Initialize D1</h3>
							<p class="text-sm text-muted">
								Create a Cloudflare D1 database to store your mocks and logs.
							</p>
						</div>
					</div>
					<div class="flex gap-4">
						<span
							class="flex size-7 flex-none items-center justify-center bg-accent font-display text-sm font-bold text-ink"
							>3</span
						>
						<div>
							<h3 class="font-display text-body font-semibold">Edge Deploy</h3>
							<p class="text-sm text-muted">
								Run <code class="border border-line bg-surface px-1"
									>npx wrangler deploy</code
								> to go live globally.
							</p>
						</div>
					</div>
				</div>
				<h3 class="mt-8 font-display text-body font-bold">Cloudflare Workers</h3>
			</div>
		</section>

		<section
			use:reveal
			class="mt-2 grid grid-cols-[1fr_5fr_1.4fr] gap-2 [grid-template-areas:'left_main_right'] max-lg:grid-cols-2 max-lg:[grid-template-areas:'main_main'_'left_right'] max-md:mt-1.5 max-md:gap-1.5"
			id="deploy"
			aria-labelledby="cta-title"
		>
			<div
				class="flex flex-col justify-between bg-dark p-5 text-dark-ink [grid-area:left] max-lg:min-h-[120px] max-md:min-h-[140px] max-md:p-4"
				style="--d: 0ms"
			>
				<span class="block size-[clamp(72px,9vw,128px)] max-md:size-14">{@render mark()}</span>
			</div>

			<div
				class="flex min-h-[360px] flex-col items-start justify-center gap-5 bg-accent p-[clamp(28px,4vw,56px)] text-ink [grid-area:main] max-md:min-h-[300px] max-md:gap-4 max-md:p-6"
				style="--d: 80ms"
			>
				<h2 id="cta-title" class="max-w-[16ch] text-head">Run it on your own metal.</h2>
				<p class="max-w-[52ch] text-body">
					No account and no hosted service. Clone the repo, run one command, and point your client
					at localhost.
				</p>
				<div class="mt-2 flex flex-wrap items-center gap-6 max-md:gap-4">
					<a
						class="inline-flex min-h-11 items-center justify-center bg-ink px-[18px] font-display text-sm font-semibold text-tile transition-[background-color,color,transform] hover:bg-tile hover:text-ink active:scale-[0.98]"
						href="https://github.com/mnabila/mocktail#quickstart"
					>
						Read the quickstart
					</a>
					<a
						class="inline-flex items-center gap-1.5 font-display text-sm font-medium underline decoration-1 underline-offset-4 transition-[color,opacity] hover:text-ink hover:opacity-60"
						href="https://github.com/mnabila/mocktail"
					>
						Star on GitHub <span aria-hidden="true">↗</span>
					</a>
				</div>
			</div>

			<div
				class="flex flex-col justify-between bg-surface p-5 text-ink [grid-area:right] max-lg:min-h-[120px] max-md:min-h-[140px] max-md:p-4"
				style="--d: 160ms"
			>
				<div
					class="overflow-hidden font-mono text-[13px] leading-[1.55] whitespace-pre text-ink"
					data-ui
					aria-hidden="true"
				>
					<pre><code
							>$ make dev

mocktail listening
on :8787</code
						></pre>
				</div>
			</div>
		</section>

		<section
			use:reveal
			class="mt-2 grid grid-cols-3 gap-2 max-md:mt-1.5 max-md:grid-cols-1 max-md:gap-1.5"
			id="credits"
			aria-labelledby="credits-title"
		>
			<div
				class="col-span-3 flex flex-col gap-3 bg-dark p-[clamp(24px,3vw,40px)] text-dark-ink max-md:col-span-1 max-md:p-6"
			>
				<h2 id="credits-title" class="text-head">Inspired by</h2>
				<p class="max-w-[58ch] text-body opacity-90">
					Mocktail stands on the tools that shaped how developers mock and ship APIs.
				</p>
			</div>

			<a
				use:reveal
				class="flex flex-col gap-3 bg-surface p-6 transition-colors hover:bg-tile max-md:p-5"
				href="https://www.cloudflare.com"
				style="--d: 100ms"
			>
				<span class="font-display text-sm font-medium text-muted">Palette</span>
				<h3 class="font-display text-lead font-semibold tracking-[-0.025em]">Cloudflare</h3>
				<p class="text-sm text-muted">
					The accent orange is a nod to Cloudflare, the edge that Workers and D1 deploy to.
				</p>
			</a>

			<a
				use:reveal
				class="flex flex-col gap-3 bg-surface p-6 transition-colors hover:bg-tile max-md:p-5"
				href="https://www.postman.com"
				style="--d: 180ms"
			>
				<span class="font-display text-sm font-medium text-muted">Portal UX</span>
				<h3 class="font-display text-lead font-semibold tracking-[-0.025em]">Postman</h3>
				<p class="text-sm text-muted">
					The portal, especially the collection sidebar, follows patterns Postman made familiar.
				</p>
			</a>

			<a
				use:reveal
				class="flex flex-col gap-3 bg-surface p-6 transition-colors hover:bg-tile max-md:p-5"
				href="https://wiremock.org"
				style="--d: 260ms"
			>
				<span class="font-display text-sm font-medium text-muted">Core concept</span>
				<h3 class="font-display text-lead font-semibold tracking-[-0.025em]">WireMock &amp; Postman Mock</h3>
				<p class="text-sm text-muted">
					Request matching and stubbed responses, the heart of Mocktail, stand on their mock servers.
				</p>
			</a>
		</section>
	</main>

	<footer class="mt-2 overflow-hidden bg-dark text-dark-ink max-md:mt-1.5">
		<div class="flex flex-col gap-12 p-[clamp(32px,4vw,64px)] max-md:p-6">
			<div class="flex items-center gap-4 text-accent">
				<span class="block size-10">{@render mark()}</span>
				<p class="font-display text-2xl font-semibold tracking-[-0.03em] text-dark-muted">
					Mocktail
				</p>
			</div>

			<div class="grid grid-cols-4 gap-6 max-md:grid-cols-2 max-md:gap-x-5 max-md:gap-y-7">
				<div class="flex flex-col items-start gap-2.5">
					<h3 class="mb-1 font-display text-sm leading-[1.2] font-semibold tracking-normal">
						Product
					</h3>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="#capabilities">Capabilities</a
					>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="#requests">Request log</a
					>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="#deploy">Deploy</a
					>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="#credits">Credits</a
					>
				</div>
				<div class="flex flex-col items-start gap-2.5">
					<h3 class="mb-1 font-display text-sm leading-[1.2] font-semibold tracking-normal">
						Docs
					</h3>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="https://github.com/mnabila/mocktail#quickstart">Quickstart</a
					>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="https://github.com/mnabila/mocktail#configuration">Configuration</a
					>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="https://github.com/mnabila/mocktail#features">OpenAPI</a
					>
				</div>
				<div class="flex flex-col items-start gap-2.5">
					<h3 class="mb-1 font-display text-sm leading-[1.2] font-semibold tracking-normal">
						Source
					</h3>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="https://github.com/mnabila/mocktail">GitHub</a
					>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="https://github.com/mnabila/mocktail/blob/main/LICENSE">License</a
					>
					<a
						class="font-display text-sm leading-[1.4] text-dark-muted transition-colors hover:text-accent"
						href="https://github.com/mnabila/mocktail/issues">Issues</a
					>
				</div>
				<div class="flex flex-col items-start gap-2.5">
					<h3 class="mb-1 font-display text-sm leading-[1.2] font-semibold tracking-normal">
						Built with
					</h3>
					<p class="font-display text-sm leading-[1.4] text-dark-muted">
						Rust, SQLite, SvelteKit and Cloudflare Workers.
					</p>
				</div>
			</div>

			<div
				class="flex items-center justify-between gap-5 border-t border-dark-line pt-8 font-display text-sm text-dark-muted"
			>
				<p>© 2026 Mocktail. Licensed under GPL-3.0.</p>
				<a class="whitespace-nowrap text-dark-ink underline underline-offset-4" href="#top"
					>Back to top ↑</a
				>
			</div>
		</div>
	</footer>
</div>
