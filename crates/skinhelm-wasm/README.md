# skinhelm-wasm

`skinhelm-wasm` is the WebAssembly/WebGL crate in the `skinhelm` workspace. It renders a full 3D Minecraft player model in the browser from a skin PNG, with support for classic or slim models, outer layers, an optional cape, orbit/zoom controls, and experimental walking animation.

The crate is designed as a browser viewer: Rust owns PNG decoding, dimension validation, mesh generation, UV mapping, pose calculation, and rendering, while `bootstrap.js` connects the generated WASM module to the DOM, local files, and `requestAnimationFrame`.

## Overview

Main capabilities:

- WebAssembly build with `wasm-bindgen`.
- Direct WebGL2 rendering through `web-sys`, without an external 3D engine.
- Generated default skin so the viewer can start without a local file.
- Local upload of modern `64x64` and legacy `64x32` skin PNGs.
- Classic 4 px arms and slim/Alex 3 px arms.
- Minecraft cuboid rendering for head, body, arms, and legs.
- Optional outer layers: hat, jacket, sleeves, and pants.
- Local upload of `64x32` capes or compatible `64x64` cape PNGs.
- Skin, model type, and official cape loading when served by the `skinhelm` backend.
- `NEAREST` texture filtering and `CLAMP_TO_EDGE` wrapping for pixel-art skins.
- Mouse orbit controls and wheel zoom.
- Experimental walking animation, disabled by default, with speed control.
- Query-string debug modes for inspecting the head, overlays, full model, and lighting.

## Workspace Relationship

The workspace has three main crates:

- `crates/skinhelm`: Axum HTTP server. It serves `/health`, `/helm/{player}`, and, with the `wasm-viewer` feature, the WASM viewer.
- `crates/skinhelm-core`: shared domain logic, Mojang client, cache, validation, types, and 2D helmeted head PNG rendering.
- `crates/skinhelm-wasm`: 3D WebAssembly browser viewer.

This crate does not resolve usernames, call Mojang APIs directly, or implement HTTP routes. That work belongs to the `skinhelm` backend and `skinhelm-core`. When the server is compiled with `wasm-viewer`, it embeds this crate's `index.html`, `bootstrap.js`, `pkg/skinhelm_wasm.js`, and `pkg/skinhelm_wasm_bg.wasm` outputs.

To serve the viewer through the backend:

```sh
cargo run -p skinhelm --features wasm-viewer
```

Then open:

```text
http://localhost:3000/viewer
```

The viewer can also be opened with a UUID path. In that flow, the backend fetches the Mojang profile, downloads the skin, reports whether the model is `slim` or `classic` through the `x-skinhelm-model` header, and reports official cape availability through the `x-skinhelm-cape` header:

```text
http://localhost:3000/viewer/bc881e0292164f6ea80f7b9df0ccf9e9
```

## wasm-bindgen and JavaScript Usage

The JavaScript API is exported from `src/lib.rs` through the `SkinhelmViewer` struct.

Minimal usage:

```js
import init, { SkinhelmViewer } from "./pkg/skinhelm_wasm.js";

await init();

const viewer = SkinhelmViewer.init();
viewer.resize();
viewer.load_default_skin();

function frame(timestamp) {
  viewer.render_frame(timestamp);
  requestAnimationFrame(frame);
}

requestAnimationFrame(frame);
```

The HTML page must provide the elements expected by `SkinhelmViewer.init()`:

```html
<canvas id="viewer-canvas"></canvas>
<span id="status"></span>
```

Important exported methods:

- `SkinhelmViewer.init()`: initializes WebGL2, viewer state, and the default skin.
- `load_skin_bytes(bytes)`: loads a classic skin from PNG bytes.
- `load_skin_bytes_with_model(bytes, slim)`: loads a PNG skin as either classic or slim.
- `load_default_skin()`: restores the generated default skin.
- `load_cape_bytes(bytes)`: loads a cape PNG.
- `clear_cape()`: removes the loaded cape.
- `set_cape_visible(visible)`: shows or hides the cape when one is loaded.
- `set_animation_enabled(enabled)`: enables or disables experimental animation.
- `set_overlays_enabled(enabled)`: shows or hides outer layers.
- `set_animation_speed(speed)`: adjusts animation speed, clamped internally to `0.1..=3.0`.
- `set_debug_mode(mode)`: enables a named debug rendering mode.
- `set_preset(preset)`: applies the camera preset; currently unknown values resolve to `default`.
- `resize()`: synchronizes canvas pixel size with CSS size and `devicePixelRatio`.
- `render_frame(timestamp_ms)`: renders one frame and advances animation time.
- `pointer_down(x, y)`, `pointer_move(x, y)`, `pointer_up()`: orbit controls.
- `wheel(delta_y)`: zoom control.

`bootstrap.js` contains a complete integration with the UI in `index.html`, including local file reads through `arrayBuffer()`, animation controls, and UUID loading when the current path matches `/viewer/{uuid}`.

## Build and Test Commands

Requirements:

- Stable Rust.
- `wasm32-unknown-unknown` target.
- `wasm-pack`.
- Node.js and npm.
- A browser with WebGL2 support.

If the WASM target is not installed yet:

```sh
rustup target add wasm32-unknown-unknown
```

Useful commands from the workspace root:

```sh
cargo fmt
cargo clippy -p skinhelm-wasm --target wasm32-unknown-unknown -- -D warnings
cargo test -p skinhelm-wasm
wasm-pack build crates/skinhelm-wasm --target web
```

Commands from `crates/skinhelm-wasm`:

```sh
wasm-pack build --target web
npm install
npm run build
```

For local Vite development:

```sh
wasm-pack build --target web
npm install
npm run dev
```

`npm run dev` starts Vite on host `127.0.0.1`; open the URL shown in the terminal.

## Crate Structure

- `src/lib.rs`: API exported through `wasm-bindgen`.
- `src/app.rs`: DOM integration, viewer state, status messages, and render loop entry points.
- `src/webgl.rs`: WebGL2 context setup, shaders, buffers, texture uploads, and draw calls.
- `src/model.rs`: Minecraft cuboid meshes, pivots, dimensions, UVs, and cape mesh.
- `src/skin.rs`: PNG decoding, dimension validation, and generated default skin.
- `src/animation.rs`: static pose, walking pose, and cape rotation helpers.
- `src/cape.rs`: pure loaded/visible cape state.
- `src/controls.rs`: camera, orbit controls, zoom, projection, and debug metrics.
- `src/math.rs`: small vector and matrix helpers used by the renderer.
- `src/error.rs`: viewer errors converted into JavaScript-facing messages.
- `bootstrap.js`: high-level JavaScript glue for UI events and UUID loading.
- `index.html`: viewer interface used by Vite and the backend.
- `pkg/`: `wasm-pack` output; embedded by the backend when `wasm-viewer` is enabled.
- `dist/`: `npm run build` output.

## UI Controls

The interface in `index.html` uses these IDs, consumed by `bootstrap.js`:

- `skin-file`: selects a local skin PNG.
- `cape-file`: selects a local cape PNG.
- `load-default`: loads the generated default skin.
- `toggle-overlays`: shows or hides outer layers.
- `toggle-cape`: shows or hides an already loaded cape.
- `toggle-slim`: uses slim arms for local PNG files.
- `toggle-animation`: enables or disables experimental animation.
- `animation-speed`: adjusts speed between `0.1` and `3.0`.
- `dock-toggle`: shows or hides the control dock.
- Mouse drag on the canvas: orbits the camera.
- Mouse wheel on the canvas: zooms in or out.

## Debug Modes

Add `debug` to the query string to force rendering modes:

```text
?debug=solid-head
?debug=head
?debug=head-textured
?debug=head-overlay
?debug=base-only
?debug=full-no-overlays
?debug=full-overlays
?debug=unlit
```

The `preset` mode exists in the API and URL, but currently all values resolve to `default`:

```text
?preset=default
```

## Supported Formats

Skins:

- `64x64`: modern format, with full second-layer support.
- `64x32`: legacy format, with a basic fallback and without modern body, arm, and leg outer layers.
- Other dimensions are rejected with a visible message in the `status` element.

Capes:

- `64x32`: classic cape layout.
- `64x64`: accepted when the classic cape region is kept in the upper-left area.
- Other dimensions are rejected.

## Development Notes

- `webgl.rs` compiles GLSL ES 3.00 shaders at runtime and uses WebGL2 only.
- The renderer uploads textures as raw RGBA decoded by the `image` crate.
- `UNPACK_FLIP_Y_WEBGL` is not used; UV coordinates follow Minecraft image pixels with the origin at the top-left.
- Each UV rectangle is inset by half a texel to reduce visual bleeding between adjacent skin regions.
- Outer layers use a very low alpha discard threshold and are drawn with `BLEND`, `POLYGON_OFFSET_FILL`, and `depth_mask(false)`.
- The cape uses a separate texture from the skin and writes to the depth buffer before overlays.
- Walking animation is intentionally simple: arms and legs swing with sine waves, the body has a subtle bob, and the cape receives a reactive correction when visible.
- Most pure logic has native Rust tests even though DOM/WebGL-dependent modules compile only for `wasm32`.
- Always run `wasm-pack build --target web` before testing in the browser or serving through the backend, because `skinhelm` embeds the files in `pkg/`.

## Known Limitations

- No elytra rendering.
- No PNG snapshot export.
- No direct username lookup in this crate; official UUID loading depends on the `skinhelm` backend.
- The UI does not yet expose full idle, running, or sneaking poses.
- Local capes outside the classic layout are not supported.
