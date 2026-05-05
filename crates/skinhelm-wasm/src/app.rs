use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, HtmlElement};

use crate::animation::{walk_pose, WalkPose};
use crate::controls::OrbitControls;
use crate::error::ViewerError;
use crate::math::Mat4;
use crate::skin::{decode_png, default_skin};
use crate::webgl::Renderer;

pub struct ViewerApp {
    renderer: Renderer,
    controls: OrbitControls,
    status: HtmlElement,
    animation_enabled: bool,
    overlays_enabled: bool,
    animation_speed: f32,
    elapsed_seconds: f32,
    last_timestamp_ms: Option<f64>,
}

impl ViewerApp {
    pub fn new(canvas_id: &'static str, status_id: &'static str) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| js_error("missing window"))?;
        let document = window
            .document()
            .ok_or_else(|| js_error("missing document"))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(ViewerError::MissingElement(canvas_id))
            .map_err(to_js_error)?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| ViewerError::MissingElement(canvas_id))
            .map_err(to_js_error)?;
        let status = document
            .get_element_by_id(status_id)
            .ok_or(ViewerError::MissingElement(status_id))
            .map_err(to_js_error)?
            .dyn_into::<HtmlElement>()
            .map_err(|_| ViewerError::MissingElement(status_id))
            .map_err(to_js_error)?;

        let mut app = Self {
            renderer: Renderer::new(canvas).map_err(to_js_error)?,
            controls: OrbitControls::new(),
            status,
            animation_enabled: true,
            overlays_enabled: true,
            animation_speed: 1.0,
            elapsed_seconds: 0.0,
            last_timestamp_ms: None,
        };
        app.load_default_skin()?;
        Ok(app)
    }

    pub fn load_skin_bytes(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        match decode_png(bytes).and_then(|skin| {
            let message = format!("Loaded skin: {}x{}", skin.width, skin.height);
            self.renderer.upload_skin(&skin)?;
            Ok(message)
        }) {
            Ok(message) => {
                self.set_status(&message);
                Ok(())
            }
            Err(error) => {
                let message = error.message();
                self.set_status(&message);
                Err(to_js_error(error))
            }
        }
    }

    pub fn load_default_skin(&mut self) -> Result<(), JsValue> {
        let skin = default_skin();
        self.renderer.upload_skin(&skin).map_err(to_js_error)?;
        self.set_status("Loaded default generated skin");
        Ok(())
    }

    pub fn set_animation_enabled(&mut self, enabled: bool) {
        self.animation_enabled = enabled;
    }

    pub fn set_overlays_enabled(&mut self, enabled: bool) {
        self.overlays_enabled = enabled;
    }

    pub fn set_animation_speed(&mut self, speed: f32) {
        self.animation_speed = speed.clamp(0.1, 3.0);
    }

    pub fn resize(&mut self) -> Result<(), JsValue> {
        self.renderer.resize();
        Ok(())
    }

    pub fn render_frame(&mut self, timestamp_ms: f64) -> Result<(), JsValue> {
        if let Some(last) = self.last_timestamp_ms {
            let delta = ((timestamp_ms - last) / 1000.0).clamp(0.0, 0.1) as f32;
            if self.animation_enabled {
                self.elapsed_seconds += delta;
            }
        }
        self.last_timestamp_ms = Some(timestamp_ms);

        let phase = self.elapsed_seconds * self.animation_speed * 4.0;
        let pose = if self.animation_enabled {
            walk_pose(phase)
        } else {
            WalkPose {
                right_leg_x: 0.0,
                left_leg_x: 0.0,
                right_arm_x: 0.0,
                left_arm_x: 0.0,
                body_y: 0.0,
                head_y: 0.0,
            }
        };

        let width = self.renderer_width().max(1.0);
        let height = self.renderer_height().max(1.0);
        let projection = Mat4::perspective(45.0_f32.to_radians(), width / height, 0.1, 200.0);
        self.renderer
            .render(
                self.controls.view_matrix(),
                projection,
                pose,
                self.overlays_enabled,
            )
            .map_err(to_js_error)
    }

    pub fn pointer_down(&mut self, x: f32, y: f32) {
        self.controls.pointer_down(x, y);
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) {
        self.controls.pointer_move(x, y);
    }

    pub fn pointer_up(&mut self) {
        self.controls.pointer_up();
    }

    pub fn wheel(&mut self, delta_y: f32) {
        self.controls.zoom(delta_y);
    }

    fn renderer_width(&self) -> f32 {
        web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("viewer-canvas"))
            .and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok())
            .map(|canvas| canvas.width() as f32)
            .unwrap_or(1.0)
    }

    fn renderer_height(&self) -> f32 {
        web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("viewer-canvas"))
            .and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok())
            .map(|canvas| canvas.height() as f32)
            .unwrap_or(1.0)
    }

    fn set_status(&self, message: &str) {
        self.status.set_text_content(Some(message));
    }
}

fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}

fn to_js_error(error: ViewerError) -> JsValue {
    JsValue::from_str(&error.message())
}
