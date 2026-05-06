# skinhelm-core

`skinhelm-core` is the domain library for the `skinhelm` workspace. It contains the shared logic for validating player input, calling Mojang APIs, downloading textures, rendering the 2D face of a Minecraft skin as PNG, and caching repeated work in memory.

This crate does not expose an HTTP server or a WebAssembly UI. It is used by the other workspace crates to keep business rules separate from transport, routing, and presentation concerns.

## Overview

The main flow supported by this crate is:

1. Receive a player as a Minecraft username, a hyphenated UUID, or a compact UUID.
2. Validate and normalize the input.
3. Resolve usernames to UUIDs through the Mojang API.
4. Fetch the session profile and decode its `textures` property.
5. Download the skin PNG.
6. Crop the front face, composite the helmet/hat layer when available, and return a square PNG.

Rendering uses the standard Minecraft skin coordinates:

- Base face: `x = 8`, `y = 8`, `width = 8`, `height = 8`.
- Helmet/hat overlay: `x = 40`, `y = 8`, `width = 8`, `height = 8`.

Modern `64x64` skins get alpha compositing for the overlay layer when it contains visible pixels. Legacy `64x32` skins are rendered from the base face only. The final image is resized with nearest-neighbor filtering and normalized to a multiple of `8` so every Minecraft head pixel becomes an equal square.

## Public API and Main Types

Public modules are exposed from `lib.rs`:

- `cache`: in-memory cache backed by `tokio::sync::RwLock`.
- `error`: shared domain error type.
- `mojang`: HTTP client for Mojang APIs and texture URLs.
- `render`: PNG rendering for the skin head.
- `types`: shared response types, validation helpers, and constants.

### `types`

Important items:

- `PlayerInput`: enum with `Uuid(String)` and `Username(String)`.
- `SkinProfile`: skin URL, `slim` model flag, and optional cape URL.
- `DEFAULT_SIZE`: `184`.
- `MIN_SIZE`: `8`.
- `MAX_SIZE`: `512`.
- `parse_player(value)`: accepts a valid username, hyphenated UUID, or compact UUID.
- `normalize_uuid(value)`: converts a valid UUID to compact lowercase form without hyphens.
- `is_username_valid(value)`: validates Minecraft usernames from `3` to `16` ASCII alphanumeric characters or `_`.
- `parse_size(value)`: validates the optional output size, using `DEFAULT_SIZE` when missing and rounding provided sizes to the nearest multiple of `8`.

### `mojang::MojangClient`

`MojangClient` wraps a `reqwest::Client` configured with a 10-second timeout and the `skinhelm/0.1.0` user agent.

Main methods:

- `new()`: creates the HTTP client.
- `resolve_username(username)`: resolves a username to a compact UUID.
- `fetch_skin_url(uuid)`: returns only the skin URL.
- `fetch_skin_profile(uuid)`: returns `SkinProfile`, including slim/classic metadata and an optional cape URL.
- `download_skin(skin_url)`: downloads skin PNG bytes.
- `download_cape(cape_url)`: downloads cape PNG bytes.

The client calls:

- `https://api.mojang.com/users/profiles/minecraft/{username}`
- `https://sessionserver.mojang.com/session/minecraft/profile/{uuid}?unsigned=false`

### `render`

`render_helm_png(skin_png, size)` receives skin PNG bytes and returns a square PNG at the nearest Minecraft-pixel-safe size.

It validates that the image can be decoded and that it is large enough to contain the base face. Invalid image data returns `AppError::InvalidSkinImage`.

### `cache::AppCache`

`AppCache` stores three in-memory caches:

- Username to UUID: 6-hour TTL.
- UUID to skin URL: 30-minute TTL.
- Skin URL plus output size to rendered PNG: 30-minute TTL.

Expired entries are cleaned lazily during reads and writes. The cache is process-local and does not persist across restarts.

### `error::AppError`

`AppError` represents expected domain failures:

- validation: `InvalidPlayer`, `InvalidSize`;
- missing data: `UsernameNotFound`, `ProfileHasNoSkin`, `ProfileHasNoCape`;
- external failures: `MojangRequest`, `SkinDownload`, `CapeDownload`;
- invalid upstream data: `MalformedUpstream`, `TextureDecode`, `TextureJson`, `InvalidSkinImage`;
- internal failures: `Internal`.

The type provides helpers for HTTP or API layers:

- `message()`: stable short error message.
- `is_bad_request()`: errors that commonly map to HTTP `400`.
- `is_not_found()`: errors that commonly map to HTTP `404`.
- `is_internal()`: errors that commonly map to HTTP `500`.

## Basic Examples

### Validate player input and size

```rust
use skinhelm_core::error::AppError;
use skinhelm_core::types::{parse_player, parse_size, PlayerInput};

fn parse_request_parts(player: &str, size: Option<&str>) -> Result<(PlayerInput, u32), AppError> {
    let player = parse_player(player)?;
    let size = parse_size(size)?;
    Ok((player, size))
}
```

### Resolve, download, and render a head

```rust
use skinhelm_core::mojang::MojangClient;
use skinhelm_core::render::render_helm_png;
use skinhelm_core::types::{parse_player, PlayerInput};

async fn render_player_head(player: &str, size: u32) -> Result<Vec<u8>, skinhelm_core::error::AppError> {
    let client = MojangClient::new()?;

    let uuid = match parse_player(player)? {
        PlayerInput::Uuid(uuid) => uuid,
        PlayerInput::Username(username) => client.resolve_username(&username).await?,
    };

    let skin_url = client.fetch_skin_url(&uuid).await?;
    let skin_png = client.download_skin(&skin_url).await?;

    render_helm_png(&skin_png, size)
}
```

### Use the application cache

```rust
use skinhelm_core::cache::AppCache;

async fn remember_render(cache: &AppCache, skin_url: &str, size: u32, png: Vec<u8>) {
    cache.set_rendered_png(skin_url, size, png).await;

    if let Some(cached) = cache.get_rendered_png(skin_url, size).await {
        assert!(!cached.is_empty());
    }
}
```

## Build and Test Commands

Run from the workspace root:

```sh
cargo fmt
cargo clippy -p skinhelm-core -- -D warnings
cargo test -p skinhelm-core
cargo build -p skinhelm-core
```

To test the full workspace:

```sh
cargo test
```

## Relationship with Other Workspace Crates

- `crates/skinhelm`: Axum HTTP server. It uses `skinhelm-core` to validate routes, resolve players, download skins, render PNGs, and map `AppError` values to HTTP responses.
- `crates/skinhelm-wasm`: WebAssembly/WebGL browser viewer for 3D skin viewing. It covers the interactive frontend experience while `skinhelm-core` keeps the backend domain logic.
- `crates/skinhelm-core`: shared core with no dependency on the HTTP or WASM crates.

This separation makes validation, Mojang integration, caching, and rendering logic testable without starting the server.

## Development Notes

- Keep shared, transport-independent domain rules in this crate.
- Avoid coupling the core crate to Axum, HTTP headers, static files, or UI details.
- When adding new validation or failure cases, prefer returning `AppError` so the `skinhelm` server can keep centralized error mapping.
- Current rendering outputs only the flat 2D front face with optional helmet/hat overlay. It does not render 3D geometry, lighting, perspective, or smoothing.
- The cache is simple and process-local. Distributed or persistent caching should live in a higher layer or be introduced behind an explicit abstraction.
- Mojang API calls and texture downloads depend on network access and upstream availability. Existing unit tests focus on local validation and rendering behavior.
