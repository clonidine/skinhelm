# skinhelm-wasm

`skinhelm-wasm` is the WebAssembly/WebGL viewer crate in the `skinhelm` workspace. It renders a complete Minecraft player model in the browser from a local skin PNG, including walk animation, mouse orbit controls, zoom, and toggleable outer layers.

## Workspace Context

This crate lives at `crates/skinhelm-wasm` and is independent from the existing `skinhelm` HTTP backend crates. It does not implement Axum endpoints, Mojang API access, username resolution, or cache behavior.

## Features

- Rust + WebAssembly viewer built with `wasm-bindgen`.
- Direct WebGL2 rendering through `web-sys`.
- Local PNG file loading from the browser.
- Generated default placeholder skin.
- Full classic Minecraft player model: head, body, arms, and legs.
- Modern `64x64` skins with hat, jacket, sleeves, and pants layers.
- Basic legacy `64x32` skin fallback.
- Nearest-neighbor texture sampling for pixel-art skins.
- Mouse drag orbit controls.
- Scroll wheel zoom.
- Walking animation with pause/resume and speed control.
- Toggle for outer overlay layers.

## Requirements

- Stable Rust.
- `wasm-pack`.
- Node.js and npm.
- A browser with WebGL2 support.

If the WASM target is missing:

```sh
rustup target add wasm32-unknown-unknown
```

## Why WASM/WebGL

The viewer keeps the model, animation, UV generation, PNG decoding, and rendering control in Rust while using WebGL2 directly in the browser. This avoids a backend rendering step and avoids large browser-side 3D engines.

## Build From Workspace Root

```sh
cargo fmt
cargo clippy -p skinhelm-wasm --target wasm32-unknown-unknown -- -D warnings
cargo test -p skinhelm-wasm
wasm-pack build crates/skinhelm-wasm --target web
```

## Build From This Crate

```sh
wasm-pack build --target web
npm install
npm run build
```

## Run Locally

From `crates/skinhelm-wasm`:

```sh
wasm-pack build --target web
npm install
npm run dev
```

Then open the Vite URL shown in the terminal.

## Project Structure

- `src/lib.rs`: exported WASM API.
- `src/app.rs`: DOM integration and viewer state.
- `src/webgl.rs`: WebGL2 program, texture upload, buffers, and draw calls.
- `src/math.rs`: small vector and matrix library.
- `src/model.rs`: Minecraft cuboid geometry and UV mapping.
- `src/skin.rs`: PNG decoding, dimension validation, and generated default skin.
- `src/animation.rs`: walk pose calculation.
- `src/controls.rs`: orbit camera controls.
- `src/error.rs`: small custom error type.
- `bootstrap.js`: minimal JavaScript bootstrap and DOM event wiring.
- `index.html`: browser UI.

## UI Controls

- `skin-file`: choose a local PNG skin.
- `load-default`: load the generated placeholder skin.
- `toggle-animation`: pause or resume walking animation.
- `toggle-overlays`: show or hide hat, jacket, sleeves, and pants overlays.
- `animation-speed`: adjust walk speed from `0.1` to `3.0`.
- Mouse drag: rotate the view.
- Mouse wheel over the canvas: zoom.

## Skin Format Support

- Accepts `64x64` modern Minecraft skins.
- Accepts `64x32` legacy skins with a basic fallback.
- Rejects other dimensions with a visible status message.
- Slim/Alex arm geometry is not implemented in this first version.

## Minecraft Model Dimensions

- Head: `8 x 8 x 8`, centered at `y = 24`.
- Body: `8 x 12 x 4`, centered at `y = 14`.
- Right arm: `4 x 12 x 4`, centered at `x = -6`, `y = 14`.
- Left arm: `4 x 12 x 4`, centered at `x = 6`, `y = 14`.
- Right leg: `4 x 12 x 4`, centered at `x = -2`, `y = 6`.
- Left leg: `4 x 12 x 4`, centered at `x = 2`, `y = 6`.

The origin is centered between the feet, Y is up, and the player initially faces the camera.

## UV Mapping Overview

UV regions are generated manually from Minecraft Java skin pixel coordinates. Image coordinates are top-left based, while WebGL UV coordinates are bottom-left based, so `skinhelm-wasm` converts `y` with:

```text
top = 1.0 - y / skin_height
bottom = 1.0 - (y + height) / skin_height
```

Texture filters are set to `NEAREST`, and wrapping is set to `CLAMP_TO_EDGE`.

## Animation Behavior

Walking uses a simple sine phase:

```text
phase = elapsed_seconds * speed * 4.0
right_leg_x = sin(phase) * 0.7
left_leg_x = -sin(phase) * 0.7
right_arm_x = -sin(phase) * 0.7
left_arm_x = sin(phase) * 0.7
```

The body gets a subtle bob while walking.

## Known Limitations

- Classic-width arms only; no slim/Alex toggle yet.
- No cape rendering.
- No elytra rendering.
- No idle, run, or sneak pose variants.
- No PNG snapshot export.
- No direct Mojang or backend lookup in this crate.

## Future Ideas

- Slim/Alex model toggle.
- Cape rendering.
- Elytra rendering.
- Idle animation.
- Run animation.
- Sneak pose.
- Export PNG snapshot.
- Integration with the existing `skinhelm` backend.
- Loading by UUID or username through backend proxy.
