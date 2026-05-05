use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, HtmlElement};

use crate::animation::{cape_reactive_walk_pose, static_pose, walk_pose};
use crate::cape::CapeRenderState;
#[cfg(debug_assertions)]
use crate::controls::{debug_head_front_view_matrix_for_preset, projected_bounds_metrics};
use crate::controls::{presentation_matrix, profile_projection, OrbitControls, ViewerPreset};
use crate::error::ViewerError;
use crate::math::Mat4;
#[cfg(debug_assertions)]
use crate::model::{BodyPart, MeshPartDebugBounds};
use crate::skin::{decode_cape_png, decode_png, default_skin, ModelVariant};
use crate::webgl::Renderer;

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugMode {
    None,
    SolidHead,
    HeadTextured,
    HeadOverlay,
    BaseOnly,
    FullNoOverlays,
    FullOverlays,
    Unlit,
}

pub struct ViewerApp {
    renderer: Renderer,
    controls: OrbitControls,
    status: HtmlElement,
    animation_enabled: bool,
    overlays_enabled: bool,
    animation_speed: f32,
    elapsed_seconds: f32,
    last_timestamp_ms: Option<f64>,
    cape: CapeRenderState,
    #[cfg(debug_assertions)]
    debug_mode: DebugMode,
    viewer_preset: ViewerPreset,
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
            animation_enabled: false,
            overlays_enabled: true,
            animation_speed: 1.0,
            elapsed_seconds: 0.0,
            last_timestamp_ms: None,
            cape: CapeRenderState::new(),
            #[cfg(debug_assertions)]
            debug_mode: DebugMode::None,
            viewer_preset: ViewerPreset::Default,
        };
        app.load_default_skin()?;
        Ok(app)
    }

    pub fn load_skin_bytes(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.load_skin_bytes_with_model(bytes, false)
    }

    pub fn load_skin_bytes_with_model(&mut self, bytes: &[u8], slim: bool) -> Result<(), JsValue> {
        match decode_png(bytes).and_then(|skin| {
            let mut skin = skin;
            skin.model = model_variant(slim);
            let model = if slim { "slim" } else { "classic" };
            let message = format!("Loaded skin: {}x{} ({model})", skin.width, skin.height);
            self.renderer.upload_skin(&skin)?;
            Ok(message)
        }) {
            Ok(message) => {
                self.set_status_with_preset(&message);
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
        self.set_status_with_preset("Loaded default generated skin");
        Ok(())
    }

    pub fn load_cape_bytes(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        match decode_cape_png(bytes).and_then(|cape| {
            let message = format!("Loaded cape: {}x{}", cape.width, cape.height);
            self.renderer.upload_cape(&cape)?;
            self.cape.mark_loaded();
            Ok(message)
        }) {
            Ok(message) => {
                self.set_status_with_preset(&message);
                Ok(())
            }
            Err(error) => {
                let message = error.message();
                self.set_status(&message);
                Err(to_js_error(error))
            }
        }
    }

    pub fn clear_cape(&mut self) {
        self.renderer.clear_cape();
        self.cape.clear();
    }

    pub fn set_cape_visible(&mut self, visible: bool) -> String {
        let message = self.cape.set_visible(visible);
        self.set_status_with_preset(message);
        message.to_owned()
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

    #[cfg(debug_assertions)]
    pub fn set_debug_mode(&mut self, mode: &str) -> String {
        self.debug_mode = DebugMode::from_query_value(mode);
        let label = self.debug_mode.status_label();
        if !label.is_empty() {
            self.set_status(label);
        }
        label.to_owned()
    }

    pub fn set_preset(&mut self, preset: &str) -> String {
        self.viewer_preset = ViewerPreset::from_query_value(preset);
        self.controls.apply_preset(self.viewer_preset);
        if let Err(error) = self.renderer.set_model_preset(self.viewer_preset) {
            self.set_status(&error.message());
        }

        let label = format!("Preset: {}", self.viewer_preset.query_value());
        self.set_status(&label);
        label
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
        let pose = if self.animation_enabled && !self.debug_active() {
            if self.cape.should_render() {
                cape_reactive_walk_pose(phase)
            } else {
                walk_pose(phase)
            }
        } else {
            static_pose()
        };

        let (width, height) = self.renderer.pixel_size();
        let view = self.controls.view_matrix();
        let projection = profile_projection(self.viewer_preset, width / height);
        let bounds = self.renderer.model_bounds();
        let presentation = bounds
            .map(|bounds| presentation_matrix(self.viewer_preset, bounds))
            .unwrap_or_else(Mat4::identity);
        let light_dir = self.controls.camera_light_direction();
        #[cfg(debug_assertions)]
        match self.debug_mode {
            DebugMode::None => {}
            DebugMode::SolidHead => {
                return self
                    .renderer
                    .render_head_debug(
                        debug_head_front_view_matrix_for_preset(self.viewer_preset),
                        projection,
                        pose,
                        false,
                        false,
                    )
                    .map_err(to_js_error);
            }
            DebugMode::HeadTextured => {
                return self
                    .renderer
                    .render_head_debug(
                        debug_head_front_view_matrix_for_preset(self.viewer_preset),
                        projection,
                        pose,
                        true,
                        false,
                    )
                    .map_err(to_js_error);
            }
            DebugMode::HeadOverlay => {
                return self
                    .renderer
                    .render_head_debug(
                        debug_head_front_view_matrix_for_preset(self.viewer_preset),
                        projection,
                        pose,
                        true,
                        true,
                    )
                    .map_err(to_js_error);
            }
            DebugMode::BaseOnly | DebugMode::FullNoOverlays => {
                let result = self
                    .renderer
                    .render(
                        self.controls.view_matrix(),
                        projection,
                        pose,
                        false,
                        false,
                        false,
                        self.viewer_preset,
                        presentation,
                        light_dir,
                    )
                    .map_err(to_js_error);
                if result.is_ok() && self.viewer_preset.uses_skinview3d_hierarchy() {
                    if let Some(bounds) = bounds {
                        self.set_status(&self.skinview3d_world_debug_status(bounds));
                    }
                }
                return result;
            }
            DebugMode::FullOverlays => {
                return self
                    .renderer
                    .render(
                        self.controls.view_matrix(),
                        projection,
                        pose,
                        true,
                        false,
                        false,
                        self.viewer_preset,
                        presentation,
                        light_dir,
                    )
                    .map_err(to_js_error);
            }
            DebugMode::Unlit => {
                return self
                    .renderer
                    .render(
                        self.controls.view_matrix(),
                        projection,
                        pose,
                        true,
                        false,
                        true,
                        self.viewer_preset,
                        presentation,
                        light_dir,
                    )
                    .map_err(to_js_error);
            }
        }

        let result = self
            .renderer
            .render(
                view,
                projection,
                pose,
                self.overlays_enabled,
                self.cape.should_render(),
                false,
                self.viewer_preset,
                presentation,
                light_dir,
            )
            .map_err(to_js_error);
        #[cfg(debug_assertions)]
        if result.is_ok() && self.debug_active() {
            if let Some(bounds) = bounds {
                if self.viewer_preset.uses_skinview3d_hierarchy() {
                    self.set_status(&self.skinview3d_world_debug_status(bounds));
                } else {
                    self.set_status(&self.metrics_status(
                        bounds,
                        presentation,
                        view,
                        projection,
                        width,
                        height,
                    ));
                }
            }
        }
        result
    }

    #[inline]
    fn debug_active(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            !matches!(self.debug_mode, DebugMode::None)
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
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

    fn set_status(&self, message: &str) {
        self.status.set_text_content(Some(message));
    }

    fn set_status_with_preset(&self, message: &str) {
        self.set_status(&format!(
            "{message} | Preset: {}",
            self.viewer_preset.query_value()
        ));
    }

    #[cfg(debug_assertions)]
    fn metrics_status(
        &self,
        bounds: crate::model::MeshDebugBounds,
        presentation: Mat4,
        view: Mat4,
        projection: Mat4,
        width: f32,
        height: f32,
    ) -> String {
        let camera = self.viewer_preset.camera();
        let metrics =
            projected_bounds_metrics(bounds, presentation, view, projection, width, height);
        format!(
            "{} | bbox x {:.0}..{:.0} y {:.0}..{:.0} | h {:.1}% w {:.1}% | m {:.0}/{:.0}/{:.0}/{:.0} | {:.0}x{:.0} | fov {:.0} z {:.2} d {:.2}",
            self.viewer_preset.query_value(),
            metrics.min_x_px,
            metrics.max_x_px,
            metrics.min_y_px,
            metrics.max_y_px,
            metrics.height_ratio * 100.0,
            metrics.width_ratio * 100.0,
            metrics.top_px,
            metrics.bottom_px,
            metrics.left_px,
            metrics.right_px,
            width,
            height,
            camera.fov_degrees,
            camera.zoom,
            self.controls.distance(),
        )
    }

    #[cfg(debug_assertions)]
    fn skinview3d_world_debug_status(&self, bounds: crate::model::MeshDebugBounds) -> String {
        let mut parts = self
            .renderer
            .model_part_bounds()
            .iter()
            .map(format_part_bounds)
            .collect::<Vec<_>>();
        parts.push(format!(
            "total c({:.2},{:.2},{:.2}) b({:.2},{:.2},{:.2})->({:.2},{:.2},{:.2})",
            bounds.center.x,
            bounds.center.y,
            bounds.center.z,
            bounds.min.x,
            bounds.min.y,
            bounds.min.z,
            bounds.max.x,
            bounds.max.y,
            bounds.max.z
        ));
        format!(
            "Preset: {} | world {}",
            self.viewer_preset.query_value(),
            parts.join(" ; ")
        )
    }
}

#[cfg(debug_assertions)]
impl DebugMode {
    fn from_query_value(value: &str) -> Self {
        match value {
            "solid-head" | "head" => Self::SolidHead,
            "head-textured" => Self::HeadTextured,
            "head-overlay" => Self::HeadOverlay,
            "base-only" => Self::BaseOnly,
            "full-no-overlays" => Self::FullNoOverlays,
            "full-overlays" => Self::FullOverlays,
            "unlit" => Self::Unlit,
            _ => Self::None,
        }
    }

    fn status_label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::SolidHead => "Debug: solid base head only",
            Self::HeadTextured => "Debug: textured base head only",
            Self::HeadOverlay => "Debug: textured base head + hat layer",
            Self::BaseOnly => "Debug: base model only",
            Self::FullNoOverlays => "Debug: full model without overlays",
            Self::FullOverlays => "Debug: full model with overlays",
            Self::Unlit => "Debug: full model unlit",
        }
    }
}

#[cfg(debug_assertions)]
fn format_part_bounds(part: &MeshPartDebugBounds) -> String {
    let label = part_label(part.part, part.overlay);
    let bounds = part.bounds;
    format!(
        "{} c({:.2},{:.2},{:.2}) b({:.2},{:.2},{:.2})->({:.2},{:.2},{:.2})",
        label,
        bounds.center.x,
        bounds.center.y,
        bounds.center.z,
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z
    )
}

#[cfg(debug_assertions)]
fn part_label(part: BodyPart, overlay: bool) -> &'static str {
    match (part, overlay) {
        (BodyPart::Head, false) => "head",
        (BodyPart::Head, true) => "hat",
        (BodyPart::Body, false) => "body",
        (BodyPart::Body, true) => "jacket",
        (BodyPart::RightArm, false) => "right_arm",
        (BodyPart::RightArm, true) => "right_sleeve",
        (BodyPart::LeftArm, false) => "left_arm",
        (BodyPart::LeftArm, true) => "left_sleeve",
        (BodyPart::RightLeg, false) => "right_leg",
        (BodyPart::RightLeg, true) => "right_pants",
        (BodyPart::LeftLeg, false) => "left_leg",
        (BodyPart::LeftLeg, true) => "left_pants",
        (BodyPart::Cape, _) => "cape",
    }
}

fn model_variant(slim: bool) -> ModelVariant {
    if slim {
        ModelVariant::Slim
    } else {
        ModelVariant::Classic
    }
}

fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}

fn to_js_error(error: ViewerError) -> JsValue {
    JsValue::from_str(&error.message())
}
