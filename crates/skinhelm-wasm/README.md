# skinhelm-wasm

`skinhelm-wasm` is the WebAssembly/WebGL viewer crate in the `skinhelm` workspace. It renders a complete Minecraft player model in the browser from a local skin PNG, including optional experimental walk animation, mouse orbit controls, zoom, toggleable outer layers, and official cape rendering when served through the backend.

## Workspace Context

This crate lives at `crates/skinhelm-wasm` and is independent from the existing `skinhelm` HTTP backend crates. It does not implement Axum endpoints, Mojang API access, username resolution, or cache behavior.

The `skinhelm` backend can optionally serve this viewer when built with its `wasm-viewer` feature:

```sh
cargo run -p skinhelm --features wasm-viewer
```

The viewer is then available at `http://localhost:3000/viewer`.

The backend also supports opening the viewer with a UUID path, which loads that player's skin, model type, and official cape when Mojang exposes one:

```text
http://localhost:3000/viewer/bc881e0292164f6ea80f7b9df0ccf9e9
```

## Features

- Rust + WebAssembly viewer built with `wasm-bindgen`.
- Direct WebGL2 rendering through `web-sys`.
- Local skin and cape PNG file loading from the browser.
- Generated default placeholder skin.
- Full classic Minecraft player model: head, body, arms, and legs.
- Modern `64x64` skins with hat, jacket, sleeves, and pants layers.
- Basic legacy `64x32` skin fallback.
- Classic and slim/Alex arm geometry.
- Cape rendering from a local PNG or from the `skinhelm` backend UUID viewer path.
- Nearest-neighbor texture sampling for pixel-art skins.
- Mouse drag orbit controls.
- Scroll wheel zoom.
- Experimental walking animation with pause/resume and speed control. It is disabled by default.
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
- `src/animation.rs`: walk and cape pose calculation.
- `src/cape.rs`: pure cape visibility state.
- `src/controls.rs`: orbit camera controls.
- `src/error.rs`: small custom error type.
- `bootstrap.js`: minimal JavaScript bootstrap and DOM event wiring.
- `index.html`: browser UI.

## UI Controls

- `skin-file`: choose a local PNG skin.
- `cape-file`: choose a local PNG cape.
- `load-default`: load the generated placeholder skin.
- `toggle-animation`: enable or disable experimental walking animation. It is off by default.
- `toggle-overlays`: show or hide hat, jacket, sleeves, and pants overlays.
- `toggle-cape`: show or hide a loaded cape.
- `toggle-slim`: use 3px slim arms for local PNG files.
- `animation-speed`: adjust walk speed from `0.1` to `3.0`.
- Mouse drag: rotate the view.
- Mouse wheel over the canvas: zoom.

## Debug Rendering Modes

Add one of these query strings to the viewer URL:

- `?debug=solid-head`
- `?debug=head-textured`
- `?debug=head-overlay`
- `?debug=full-no-overlays`
- `?debug=full-overlays`
- `?debug=unlit`

## Viewer Presets

- `?preset=default`

Only the `default` preset is exposed. Any other `preset` query value is ignored and resolves to `default`.

## Skin Format Support

- Accepts `64x64` modern Minecraft skins.
- Accepts `64x32` legacy skins with a basic fallback.
- Rejects other dimensions with a visible status message.
- Supports classic 4px arms and slim/Alex 3px arms.
- Local capes accept `64x32` classic Minecraft cape PNGs and `64x64` cape PNGs that keep the classic cape region in the upper-left area.
- Official capes are loaded from `textures.CAPE.url` when using the backend UUID viewer path.

## Minecraft Model Dimensions

- Head: `8 x 8 x 8`, spanning `y = 24..32`, centered at `y = 28`.
- Body: `8 x 12 x 4`, spanning `y = 12..24`, centered at `y = 18`.
- Right arm classic: `4 x 12 x 4`, centered at `x = -6`, `y = 18`.
- Left arm classic: `4 x 12 x 4`, centered at `x = 6`, `y = 18`.
- Right arm slim: `3 x 12 x 4`, centered at `x = -5.5`, `y = 18`.
- Left arm slim: `3 x 12 x 4`, centered at `x = 5.5`, `y = 18`.
- Right leg: `4 x 12 x 4`, spanning `y = 0..12`, centered at `x = -2`, `y = 6`.
- Left leg: `4 x 12 x 4`, spanning `y = 0..12`, centered at `x = 2`, `y = 6`.

The origin is centered between the feet, Y is up, the full player is 32 pixels tall, and the player initially faces the camera.

## UV Mapping Overview

UV regions are generated manually from Minecraft Java skin pixel coordinates. The PNG is decoded in Rust and uploaded to WebGL from a raw RGBA buffer without `UNPACK_FLIP_Y_WEBGL`, so row `0` in the uploaded texture is the skin's top row. `skinhelm-wasm` therefore keeps Minecraft's top-left pixel coordinates in normalized texture space:

```text
top = y / skin_height
bottom = (y + height) / skin_height
```

Texture filters are set to `NEAREST`, and wrapping is set to `CLAMP_TO_EDGE`.
In code, each edge is inset by half a texel:

```text
left = (x + 0.5) / skin_width
right = (x + width - 0.5) / skin_width
top = (y + 0.5) / skin_height
bottom = (y + height - 0.5) / skin_height
```

That keeps the nearest-neighbor sampler inside each Minecraft skin region and prevents adjacent pixels from leaking along arm or leg seams.

Capes use the standard cape texture regions and render as a thin `10 x 16 x 1` cuboid behind the body. The cape texture is uploaded separately from the skin texture with nearest-neighbor filtering and alpha-cutout support. The cape writes to the depth buffer so limbs and overlays behind it do not bleed through from certain camera angles.

## Animation Behavior

Walking uses a simple sine phase:

```text
phase = elapsed_seconds * speed * 4.0
right_leg_x = sin(phase) * 0.42
left_leg_x = -sin(phase) * 0.42
right_arm_x = -sin(phase) * 0.55
left_arm_x = sin(phase) * 0.55
cape_x = sin(phase / 1.5) * 0.06 + PI * 0.06
```

Animation is experimental and disabled by default. The body gets a very subtle bob while walking, and the swing is intentionally conservative to reduce limb intersections. When animation is paused, the cape stays in its default rest angle. The pure cape helpers also include idle and running formulas for future viewer states:

```text
idle cape_x = sin(t) * 0.01 + PI * 0.06
running cape_x = sin(t * 2.0) * 0.1 + PI * 0.3
```

When a cape is visible, walking uses a reactive variant of the same pose: arm and leg swing remains unchanged, and backward limb swing adds a small contact push to the cape rotation. This is a simple collision-style correction, not cloth simulation.

## Known Limitations

- Local cape loading currently supports the classic `64x32` layout and compatible `64x64` images; nonstandard cape atlases are rejected.
- No elytra rendering.
- No full idle, run, or sneak player pose variants in the UI yet.
- No PNG snapshot export.
- No direct Mojang lookup in this crate; UUID loading is handled by the optional `skinhelm` backend viewer routes.

## Future Ideas

- Elytra rendering.
- Idle animation.
- Run animation.
- Sneak pose.
- Export PNG snapshot.
- Username loading through the backend proxy.
