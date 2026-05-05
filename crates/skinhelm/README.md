# skinhelm

`skinhelm` is the HTTP binary crate for the workspace. It runs an Axum server that accepts a Minecraft username or UUID, queries Mojang APIs, downloads the matching skin, and returns a square PNG of the player's head with the helmet/hat layer composited over the base face.

This crate owns the application layer: routing, shared state, logging, in-memory caching, and graceful shutdown. Validation, Mojang integration, image rendering, and shared domain types live in `skinhelm-core`.

## Overview

- Rust HTTP server built with `axum` and `tokio`.
- Health endpoint at `GET /health`.
- Head rendering endpoint at `GET /helm/{player}`.
- Accepts usernames, hyphenated UUIDs, and compact UUIDs.
- Returns `image/png` using pixel-perfect nearest-neighbor scaling.
- Uses an in-memory cache for username-to-UUID lookups, UUID-to-skin-URL lookups, and rendered PNGs.
- Can serve the `skinhelm-wasm` WebAssembly/WebGL viewer through the optional `wasm-viewer` feature.
- Uses `tracing` and `tracing-subscriber` for logging.

## Running

From the workspace root:

```sh
cargo run -p skinhelm
```

The server listens on:

```text
0.0.0.0:3000
```

Check process health with:

```sh
curl "http://localhost:3000/health"
```

Expected response:

```json
{ "status": "ok" }
```

## Rendering a Head

Use `GET /helm/{player}`. The `player` path parameter may be:

- a valid Minecraft username, such as `Steve`;
- a hyphenated UUID;
- a compact 32-character hexadecimal UUID.

Examples:

```sh
curl -o head.png "http://localhost:3000/helm/Steve"
```

```sh
curl -o head-256.png "http://localhost:3000/helm/bc881e0292164f6ea80f7b9df0ccf9e9?size=256"
```

The optional `size` query parameter controls the output PNG dimensions:

- default: `180`;
- minimum: `8`;
- maximum: `512`.

Successful responses include:

```text
Content-Type: image/png
Cache-Control: public, max-age=3600
```

Errors are returned as JSON:

```json
{ "error": "short message" }
```

Primary status mapping:

- `400`: invalid player or invalid `size`;
- `404`: username not found, missing skin, or missing cape on viewer cape routes;
- `502`: Mojang request failure, texture download failure, or invalid upstream data;
- `500`: unexpected internal error.

## `wasm-viewer` Feature

The optional `wasm-viewer` feature embeds generated files from the `skinhelm-wasm` crate and exposes a 3D viewer on the same server.

Run:

```sh
cargo run -p skinhelm --features wasm-viewer
```

Additional routes:

- `GET /viewer`: redirects to `/viewer/`;
- `GET /viewer/`: serves the viewer HTML;
- `GET /viewer/{uuid}`: opens the viewer for a UUID;
- `GET /viewer/skin/{uuid}`: downloads the skin and reports the `classic` or `slim` model through headers;
- `GET /viewer/cape/{uuid}`: downloads the official cape when available;
- `GET /viewer/bootstrap.js`;
- `GET /viewer/pkg/skinhelm_wasm.js`;
- `GET /viewer/pkg/skinhelm_wasm_bg.wasm`.

Example:

```text
http://localhost:3000/viewer/bc881e0292164f6ea80f7b9df0ccf9e9
```

Before using this feature, generate the WASM artifacts in `crates/skinhelm-wasm/pkg` as documented in the `skinhelm-wasm` README.

## Relationship With Other Workspace Crates

- `skinhelm`: this crate. Implements the HTTP binary, route setup, application state, logging, and `Ctrl+C` shutdown handling.
- `skinhelm-core`: shared library used by this crate for input validation, Mojang requests, `textures` payload decoding, image downloads, caching, and 2D PNG rendering.
- `skinhelm-wasm`: WebAssembly/WebGL viewer for rendering the full Minecraft model in the browser. It is independent from the backend, but can be served by `skinhelm` with `wasm-viewer`.

## Development Commands

From the workspace root:

```sh
cargo fmt
cargo clippy -p skinhelm -- -D warnings
cargo test -p skinhelm
```

Most domain behavior used by this crate lives in `skinhelm-core`, so it is also useful to run:

```sh
cargo test -p skinhelm-core
```

Release build:

```sh
cargo build -p skinhelm --release
```

## Development Notes

- `src/main.rs` initializes logging, the Mojang client, the shared cache, and the TCP listener.
- `src/routes.rs` defines HTTP routes, converts domain errors into responses, and coordinates cache usage.
- `src/shutdown.rs` waits for `Ctrl+C` and enables graceful shutdown.
- `src/viewer.rs` exists only with `wasm-viewer` and serves static files embedded with `include_str!` and `include_bytes!`.
- The cache is local to the running process; restarting the server clears it.
- Uncached requests depend on Mojang API availability and on the texture URLs returned by Mojang.
