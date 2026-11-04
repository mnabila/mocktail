# Mocktail

Self-hostable API mocking in a single Rust binary. Point your app at Mocktail
instead of a real or not-yet-built backend: match incoming requests, return
templated responses, replay OpenAPI specs, inject faults, and inspect every
call. No account, no hosted service, no monthly bill - your mocks and logs stay
on your hardware.

The same core runs two ways: a native binary backed by SQLite (one file, no
external database), or Cloudflare Workers backed by D1. An embedded SvelteKit
admin portal ships inside both.

## Features

- **Workspaces & collections** - Host-routed workspaces
  (`{workspace}.<adminHost>/{collection}/...`) with RBAC (`owner`, `editor`,
  `viewer`).
- **Request matching** - Match by method, path (`:param` / `{param}`), query,
  headers, and body (exact, regex, JSON subset), with deterministic precedence.
- **Response templating** - Build responses from the request itself:
  `{{path.x}}`, `{{query.x}}`, `{{header.x}}`, `{{body}}`.
- **Faults & delays** - Inject latency, malformed bodies, and connection resets
  to test how your app handles a failing upstream.
- **OpenAPI 2.0 / 3.x** - Import specs (JSON/YAML) with diff preview and
  round-trip re-import, or export a collection back out.
- **SSO auth** - Google, GitHub, and GitLab OAuth2/OIDC with explicit account
  linking (never auto-merge by email).
- **Request logs** - Inspect matched and unmatched traffic per workspace.

## Quickstart

The Makefile, Dockerfile, and example config live in the native crate, so run
the local commands from there:

```bash
cd crates/mocktail-native
```

### Local development

Requires Rust (edition 2024) and Node.js 22+. One command builds the SvelteKit
portal and starts the server:

```bash
make dev
```

This serves the portal, admin API, and mock runtime on
`http://localhost:8787`, backed by `mocktail.db` (SQLite). With no `config.yaml`
present, `make` falls back to zero-config dev overrides (SQLite + plain-HTTP
cookies). Run `make help` to list every target (`build`, `release`, `test`,
`lint`, `wasm`, `docker`, ...).

For frontend hot-reload against a running backend:

```bash
make run                                                    # terminal 1: backend on :8787
cd ../../frontend && MOCKTAIL_API_URL=http://localhost:8787 npm run dev  # terminal 2: Vite dev server
```

### Configuration

Copy [`example.config.yaml`](crates/mocktail-native/example.config.yaml) to
`config.yaml` and edit it, or override any dotted key per run with `-o`:

```bash
cargo run -p mocktail-native -- --config config.yaml -o auth.sessionSecret="$SECRET"
```

Admin sign-in needs at least one SSO provider configured plus
`auth.sessionSecret`. For plain-HTTP local access, set
`server.adminHost: localhost`, `server.publicBaseUrl: http://localhost:8787`,
and `auth.secureCookies: false`.

### Docker

```bash
cd crates/mocktail-native
cp example.config.yaml config.yaml   # set adminHost/publicBaseUrl and SSO credentials
docker compose up --build
```

Builds the image from the local [`Dockerfile`](crates/mocktail-native/Dockerfile)
and serves on `http://localhost:8787`. SQLite state persists on the
`mocktail-data` volume (`docker compose down -v` removes it). `make docker`
builds the image without starting it.

### Cloudflare Workers

The worker in [`crates/mocktail-worker`](crates/mocktail-worker) stores data in
Cloudflare D1. Configuration lives in
[`wrangler.toml`](crates/mocktail-worker/wrangler.toml). Local preview needs no
Cloudflare account (D1 runs locally):

```bash
cd crates/mocktail-worker
npm install
cp .dev.vars.example .dev.vars
npm run d1:migrate:local
npm run dev   # http://localhost:8787
```

If you change the D1 `database_id`, rerun `npm run d1:migrate:local`: Miniflare
keeps one local database per ID.

The portal, admin API, and mock runtime all run on that one URL. Add OAuth
credentials to `.dev.vars` if you need to sign in locally. To deploy:

```bash
cd crates/mocktail-worker
npx wrangler login
npx wrangler d1 create mocktail   # paste database_id into wrangler.toml
npm run d1:migrate:remote
npm run deploy
npx wrangler secret put SESSION_SECRET   # deploys a new version
# Optional SSO: GOOGLE_CLIENT_ID/SECRET, GITHUB_CLIENT_ID/SECRET,
# GITLAB_CLIENT_ID/SECRET
```

`npm run deploy` builds the SvelteKit portal into `frontend/build` and uploads
it with the Worker as static assets; the Worker serves it on the admin host
with SPA fallback. Set `APEX_HOST`/`PUBLIC_BASE_URL` in `wrangler.toml` and the
matching `routes` for your zone. Without a route, use the Worker's
`<name>.<account>.workers.dev` hostname as `APEX_HOST`; workspace subdomains
require the route. Requests are host-routed: admin on the apex host, mocks on
`{workspace}.<APEX_HOST>` (requires wildcard DNS + TLS). Register OAuth
callbacks as `<PUBLIC_BASE_URL>/auth/{provider}/callback`.

#### DNS and routes

To serve on a custom domain, the zone (`mnabila.my.id` in the sample) must use
Cloudflare nameservers. Point `routes` in `wrangler.toml` at your admin host and
its wildcard, then add two **proxied** (orange-cloud) DNS records in the
Cloudflare dashboard so both resolve to the Worker:

| Type   | Name                      | Content | Proxy   |
| ------ | ------------------------- | ------- | ------- |
| `AAAA` | `mocktail` (admin host)   | `100::` | Proxied |
| `AAAA` | `*.mocktail` (workspaces) | `100::` | Proxied |

`100::` is Cloudflare's discard address; the orange-cloud proxy makes the Worker
route take over before the origin is ever hit. Workspace hosts
(`{workspace}.mocktail.mnabila.my.id`) are sub-subdomains, which **Universal SSL
does not cover** - enable [Advanced Certificate Manager][acm] (or Total TLS) so
`*.mocktail.mnabila.my.id` gets a wildcard cert. A plain apex host
(`example.com` + `*.example.com`) is covered by Universal SSL and needs no ACM.

[acm]: https://developers.cloudflare.com/ssl/edge-certificates/advanced-certificate-manager/

#### Worker configuration

The Worker does not read `config.yaml`; that file is only used by the native
binary and Docker image. Use `[vars]` in `wrangler.toml` for non-secrets and
Wrangler secrets for credentials:

| `config.yaml`                           | Worker                                          |
| --------------------------------------- | ----------------------------------------------- |
| `server.adminHost`                      | `APEX_HOST` (var; hostname only, no scheme)     |
| `server.publicBaseUrl`                  | `PUBLIC_BASE_URL` (var; full URL)               |
| `auth.sessionSecret`                    | `SESSION_SECRET` (secret, required)             |
| `auth.google.clientId` / `clientSecret` | `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`     |
| `auth.github.clientId` / `clientSecret` | `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET`     |
| `auth.gitlab.clientId` / `clientSecret` | `GITLAB_CLIENT_ID` / `GITLAB_CLIENT_SECRET`     |
| `server.bind`                           | Not used; Cloudflare provides the HTTP endpoint |
| `sqlite.path`                           | Not used; D1 binding `DB` in `wrangler.toml`    |
| `auth.sessionTtlHours`                  | Fixed at 168 hours (7 days)                     |
| `auth.secureCookies`                    | Fixed at `true`                                 |

The callback URL defaults to `<PUBLIC_BASE_URL>/auth/{provider}/callback`. To
override it per provider, set `GOOGLE_REDIRECT_URL` / `GITHUB_REDIRECT_URL` /
`GITLAB_REDIRECT_URL` (var or secret) and register that URL instead.

A provider is enabled only when both its ID and secret are present. For
production, set secrets with:

```bash
npx wrangler secret put SESSION_SECRET
npx wrangler secret put GITHUB_CLIENT_ID
npx wrangler secret put GITHUB_CLIENT_SECRET
```

Locally, put the same names in `.dev.vars` (see `.dev.vars.example`). Changing
`[vars]` requires `npm run deploy`; `wrangler secret put` publishes a new
version immediately.

## Checks

From `crates/mocktail-native`, `make check` runs everything (fmt, lint, test,
and the WASM-clean gate). Or run them directly:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude mocktail-worker --all-targets -- -D warnings
cargo test --workspace --exclude mocktail-worker
cargo check -p mocktail-worker --target wasm32-unknown-unknown
```

## License

[GPL-3.0](LICENSE)

---

Inspired by WireMock, Postman Mock Server, and other API mocking tools.
