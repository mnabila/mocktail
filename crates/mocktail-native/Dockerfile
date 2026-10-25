# Multi-stage build for the native mocktail binary (Docker target).

# 1. Build the SvelteKit portal; its output is embedded into the binary.
FROM node:current-slim AS frontend
WORKDIR /fe
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install
COPY frontend/ ./
RUN npm run build

# 2. Build the Rust binary with the portal assets in place.
FROM rust:1-slim AS build
WORKDIR /src
COPY . .
# rust-embed reads frontend/build at compile time.
COPY --from=frontend /fe/build ./frontend/build
RUN cargo build --release --bin mocktail

# 3. Minimal runtime image.
FROM debian:stable-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/mocktail /usr/local/bin/mocktail
# One HTTP port for both admin and mock traffic.
EXPOSE 8787
ENTRYPOINT ["mocktail"]
CMD ["--config", "/etc/mocktail/config.yaml"]
