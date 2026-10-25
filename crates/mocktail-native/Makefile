# mocktail developer Makefile.
# Local dev uses the SQLite store (no external database required).

# Overridable settings.
BIND            ?= 0.0.0.0:8787
DB              ?= mocktail.db
CONFIG          ?=
EXAMPLE_CONFIG  := example.config.yaml

# Run args precedence:
#   CONFIG=path    -> load that file
#   ./config.yaml  -> load it by default (copy from $(EXAMPLE_CONFIG))
#   otherwise      -> zero-config dev overrides (SQLite + plain-HTTP cookies)
ifneq ($(CONFIG),)
RUN_ARGS = --config $(CONFIG)
else ifneq ($(wildcard config.yaml),)
RUN_ARGS = --config config.yaml
else
RUN_ARGS = -o server.bind=$(BIND) -o sqlite.path=$(DB) -o auth.secureCookies=false
endif

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

.PHONY: config
config: ## Create config.yaml from example.config.yaml if missing
	@test -f config.yaml || cp $(EXAMPLE_CONFIG) config.yaml

.PHONY: dev
dev: frontend ## Run the portal locally (SQLite, http://localhost:8787)
	cargo run -p mocktail-native -- $(RUN_ARGS)

.PHONY: run
run: ## Run without rebuilding the frontend (uses embedded assets)
	cargo run -p mocktail-native -- $(RUN_ARGS)

.PHONY: frontend
frontend: ## Build the SvelteKit portal into frontend/build (embedded by the binary)
	cd ../../frontend/ && npm install && npm run build

.PHONY: build
build: frontend ## Build the whole workspace (debug)
	cargo build

.PHONY: release
release: frontend ## Build the native binary (release)
	cargo build --release

.PHONY: test
test: ## Run the workspace test suite
	cargo test

.PHONY: lint
lint: ## Run clippy across all targets
	cargo clippy --all-targets

.PHONY: fmt
fmt: ## Format the code
	cargo fmt

.PHONY: fmt-check
fmt-check: ## Check formatting without writing
	cargo fmt --check

.PHONY: wasm
wasm: ## WASM-clean gate (core, http, sqlite infra, worker)
	cargo build -p mocktail-core --target wasm32-unknown-unknown
	cargo build -p mocktail-http --target wasm32-unknown-unknown
	cargo build -p mocktail-infra-sqlite --target wasm32-unknown-unknown
	cargo build -p mocktail-worker --target wasm32-unknown-unknown

.PHONY: check
check: fmt-check lint test wasm ## Run all checks (fmt, lint, test, wasm gate)

.PHONY: docker
docker: ## Build the Docker image
	docker build -t mocktail .

.PHONY: clean
clean: ## Remove build artifacts and the local SQLite database
	cargo clean
	rm -rf frontend/build frontend/.svelte-kit
	rm -f $(DB) $(DB)-wal $(DB)-shm
