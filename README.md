# skinhelm

`skinhelm` is a Rust workspace for viewing Minecraft skins. Its main experience is a browser-based 3D skin viewer built with WebAssembly and WebGL2, backed by a small Axum service that can resolve Minecraft usernames or UUIDs through Mojang, fetch official skins and capes, and serve the viewer as a self-contained web page.

The project also includes a lightweight 2D helmeted-head PNG endpoint for integrations that only need a square avatar image.

## What It Does

- Runs a full 3D Minecraft skin viewer in the browser.
- Uses Rust compiled to WebAssembly through `wasm-bindgen`.
- Renders with WebGL2 directly through `web-sys`, without an external 3D engine.
- Supports modern `64x64` skins and legacy `64x32` skins.
- Supports classic and slim/Alex player models.
- Renders outer layers such as hat, jacket, sleeves, and pants.
- Supports local skin and cape PNG uploads in the viewer UI.
- Exports the current skin head as a pixel-perfect `180x180` PNG.
- Loads official skins and capes by Minecraft username or UUID when served by the backend.
- Keeps pixel-art textures sharp with nearest-neighbor filtering.
- Provides orbit, zoom, resize-aware rendering, and optional experimental walk animation.
- Exposes `GET /helm/{player}` for simple 2D avatar PNG generation.

## Quick Start

Requirements:

- Stable Rust.
- `wasm32-unknown-unknown` target.
- `wasm-pack`.
- Node.js and npm if you plan to develop or rebuild the browser UI.
- A browser with WebGL2 support.
- Network access to Mojang APIs and texture URLs for official player loading.

Install the WASM target if needed:

```sh
rustup target add wasm32-unknown-unknown
```

Build the WebAssembly viewer package:

```sh
wasm-pack build crates/skinhelm-wasm --target web
```

Run the backend with the embedded viewer:

```sh
cargo run -p skinhelm --features wasm-viewer
```

Open:

```text
http://localhost:3000/viewer/
```

You can also open the viewer with an official player skin preloaded:

```text
http://localhost:3000/viewer/Steve
```

```text
http://localhost:3000/viewer/bc881e0292164f6ea80f7b9df0ccf9e9
```

In that flow, the backend resolves the player, downloads the skin, detects whether the model is `classic` or `slim`, and loads an official cape when Mojang provides one.

## Local Viewer Development

For browser UI development:

```sh
cd crates/skinhelm-wasm
wasm-pack build --target web
npm install
npm run dev
```

Vite serves the viewer on `127.0.0.1`; use the URL printed in the terminal.

For a production browser build:

```sh
cd crates/skinhelm-wasm
wasm-pack build --target web
npm install
npm run build
```

## Workspace Overview

The workspace is split into three crates with clear responsibilities:

- `crates/skinhelm-wasm` owns the browser experience: the WebAssembly/WebGL2 viewer, the viewer UI, skin and cape loading, 3D model rendering, camera controls, and animation.
- `crates/skinhelm` owns the HTTP surface: the Axum server, viewer routes, embedded WASM assets, Mojang-backed skin/cape responses, health checks, and the 2D avatar endpoint.
- `crates/skinhelm-core` owns the shared domain logic: Mojang API access, player validation, caching, common error types, and 2D PNG rendering.

## HTTP Routes

When running with `--features wasm-viewer`:

- `GET /viewer/`: serves the 3D viewer.
- `GET /viewer/{player}`: serves the viewer and loads a username or UUID.
- `GET /viewer/skin/{player}`: returns the official skin PNG plus viewer metadata headers.
- `GET /viewer/cape/{player}`: returns the official cape PNG when available.

Always available:

- `GET /health`: returns `{ "status": "ok" }`.
- `GET /helm/{player}`: returns a 2D helmeted-head PNG.

`/helm/{player}` accepts a Minecraft username, a hyphenated UUID, or a compact UUID. It also accepts an optional `size` query parameter:

- Default: `184`.
- Minimum: `8`.
- Maximum: `512`.
- Non-multiple sizes are rounded to the nearest multiple of `8`, so each Minecraft head pixel renders as an equal square.

Examples:

```sh
curl -o head.png "http://localhost:3000/helm/bc881e0292164f6ea80f7b9df0ccf9e9"
curl -o steve.png "http://localhost:3000/helm/Steve?size=256"
curl "http://localhost:3000/health"
```

## Caching

The backend uses an in-memory cache for Mojang lookups and rendered 2D PNGs:

- Username to UUID: 6 hours.
- UUID to skin URL: 30 minutes.
- Skin URL plus output size to rendered PNG: 30 minutes.

There is no external cache dependency; cached data is cleared when the process restarts.

## Quality Commands

```sh
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo clippy -p skinhelm-wasm --target wasm32-unknown-unknown -- -D warnings
```

## Known Limitations

- Elytra rendering is not implemented.
