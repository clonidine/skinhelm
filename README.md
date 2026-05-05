# skinhelm

`skinhelm` is a small Rust HTTP service that renders 2D Minecraft skin heads with the front helmet / hat layer composited over the base face. It accepts a Minecraft username or UUID, resolves the Mojang profile, downloads the skin PNG, and returns a square PNG scaled with nearest-neighbor filtering.

## Features

- `GET /health` health check.
- `GET /helm/{player}` for usernames, hyphenated UUIDs, or compact UUIDs.
- Mojang username resolution and sessionserver profile lookup.
- Base64 `textures` decoding and skin URL extraction.
- Minecraft `64x64` skin support with helmet / hat overlay.
- Legacy `64x32` skin support without overlay.
- Pixel-perfect nearest-neighbor scaling.
- In-memory TTL cache for username resolution, skin URLs, and rendered PNGs.
- Structured application logging through `tracing`.

## Requirements

- Stable Rust toolchain.
- Network access to Mojang APIs and the returned skin texture URLs.

## Workspace Layout

`skinhelm` is organized as a small Cargo workspace:

- `crates/skinhelm`: HTTP server, Axum routes, application state, logging setup, and graceful shutdown.
- `crates/skinhelm-core`: Mojang client, validation, cache, PNG rendering, shared types, and domain errors.
- `crates/skinhelm-wasm`: WebAssembly/WebGL browser viewer for complete 3D Minecraft skins.

## Run Locally

```sh
cargo run
```

The service binds to `0.0.0.0:3000`.

## Endpoints

### `GET /health`

Returns:

```json
{ "status": "ok" }
```

### `GET /helm/{player}`

`player` may be:

- A Minecraft username.
- A UUID with hyphens.
- A UUID without hyphens.

Successful responses return `image/png` with:

```text
Content-Type: image/png
Cache-Control: public, max-age=3600
```

## Query Parameters

- `size`: optional output size in pixels.
- Default: `180`.
- Minimum: `8`.
- Maximum: `512`.

Invalid `size` values return HTTP `400`.

## Curl Examples

Default `180x180` output:

```sh
curl -o head.png "http://localhost:3000/helm/bc881e0292164f6ea80f7b9df0ccf9e9"
```

Explicit `180x180` output:

```sh
curl -o head.png "http://localhost:3000/helm/bc881e0292164f6ea80f7b9df0ccf9e9?size=180"
```

```sh
curl -o steve.png "http://localhost:3000/helm/Steve?size=256"
```

```sh
curl "http://localhost:3000/health"
```

## Minecraft Skin Coordinates

For modern `64x64` skins, `skinhelm` uses the standard front head regions:

- Base head front: `x = 8`, `y = 8`, `width = 8`, `height = 8`.
- Helmet / hat overlay front: `x = 40`, `y = 8`, `width = 8`, `height = 8`.

The base face is rendered first. The overlay is alpha-composited on top, preserving transparency. The final `8x8` result is resized to `size x size` using nearest-neighbor filtering, so the output stays sharp and pixel-perfect.

For legacy `64x32` skins, the overlay is ignored and only the base head front is rendered.

## Caching Behavior

`skinhelm` uses an internal in-memory cache implemented with `HashMap` and `tokio::sync::RwLock`.

- Username to UUID: 6 hours.
- UUID to skin URL: 30 minutes.
- Skin URL plus size to rendered PNG: 30 minutes.

Expired entries are cleaned lazily during cache access and writes. There is no external cache dependency.

## Logging

Application logs use `tracing` and `tracing-subscriber`. The server logs startup, cache hits at debug level, and upstream or internal errors with structured fields where useful.

## Error Responses

All JSON errors use:

```json
{ "error": "short message" }
```

Status mapping:

- `400`: invalid player or invalid size.
- `404`: username not found or profile has no skin.
- `502`: Mojang request failure, malformed upstream data, texture decode failure, texture JSON failure, skin download failure, or invalid downloaded PNG.
- `500`: unexpected internal error.

## Test Commands

```sh
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## Release Build

```sh
cargo build --release
```

## Known Limitations

- The service renders only the flat 2D front head with optional helmet / hat overlay.
- It does not render 3D cubes, lighting, shadows, perspective, or smoothing.
- Cache is process-local and is cleared when the service restarts.
- Mojang and texture CDN availability directly affect uncached requests.

## Future Extension Ideas

- `/avatar`
- `/head`
- `/headhelm`
