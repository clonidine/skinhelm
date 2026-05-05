use crate::math::{Mat4, Vec3};
use crate::model::MeshDebugBounds;

pub const DEFAULT_PROFILE_FOV_DEGREES: f32 = 50.0;
pub const DEFAULT_PROFILE_ZOOM: f32 = 0.9;
pub const OVERLAY_ALPHA_DISCARD_THRESHOLD: f32 = 0.00001;
pub const PROFILE_TARGET: Vec3 = Vec3::new(0.0, 15.5, 0.0);
pub const PROFILE_FAR_PLANE: f32 = 360.0;

const DEFAULT_PROFILE_YAW_DEGREES: f32 = -15.0;
const DEFAULT_PROFILE_PITCH_DEGREES: f32 = 6.0;
const MIN_CAMERA_DISTANCE: f32 = 10.0;
const MAX_CAMERA_DISTANCE: f32 = 256.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerPreset {
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightMode {
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPreset {
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub target: Vec3,
    pub fov_degrees: f32,
    pub zoom: f32,
}

impl ViewerPreset {
    #[inline]
    pub fn from_query_value(_value: &str) -> Self {
        Self::Default
    }

    #[inline]
    pub fn query_value(self) -> &'static str {
        match self {
            Self::Default => "default",
        }
    }

    #[inline]
    pub fn uses_skinview3d_hierarchy(self) -> bool {
        false
    }

    #[inline]
    pub fn light_mode(self) -> LightMode {
        match self {
            Self::Default => LightMode::Default,
        }
    }

    #[inline]
    pub fn camera(self) -> CameraPreset {
        match self {
            Self::Default => CameraPreset {
                yaw_degrees: DEFAULT_PROFILE_YAW_DEGREES,
                pitch_degrees: DEFAULT_PROFILE_PITCH_DEGREES,
                target: PROFILE_TARGET,
                fov_degrees: DEFAULT_PROFILE_FOV_DEGREES,
                zoom: DEFAULT_PROFILE_ZOOM,
            },
        }
    }
}

#[cfg(any(debug_assertions, test))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedBoundsMetrics {
    pub min_x_px: f32,
    pub max_x_px: f32,
    pub min_y_px: f32,
    pub max_y_px: f32,
    pub left_px: f32,
    pub right_px: f32,
    pub top_px: f32,
    pub bottom_px: f32,
    pub width_ratio: f32,
    pub height_ratio: f32,
}

#[derive(Debug, Clone)]
pub struct OrbitControls {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
    dragging: bool,
    last_x: f32,
    last_y: f32,
}

impl OrbitControls {
    #[inline]
    pub fn new() -> Self {
        Self::with_preset(ViewerPreset::Default)
    }

    #[inline]
    pub fn with_preset(preset: ViewerPreset) -> Self {
        let camera = preset.camera();
        Self {
            yaw: camera.yaw_degrees.to_radians(),
            pitch: camera.pitch_degrees.to_radians(),
            distance: fit_camera_distance(camera.fov_degrees, camera.zoom),
            target: camera.target,
            dragging: false,
            last_x: 0.0,
            last_y: 0.0,
        }
    }

    #[inline]
    pub fn apply_preset(&mut self, preset: ViewerPreset) {
        let next = Self::with_preset(preset);
        self.yaw = next.yaw;
        self.pitch = next.pitch;
        self.distance = next.distance;
        self.target = next.target;
    }

    #[inline]
    pub fn pointer_down(&mut self, x: f32, y: f32) {
        self.dragging = true;
        self.last_x = x;
        self.last_y = y;
    }

    #[inline]
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        if !self.dragging {
            return;
        }
        let dx = x - self.last_x;
        let dy = y - self.last_y;
        self.last_x = x;
        self.last_y = y;
        self.yaw -= dx * 0.01;
        self.pitch = (self.pitch + dy * 0.01).clamp(-1.2, 1.2);
    }

    #[inline]
    pub fn pointer_up(&mut self) {
        self.dragging = false;
    }

    #[inline]
    pub fn zoom(&mut self, delta_y: f32) {
        self.distance =
            (self.distance + delta_y * 0.04).clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
    }

    #[inline(always)]
    pub fn distance(&self) -> f32 {
        self.distance
    }

    #[inline(always)]
    pub fn target(&self) -> Vec3 {
        self.target
    }

    #[inline]
    pub fn camera_light_direction(&self) -> Vec3 {
        self.eye_position().subtract(self.target).normalize()
    }

    #[inline]
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.eye_position(), self.target, Vec3::new(0.0, 1.0, 0.0))
    }

    #[inline]
    fn eye_position(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        Vec3::new(
            self.distance * self.yaw.sin() * cos_pitch,
            self.target.y + self.distance * self.pitch.sin(),
            self.distance * self.yaw.cos() * cos_pitch,
        )
    }
}

impl Default for OrbitControls {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn default_profile_projection(aspect: f32) -> Mat4 {
    profile_projection(ViewerPreset::Default, aspect)
}

#[inline]
pub fn profile_projection(preset: ViewerPreset, aspect: f32) -> Mat4 {
    Mat4::perspective(
        preset.camera().fov_degrees.to_radians(),
        aspect,
        0.1,
        PROFILE_FAR_PLANE,
    )
}

#[inline]
pub fn fit_camera_distance(fov_degrees: f32, zoom: f32) -> f32 {
    let safe_zoom = zoom.max(0.0001);
    let distance = 4.5 + 16.5 / (fov_degrees.to_radians() * 0.5).tan() / safe_zoom;
    distance.clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE)
}

#[inline]
pub fn presentation_matrix(preset: ViewerPreset, bounds: MeshDebugBounds) -> Mat4 {
    let _ = (preset, bounds);
    Mat4::identity()
}

#[cfg(any(debug_assertions, test))]
pub fn projected_bounds_metrics(
    bounds: MeshDebugBounds,
    presentation: Mat4,
    view: Mat4,
    projection: Mat4,
    width: f32,
    height: f32,
) -> ProjectedBoundsMetrics {
    let matrix = projection.multiply(view).multiply(presentation);
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for point in bounds_corners(bounds) {
        let clip_x =
            matrix.m[0] * point.x + matrix.m[4] * point.y + matrix.m[8] * point.z + matrix.m[12];
        let clip_y =
            matrix.m[1] * point.x + matrix.m[5] * point.y + matrix.m[9] * point.z + matrix.m[13];
        let clip_w =
            matrix.m[3] * point.x + matrix.m[7] * point.y + matrix.m[11] * point.z + matrix.m[15];
        let ndc_x = clip_x / clip_w;
        let ndc_y = clip_y / clip_w;
        let screen_x = (ndc_x + 1.0) * 0.5 * width;
        let screen_y = (1.0 - ndc_y) * 0.5 * height;
        min_x = min_x.min(screen_x);
        max_x = max_x.max(screen_x);
        min_y = min_y.min(screen_y);
        max_y = max_y.max(screen_y);
    }

    ProjectedBoundsMetrics {
        min_x_px: min_x,
        max_x_px: max_x,
        min_y_px: min_y,
        max_y_px: max_y,
        left_px: min_x,
        right_px: width - max_x,
        top_px: min_y,
        bottom_px: height - max_y,
        width_ratio: (max_x - min_x) / width,
        height_ratio: (max_y - min_y) / height,
    }
}

#[inline]
#[cfg(any(debug_assertions, test))]
fn bounds_corners(bounds: MeshDebugBounds) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}

#[inline]
#[cfg(any(debug_assertions, test))]
pub fn debug_head_front_view_matrix() -> Mat4 {
    Mat4::look_at(
        Vec3::new(0.0, 28.0, 64.0),
        Vec3::new(0.0, 28.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    )
}

#[inline]
#[cfg(any(debug_assertions, test))]
pub fn debug_head_front_view_matrix_for_preset(preset: ViewerPreset) -> Mat4 {
    let _ = preset;
    debug_head_front_view_matrix()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_controls_match_default_profile_preset() {
        let controls = OrbitControls::new();

        assert!((controls.yaw - (-15.0_f32).to_radians()).abs() < 0.0001);
        assert!((controls.pitch - 6.0_f32.to_radians()).abs() < 0.0001);
        assert!((controls.distance - fit_camera_distance(50.0, 0.9)).abs() < 0.0001);
        assert_eq!(controls.target, Vec3::new(0.0, 15.5, 0.0));
    }

    #[test]
    fn every_query_value_resolves_to_default_preset() {
        assert_eq!(
            ViewerPreset::from_query_value("default"),
            ViewerPreset::Default
        );
        assert_eq!(
            ViewerPreset::from_query_value("laby-profile"),
            ViewerPreset::Default
        );
        assert_eq!(
            ViewerPreset::from_query_value("skinview3d-1to1"),
            ViewerPreset::Default
        );
        assert_eq!(ViewerPreset::from_query_value(""), ViewerPreset::Default);
    }

    #[test]
    fn default_camera_matches_chosen_values() {
        let camera = ViewerPreset::Default.camera();

        assert_eq!(camera.target, Vec3::new(0.0, 15.5, 0.0));
        assert!((camera.yaw_degrees - -15.0).abs() < 0.0001);
        assert!((camera.pitch_degrees - 6.0).abs() < 0.0001);
        assert!((camera.fov_degrees - 50.0).abs() < 0.0001);
        assert!((camera.zoom - 0.9).abs() < 0.0001);
    }

    #[test]
    fn camera_fit_distance_matches_default_formula() {
        let distance = fit_camera_distance(50.0, 0.9);
        let expected = 4.5 + 16.5 / (50.0_f32.to_radians() * 0.5).tan() / 0.9;

        assert!((distance - expected).abs() < 0.0001);
    }

    #[test]
    fn camera_fit_distance_is_clamped() {
        assert!((fit_camera_distance(179.0, 10_000.0) - MIN_CAMERA_DISTANCE).abs() < 0.0001);
        assert!((fit_camera_distance(1.0, 0.0001) - MAX_CAMERA_DISTANCE).abs() < 0.0001);
    }

    #[test]
    fn default_fit_keeps_full_model_visible() {
        let controls = OrbitControls::new();
        let view_projection = default_profile_projection(1.0).multiply(controls.view_matrix());
        let feet = ndc_y(view_projection, Vec3::new(0.0, 0.0, 0.0));
        let headwear_top = ndc_y(view_projection, Vec3::new(0.0, 32.5, 0.0));
        let canvas_coverage = (headwear_top - feet) * 0.5;

        assert!((-1.0..=1.0).contains(&feet));
        assert!((-1.0..=1.0).contains(&headwear_top));
        assert!((0.78..=0.82).contains(&canvas_coverage));
    }

    #[test]
    fn default_presentation_does_not_move_bounds() {
        let bounds = MeshDebugBounds {
            min: Vec3::new(-8.25, -0.25, -4.5),
            max: Vec3::new(8.25, 32.5, 4.5),
            size: Vec3::new(16.5, 32.75, 9.0),
            center: Vec3::new(0.0, 16.125, 0.0),
        };
        let presentation = presentation_matrix(ViewerPreset::Default, bounds);
        let min = presentation.transform_point(bounds.min);
        let max = presentation.transform_point(bounds.max);

        assert_vec3_close(min, bounds.min);
        assert_vec3_close(max, bounds.max);
        assert_vec3_close(
            Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z),
            bounds.size,
        );
    }

    #[test]
    fn overlay_alpha_discard_threshold_matches_skinview3d_alpha_test_scale() {
        assert!((OVERLAY_ALPHA_DISCARD_THRESHOLD - 0.00001).abs() < f32::EPSILON);
    }

    #[test]
    fn profile_projection_uses_supplied_aspect() {
        let square = default_profile_projection(1.0);
        let wide = default_profile_projection(2.0);

        assert!((wide.m[0] - square.m[0] * 0.5).abs() < 0.0001);
        assert!((wide.m[5] - square.m[5]).abs() < 0.0001);
    }

    #[test]
    fn debug_head_camera_is_front_facing_and_centered_on_head() {
        let view = debug_head_front_view_matrix();

        assert!((view.m[12] - 0.0).abs() < 0.0001);
        assert!((view.m[13] - -28.0).abs() < 0.0001);
        assert!((view.m[14] - -64.0).abs() < 0.0001);
    }

    #[test]
    fn debug_head_camera_for_any_preset_uses_default_head_camera() {
        let view =
            debug_head_front_view_matrix_for_preset(ViewerPreset::from_query_value("legacy"));

        assert!((view.m[12] - 0.0).abs() < 0.0001);
        assert!((view.m[13] - -28.0).abs() < 0.0001);
        assert!((view.m[14] - -64.0).abs() < 0.0001);
    }

    #[test]
    fn zoom_can_move_camera_farther_out() {
        let mut controls = OrbitControls::new();

        controls.zoom(10_000.0);

        assert!((controls.distance - MAX_CAMERA_DISTANCE).abs() < 0.0001);
    }

    fn ndc_y(matrix: Mat4, point: Vec3) -> f32 {
        let clip_y =
            matrix.m[1] * point.x + matrix.m[5] * point.y + matrix.m[9] * point.z + matrix.m[13];
        let clip_w =
            matrix.m[3] * point.x + matrix.m[7] * point.y + matrix.m[11] * point.z + matrix.m[15];

        clip_y / clip_w
    }

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!((actual.x - expected.x).abs() < 0.0001);
        assert!((actual.y - expected.y).abs() < 0.0001);
        assert!((actual.z - expected.z).abs() < 0.0001);
    }
}
