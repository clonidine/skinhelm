pub mod animation;
pub mod error;
pub mod math;
pub mod model;
pub mod skin;

#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod controls;
#[cfg(target_arch = "wasm32")]
mod webgl;

#[cfg(target_arch = "wasm32")]
use app::ViewerApp;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct SkinhelmViewer {
    app: ViewerApp,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl SkinhelmViewer {
    #[wasm_bindgen(js_name = init)]
    pub fn init() -> Result<SkinhelmViewer, JsValue> {
        console_error_panic_hook::set_once();
        Ok(Self {
            app: ViewerApp::new("viewer-canvas", "status")?,
        })
    }

    pub fn load_skin_bytes(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.app.load_skin_bytes(bytes)
    }

    pub fn load_default_skin(&mut self) -> Result<(), JsValue> {
        self.app.load_default_skin()
    }

    pub fn set_animation_enabled(&mut self, enabled: bool) {
        self.app.set_animation_enabled(enabled);
    }

    pub fn set_overlays_enabled(&mut self, enabled: bool) {
        self.app.set_overlays_enabled(enabled);
    }

    pub fn set_animation_speed(&mut self, speed: f32) {
        self.app.set_animation_speed(speed);
    }

    pub fn resize(&mut self) -> Result<(), JsValue> {
        self.app.resize()
    }

    pub fn render_frame(&mut self, timestamp_ms: f64) -> Result<(), JsValue> {
        self.app.render_frame(timestamp_ms)
    }

    pub fn pointer_down(&mut self, x: f32, y: f32) {
        self.app.pointer_down(x, y);
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) {
        self.app.pointer_move(x, y);
    }

    pub fn pointer_up(&mut self) {
        self.app.pointer_up();
    }

    pub fn wheel(&mut self, delta_y: f32) {
        self.app.wheel(delta_y);
    }
}
