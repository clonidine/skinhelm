use crate::animation::WalkPose;
use crate::math::{Mat4, Vec3};
use crate::skin::{ModelVariant, SkinFormat, SkinImage};

pub const VERTEX_STRIDE: usize = 8;
pub const HEAD_OBJECT_METERS: f32 = 0.5;
pub const HEAD_SKIN_PIXELS: f32 = 8.0;
pub const SKIN_PIXEL_METERS: f32 = HEAD_OBJECT_METERS / HEAD_SKIN_PIXELS;

const BODY_OVERLAY_SIZE: Vec3 = Vec3::new(
    8.0 + OUTER_LAYER_DILATION * 2.0,
    12.0 + OUTER_LAYER_DILATION * 2.0,
    4.0 + OUTER_LAYER_DILATION * 2.0,
);
const HEAD_OVERLAY_SIZE: Vec3 = Vec3::new(
    8.0 + HAT_LAYER_DILATION * 2.0,
    8.0 + HAT_LAYER_DILATION * 2.0,
    8.0 + HAT_LAYER_DILATION * 2.0,
);
const LEG_OVERLAY_SIZE: Vec3 = Vec3::new(
    4.0 + OUTER_LAYER_DILATION * 2.0,
    12.0 + OUTER_LAYER_DILATION * 2.0,
    4.0 + OUTER_LAYER_DILATION * 2.0,
);
const HAT_LAYER_DILATION: f32 = 0.5;
const OUTER_LAYER_DILATION: f32 = 0.25;
const UV_EDGE_INSET_TEXELS: f32 = 0.001;
const OVERLAY_ALPHA_TEST_THRESHOLD: u8 = 128;
const OVERLAY_FACE_MIN_VISIBLE_RATIO: f32 = 0.45;
const MIN_HEAD_OVERLAY_PIXELS: usize = 64;
const MIN_BODY_OVERLAY_PIXELS: usize = 64;
const MIN_LIMB_OVERLAY_PIXELS: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UvRect {
    #[inline(always)]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[inline]
    pub fn normalized(self, skin_width: f32, skin_height: f32) -> NormalizedUv {
        let x_inset = UV_EDGE_INSET_TEXELS.min(self.w * 0.5);
        let y_inset = UV_EDGE_INSET_TEXELS.min(self.h * 0.5);
        NormalizedUv {
            left: (self.x + x_inset) / skin_width,
            right: (self.x + self.w - x_inset) / skin_width,
            top: (self.y + y_inset) / skin_height,
            bottom: (self.y + self.h - y_inset) / skin_height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedUv {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyPart {
    Head,
    Body,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
    Cape,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<f32>,
    pub vertex_count: i32,
    pub part: BodyPart,
    pub overlay: bool,
    pub pivot: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshDebugBounds {
    pub min: Vec3,
    pub max: Vec3,
    pub size: Vec3,
    pub center: Vec3,
}

#[cfg(any(debug_assertions, test))]
pub type PlayerDebugBounds = MeshDebugBounds;

#[cfg(any(debug_assertions, test))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshPartDebugBounds {
    pub part: BodyPart,
    pub overlay: bool,
    pub bounds: MeshDebugBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerModelStyle {
    FeetAtY0,
    Skinview3d1To1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Skinview3dPartNode {
    pub part: BodyPart,
    pub overlay: bool,
    pub group_position: Vec3,
    pub mesh_local_position: Vec3,
    pub size: Vec3,
    pub pivot: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skinview3dHierarchy {
    pub skin_position: Vec3,
    pub parts: Vec<Skinview3dPartNode>,
}

#[cfg(any(debug_assertions, test))]
pub fn debug_head_bounds() -> MeshDebugBounds {
    let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
    let head = meshes
        .iter()
        .find(|mesh| mesh.part == BodyPart::Head && !mesh.overlay)
        .expect("base head mesh is generated");
    mesh_debug_bounds(head)
}

#[cfg(any(debug_assertions, test))]
pub fn player_debug_bounds(format: SkinFormat, variant: ModelVariant) -> PlayerDebugBounds {
    let meshes = build_player_meshes(format, variant);
    meshes_debug_bounds(meshes.iter())
}

#[cfg(any(debug_assertions, test))]
pub fn player_debug_bounds_for_style(
    format: SkinFormat,
    variant: ModelVariant,
    style: PlayerModelStyle,
) -> PlayerDebugBounds {
    let meshes = build_player_meshes_for_style(format, variant, style);
    meshes_debug_bounds(meshes.iter())
}

#[cfg(any(debug_assertions, test))]
pub fn player_part_debug_bounds_for_style(
    format: SkinFormat,
    variant: ModelVariant,
    style: PlayerModelStyle,
) -> Vec<MeshPartDebugBounds> {
    let meshes = build_player_meshes_for_style(format, variant, style);
    meshes_part_debug_bounds(meshes.iter())
}

pub fn meshes_debug_bounds<'a>(meshes: impl Iterator<Item = &'a Mesh>) -> MeshDebugBounds {
    let (min, max) = meshes
        .flat_map(|mesh| mesh.vertices.chunks_exact(VERTEX_STRIDE))
        .fold(
            (
                Vec3::new(f32::MAX, f32::MAX, f32::MAX),
                Vec3::new(f32::MIN, f32::MIN, f32::MIN),
            ),
            |(min, max), vertex| {
                (
                    Vec3::new(
                        min.x.min(vertex[0]),
                        min.y.min(vertex[1]),
                        min.z.min(vertex[2]),
                    ),
                    Vec3::new(
                        max.x.max(vertex[0]),
                        max.y.max(vertex[1]),
                        max.z.max(vertex[2]),
                    ),
                )
            },
        );
    bounds_from_min_max(min, max)
}

#[cfg(any(debug_assertions, test))]
pub fn meshes_part_debug_bounds<'a>(
    meshes: impl Iterator<Item = &'a Mesh>,
) -> Vec<MeshPartDebugBounds> {
    meshes
        .map(|mesh| MeshPartDebugBounds {
            part: mesh.part,
            overlay: mesh.overlay,
            bounds: mesh_debug_bounds(mesh),
        })
        .collect()
}

pub fn mesh_debug_bounds(mesh: &Mesh) -> MeshDebugBounds {
    let (min, max) = mesh.vertices.chunks_exact(VERTEX_STRIDE).fold(
        (
            Vec3::new(f32::MAX, f32::MAX, f32::MAX),
            Vec3::new(f32::MIN, f32::MIN, f32::MIN),
        ),
        |(min, max), vertex| {
            (
                Vec3::new(
                    min.x.min(vertex[0]),
                    min.y.min(vertex[1]),
                    min.z.min(vertex[2]),
                ),
                Vec3::new(
                    max.x.max(vertex[0]),
                    max.y.max(vertex[1]),
                    max.z.max(vertex[2]),
                ),
            )
        },
    );
    bounds_from_min_max(min, max)
}

#[inline]
fn bounds_from_min_max(min: Vec3, max: Vec3) -> MeshDebugBounds {
    MeshDebugBounds {
        min,
        max,
        size: Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z),
        center: Vec3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            (min.z + max.z) * 0.5,
        ),
    }
}

#[derive(Debug, Clone, Copy)]
struct FaceUvs {
    top: UvRect,
    bottom: UvRect,
    right: UvRect,
    front: UvRect,
    left: UvRect,
    back: UvRect,
}

impl FaceUvs {
    #[inline]
    fn rects(self) -> [UvRect; 6] {
        [
            self.top,
            self.bottom,
            self.right,
            self.front,
            self.left,
            self.back,
        ]
    }
}

#[derive(Debug, Clone, Copy)]
struct PartSpec {
    part: BodyPart,
    center: Vec3,
    size: Vec3,
    pivot: Vec3,
    uvs: FaceUvs,
    overlay: bool,
    inflate: f32,
    hidden_top: bool,
}

#[derive(Debug, Clone, Copy)]
struct ArmUvLayout {
    top_x: f32,
    bottom_x: f32,
    right_x: f32,
    front_x: f32,
    left_x: f32,
    back_x: f32,
    cap_y: f32,
    side_y: f32,
}

pub fn build_player_meshes(format: SkinFormat, variant: ModelVariant) -> Vec<Mesh> {
    build_player_meshes_for_style(format, variant, PlayerModelStyle::FeetAtY0)
}

pub fn build_player_meshes_for_style(
    format: SkinFormat,
    variant: ModelVariant,
    style: PlayerModelStyle,
) -> Vec<Mesh> {
    if matches!(style, PlayerModelStyle::Skinview3d1To1) {
        return build_skinview3d_player_meshes(format, variant);
    }

    let skin_height = match format {
        SkinFormat::Modern64x64 => 64.0,
        SkinFormat::Legacy64x32 => 32.0,
    };
    let mut specs = base_specs(format, variant);
    if matches!(format, SkinFormat::Modern64x64) {
        specs.extend(overlay_specs(variant));
    } else {
        specs.push(hat_spec());
    }

    specs
        .into_iter()
        .map(|spec| build_mesh(spec, 64.0, skin_height))
        .collect()
}

pub fn overlay_part_has_visible_pixels(skin: &SkinImage, part: BodyPart) -> bool {
    if !matches!(
        part,
        BodyPart::Head
            | BodyPart::Body
            | BodyPart::RightArm
            | BodyPart::LeftArm
            | BodyPart::RightLeg
            | BodyPart::LeftLeg
    ) {
        return false;
    }

    if matches!(skin.format, SkinFormat::Legacy64x32) && part != BodyPart::Head {
        return false;
    }

    let visible_pixels = skinview3d_uvs(skin.format, skin.model, part, true)
        .rects()
        .into_iter()
        .map(|rect| uv_rect_visible_pixel_count(skin, rect))
        .sum::<usize>();

    visible_pixels >= overlay_visible_pixel_minimum(part)
}

pub fn prune_sparse_overlay_faces(meshes: &mut [Mesh], skin: &SkinImage) {
    for mesh in meshes.iter_mut().filter(|mesh| mesh.overlay) {
        let mut vertices = Vec::with_capacity(mesh.vertices.len());
        for face in mesh.vertices.chunks_exact(6 * VERTEX_STRIDE) {
            if overlay_face_has_enough_visible_pixels(face, skin) {
                vertices.extend_from_slice(face);
            }
        }
        mesh.vertex_count = (vertices.len() / VERTEX_STRIDE) as i32;
        mesh.vertices = vertices;
    }
}

pub fn skinview3d_hierarchy(variant: ModelVariant) -> Skinview3dHierarchy {
    let skin_position = Vec3::new(0.0, 8.0, 0.0);
    let arm_width = arm_width(variant);
    let sleeve_width = arm_width + OUTER_LAYER_DILATION * 2.0;
    let right_arm_local_x = match variant {
        ModelVariant::Classic => -1.0,
        ModelVariant::Slim => -0.5,
    };
    let left_arm_local_x = match variant {
        ModelVariant::Classic => 1.0,
        ModelVariant::Slim => 0.5,
    };

    Skinview3dHierarchy {
        skin_position,
        parts: vec![
            Skinview3dPartNode {
                part: BodyPart::Head,
                overlay: false,
                group_position: Vec3::new(0.0, 0.0, 0.0),
                mesh_local_position: Vec3::new(0.0, 4.0, 0.0),
                size: Vec3::new(8.0, 8.0, 8.0),
                pivot: skin_position,
            },
            Skinview3dPartNode {
                part: BodyPart::Head,
                overlay: true,
                group_position: Vec3::new(0.0, 0.0, 0.0),
                mesh_local_position: Vec3::new(0.0, 4.0, 0.0),
                size: HEAD_OVERLAY_SIZE,
                pivot: skin_position,
            },
            Skinview3dPartNode {
                part: BodyPart::Body,
                overlay: false,
                group_position: Vec3::new(0.0, 0.0, 0.0),
                mesh_local_position: Vec3::new(0.0, -6.0, 0.0),
                size: Vec3::new(8.0, 12.0, 4.0),
                pivot: Vec3::new(0.0, skin_position.y - 6.0, 0.0),
            },
            Skinview3dPartNode {
                part: BodyPart::Body,
                overlay: true,
                group_position: Vec3::new(0.0, 0.0, 0.0),
                mesh_local_position: Vec3::new(0.0, -6.0, 0.0),
                size: BODY_OVERLAY_SIZE,
                pivot: Vec3::new(0.0, skin_position.y - 6.0, 0.0),
            },
            Skinview3dPartNode {
                part: BodyPart::RightArm,
                overlay: false,
                group_position: Vec3::new(-5.0, -2.0, 0.0),
                mesh_local_position: Vec3::new(right_arm_local_x, -4.0, 0.0),
                size: Vec3::new(arm_width, 12.0, 4.0),
                pivot: Vec3::new(-5.0, skin_position.y - 2.0, 0.0),
            },
            Skinview3dPartNode {
                part: BodyPart::RightArm,
                overlay: true,
                group_position: Vec3::new(-5.0, -2.0, 0.0),
                mesh_local_position: Vec3::new(right_arm_local_x, -4.0, 0.0),
                size: Vec3::new(sleeve_width, 12.5, 4.5),
                pivot: Vec3::new(-5.0, skin_position.y - 2.0, 0.0),
            },
            Skinview3dPartNode {
                part: BodyPart::LeftArm,
                overlay: false,
                group_position: Vec3::new(5.0, -2.0, 0.0),
                mesh_local_position: Vec3::new(left_arm_local_x, -4.0, 0.0),
                size: Vec3::new(arm_width, 12.0, 4.0),
                pivot: Vec3::new(5.0, skin_position.y - 2.0, 0.0),
            },
            Skinview3dPartNode {
                part: BodyPart::LeftArm,
                overlay: true,
                group_position: Vec3::new(5.0, -2.0, 0.0),
                mesh_local_position: Vec3::new(left_arm_local_x, -4.0, 0.0),
                size: Vec3::new(sleeve_width, 12.5, 4.5),
                pivot: Vec3::new(5.0, skin_position.y - 2.0, 0.0),
            },
            Skinview3dPartNode {
                part: BodyPart::RightLeg,
                overlay: false,
                group_position: Vec3::new(-1.9, -12.0, -0.1),
                mesh_local_position: Vec3::new(0.0, 0.0, 0.0),
                size: Vec3::new(4.0, 12.0, 4.0),
                pivot: Vec3::new(-1.9, skin_position.y - 12.0, -0.1),
            },
            Skinview3dPartNode {
                part: BodyPart::RightLeg,
                overlay: true,
                group_position: Vec3::new(-1.9, -12.0, -0.1),
                mesh_local_position: Vec3::new(0.0, 0.0, 0.0),
                size: LEG_OVERLAY_SIZE,
                pivot: Vec3::new(-1.9, skin_position.y - 12.0, -0.1),
            },
            Skinview3dPartNode {
                part: BodyPart::LeftLeg,
                overlay: false,
                group_position: Vec3::new(1.9, -12.0, -0.1),
                mesh_local_position: Vec3::new(0.0, 0.0, 0.0),
                size: Vec3::new(4.0, 12.0, 4.0),
                pivot: Vec3::new(1.9, skin_position.y - 12.0, -0.1),
            },
            Skinview3dPartNode {
                part: BodyPart::LeftLeg,
                overlay: true,
                group_position: Vec3::new(1.9, -12.0, -0.1),
                mesh_local_position: Vec3::new(0.0, 0.0, 0.0),
                size: LEG_OVERLAY_SIZE,
                pivot: Vec3::new(1.9, skin_position.y - 12.0, -0.1),
            },
        ],
    }
}

fn build_skinview3d_player_meshes(format: SkinFormat, variant: ModelVariant) -> Vec<Mesh> {
    let skin_height = match format {
        SkinFormat::Modern64x64 => 64.0,
        SkinFormat::Legacy64x32 => 32.0,
    };
    let hierarchy = skinview3d_hierarchy(variant);
    let mut specs = Vec::new();

    for node in hierarchy.parts {
        if node.overlay && !matches!(format, SkinFormat::Modern64x64) && node.part != BodyPart::Head
        {
            continue;
        }

        specs.push(PartSpec {
            part: node.part,
            center: Vec3::new(
                hierarchy.skin_position.x + node.group_position.x + node.mesh_local_position.x,
                hierarchy.skin_position.y + node.group_position.y + node.mesh_local_position.y,
                hierarchy.skin_position.z + node.group_position.z + node.mesh_local_position.z,
            ),
            size: node.size,
            pivot: node.pivot,
            uvs: skinview3d_uvs(format, variant, node.part, node.overlay),
            overlay: node.overlay,
            inflate: 1.0,
            hidden_top: false,
        });
    }

    specs
        .into_iter()
        .map(|spec| build_mesh(spec, 64.0, skin_height))
        .collect()
}

#[inline]
pub fn part_model_matrix(part: BodyPart, pivot: Vec3, pose: WalkPose) -> Mat4 {
    let angle = match part {
        BodyPart::RightLeg => pose.right_leg_x,
        BodyPart::LeftLeg => pose.left_leg_x,
        BodyPart::RightArm => pose.right_arm_x,
        BodyPart::LeftArm => pose.left_arm_x,
        BodyPart::Cape => pose.cape_x,
        BodyPart::Head | BodyPart::Body => 0.0,
    };
    let bob = match part {
        BodyPart::Head => pose.body_y + pose.head_y,
        BodyPart::Body | BodyPart::RightArm | BodyPart::LeftArm | BodyPart::Cape => pose.body_y,
        BodyPart::RightLeg | BodyPart::LeftLeg => 0.0,
    };

    Mat4::translation(0.0, skin_pixels_to_meters(bob), 0.0)
        .multiply(Mat4::translation(pivot.x, pivot.y, pivot.z))
        .multiply(Mat4::rotation_x(angle))
        .multiply(Mat4::translation(-pivot.x, -pivot.y, -pivot.z))
}

pub fn build_cape_mesh(cape_width: f32, cape_height: f32) -> Mesh {
    build_mesh(
        PartSpec {
            part: BodyPart::Cape,
            center: Vec3::new(0.0, 16.0, -2.76),
            size: Vec3::new(10.0, 16.0, 1.0),
            pivot: Vec3::new(0.0, 24.0, -2.25),
            uvs: FaceUvs {
                top: UvRect::new(1.0, 0.0, 10.0, 1.0),
                bottom: UvRect::new(11.0, 0.0, 10.0, 1.0),
                right: UvRect::new(0.0, 1.0, 1.0, 16.0),
                front: UvRect::new(12.0, 1.0, 10.0, 16.0),
                left: UvRect::new(11.0, 1.0, 1.0, 16.0),
                back: UvRect::new(1.0, 1.0, 10.0, 16.0),
            },
            overlay: false,
            inflate: 1.0,
            hidden_top: false,
        },
        cape_width,
        cape_height,
    )
}

fn build_mesh(spec: PartSpec, skin_width: f32, skin_height: f32) -> Mesh {
    let center = skin_pixel_vec3_to_meters(spec.center);
    let size = skin_pixel_vec3_to_meters(spec.size);
    let pivot = skin_pixel_vec3_to_meters(spec.pivot);
    let mut vertices = Vec::with_capacity(36 * VERTEX_STRIDE);
    let half = Vec3::new(
        size.x * spec.inflate * 0.5,
        size.y * spec.inflate * 0.5,
        size.z * spec.inflate * 0.5,
    );
    let min = Vec3::new(center.x - half.x, center.y - half.y, center.z - half.z);
    let max = Vec3::new(center.x + half.x, center.y + half.y, center.z + half.z);

    if !spec.hidden_top {
        push_face_reversed(
            &mut vertices,
            [
                Vec3::new(min.x, max.y, max.z),
                Vec3::new(min.x, max.y, min.z),
                Vec3::new(max.x, max.y, min.z),
                Vec3::new(max.x, max.y, max.z),
            ],
            spec.uvs.top.normalized(skin_width, skin_height),
            Vec3::new(0.0, 1.0, 0.0),
        );
    }
    push_face_reversed(
        &mut vertices,
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        spec.uvs.bottom.normalized(skin_width, skin_height),
        Vec3::new(0.0, -1.0, 0.0),
    );
    push_face_reversed(
        &mut vertices,
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, min.y, max.z),
        ],
        spec.uvs.right.normalized(skin_width, skin_height),
        Vec3::new(-1.0, 0.0, 0.0),
    );
    push_face(
        &mut vertices,
        [
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, min.y, max.z),
        ],
        spec.uvs.front.normalized(skin_width, skin_height),
        Vec3::new(0.0, 0.0, 1.0),
    );
    push_face_reversed(
        &mut vertices,
        [
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        spec.uvs.left.normalized(skin_width, skin_height),
        Vec3::new(1.0, 0.0, 0.0),
    );
    push_face(
        &mut vertices,
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        spec.uvs.back.normalized(skin_width, skin_height),
        Vec3::new(0.0, 0.0, -1.0),
    );

    Mesh {
        vertex_count: (vertices.len() / VERTEX_STRIDE) as i32,
        vertices,
        part: spec.part,
        overlay: spec.overlay,
        pivot,
    }
}

fn uv_rect_visible_pixel_count(skin: &SkinImage, rect: UvRect) -> usize {
    let min_x = rect.x.max(0.0).floor() as u32;
    let min_y = rect.y.max(0.0).floor() as u32;
    let max_x = (rect.x + rect.w).ceil().max(0.0).min(skin.width as f32) as u32;
    let max_y = (rect.y + rect.h).ceil().max(0.0).min(skin.height as f32) as u32;

    (min_y..max_y)
        .map(|y| {
            (min_x..max_x)
                .filter(|x| {
                    let offset = ((y * skin.width + x) * 4 + 3) as usize;
                    skin.rgba.get(offset).copied().unwrap_or(0) >= OVERLAY_ALPHA_TEST_THRESHOLD
                })
                .count()
        })
        .sum()
}

fn overlay_face_has_enough_visible_pixels(face: &[f32], skin: &SkinImage) -> bool {
    let rect = face_uv_rect(face, skin);
    let area = (rect.w * rect.h).max(1.0);
    let required = (area * OVERLAY_FACE_MIN_VISIBLE_RATIO).ceil() as usize;
    uv_rect_visible_pixel_count(skin, rect) >= required
}

fn face_uv_rect(face: &[f32], skin: &SkinImage) -> UvRect {
    let (min_u, max_u, min_v, max_v) = face.chunks_exact(VERTEX_STRIDE).fold(
        (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
        |(min_u, max_u, min_v, max_v), vertex| {
            (
                min_u.min(vertex[3]),
                max_u.max(vertex[3]),
                min_v.min(vertex[4]),
                max_v.max(vertex[4]),
            )
        },
    );

    let x = (min_u * skin.width as f32 - UV_EDGE_INSET_TEXELS).floor();
    let y = (min_v * skin.height as f32 - UV_EDGE_INSET_TEXELS).floor();
    let max_x = (max_u * skin.width as f32 + UV_EDGE_INSET_TEXELS).ceil();
    let max_y = (max_v * skin.height as f32 + UV_EDGE_INSET_TEXELS).ceil();

    UvRect::new(
        x.max(0.0),
        y.max(0.0),
        (max_x - x).max(0.0),
        (max_y - y).max(0.0),
    )
}

fn overlay_visible_pixel_minimum(part: BodyPart) -> usize {
    match part {
        BodyPart::Head => MIN_HEAD_OVERLAY_PIXELS,
        BodyPart::Body => MIN_BODY_OVERLAY_PIXELS,
        BodyPart::RightArm | BodyPart::LeftArm | BodyPart::RightLeg | BodyPart::LeftLeg => {
            MIN_LIMB_OVERLAY_PIXELS
        }
        BodyPart::Cape => usize::MAX,
    }
}

#[inline(always)]
pub const fn skin_pixels_to_meters(value: f32) -> f32 {
    value * SKIN_PIXEL_METERS
}

#[inline(always)]
pub const fn skin_pixel_vec3_to_meters(value: Vec3) -> Vec3 {
    Vec3::new(
        skin_pixels_to_meters(value.x),
        skin_pixels_to_meters(value.y),
        skin_pixels_to_meters(value.z),
    )
}

#[inline]
fn push_face(vertices: &mut Vec<f32>, positions: [Vec3; 4], uv: NormalizedUv, normal: Vec3) {
    push_face_with_winding(vertices, positions, uv, normal, false);
}

#[inline]
fn push_face_reversed(
    vertices: &mut Vec<f32>,
    positions: [Vec3; 4],
    uv: NormalizedUv,
    normal: Vec3,
) {
    push_face_with_winding(vertices, positions, uv, normal, true);
}

#[inline]
fn push_face_with_winding(
    vertices: &mut Vec<f32>,
    positions: [Vec3; 4],
    uv: NormalizedUv,
    normal: Vec3,
    reversed: bool,
) {
    let uvs = [
        [uv.left, uv.bottom],
        [uv.left, uv.top],
        [uv.right, uv.top],
        [uv.right, uv.bottom],
    ];
    let indices = if reversed {
        [0_usize, 2, 1, 0, 3, 2]
    } else {
        [0_usize, 1, 2, 0, 2, 3]
    };
    for index in indices {
        let position = positions[index];
        vertices.extend_from_slice(&[
            position.x,
            position.y,
            position.z,
            uvs[index][0],
            uvs[index][1],
            normal.x,
            normal.y,
            normal.z,
        ]);
    }
}

fn base_specs(format: SkinFormat, variant: ModelVariant) -> Vec<PartSpec> {
    let arm_width = arm_width(variant);
    let right_arm_x = -4.0 - arm_width * 0.5;
    let left_arm_x = 4.0 + arm_width * 0.5;
    let right_leg_x = -1.9;
    let left_leg_x = 1.9;
    let left_arm_uv = if matches!(format, SkinFormat::Modern64x64) {
        left_arm_uv(variant)
    } else {
        right_arm_uv(variant)
    };
    let left_leg_uv = if matches!(format, SkinFormat::Modern64x64) {
        FaceUvs {
            top: UvRect::new(20.0, 48.0, 4.0, 4.0),
            bottom: UvRect::new(24.0, 48.0, 4.0, 4.0),
            right: UvRect::new(16.0, 52.0, 4.0, 12.0),
            front: UvRect::new(20.0, 52.0, 4.0, 12.0),
            left: UvRect::new(24.0, 52.0, 4.0, 12.0),
            back: UvRect::new(28.0, 52.0, 4.0, 12.0),
        }
    } else {
        leg_uv()
    };

    vec![
        PartSpec {
            part: BodyPart::Head,
            center: Vec3::new(0.0, 28.0, 0.0),
            size: Vec3::new(8.0, 8.0, 8.0),
            pivot: Vec3::new(0.0, 24.0, 0.0),
            uvs: head_uv(),
            overlay: false,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::Body,
            center: Vec3::new(0.0, 18.0, 0.0),
            size: Vec3::new(8.0, 12.0, 4.0),
            pivot: Vec3::new(0.0, 18.0, 0.0),
            uvs: body_uv(),
            overlay: false,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::RightArm,
            center: Vec3::new(right_arm_x, 18.0, 0.0),
            size: Vec3::new(arm_width, 12.0, 4.0),
            pivot: Vec3::new(-5.0, 22.0, 0.0),
            uvs: right_arm_uv(variant),
            overlay: false,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::LeftArm,
            center: Vec3::new(left_arm_x, 18.0, 0.0),
            size: Vec3::new(arm_width, 12.0, 4.0),
            pivot: Vec3::new(5.0, 22.0, 0.0),
            uvs: left_arm_uv,
            overlay: false,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::RightLeg,
            center: Vec3::new(right_leg_x, 6.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(right_leg_x, 12.0, 0.0),
            uvs: leg_uv(),
            overlay: false,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::LeftLeg,
            center: Vec3::new(left_leg_x, 6.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(left_leg_x, 12.0, 0.0),
            uvs: left_leg_uv,
            overlay: false,
            inflate: 1.0,
            hidden_top: false,
        },
    ]
}

fn overlay_specs(variant: ModelVariant) -> Vec<PartSpec> {
    let arm_width = arm_width(variant);
    let arm_overlay_width = arm_width + OUTER_LAYER_DILATION * 2.0;
    let arm_overlay_size = Vec3::new(
        arm_overlay_width,
        12.0 + OUTER_LAYER_DILATION * 2.0,
        4.0 + OUTER_LAYER_DILATION * 2.0,
    );
    let right_arm_x = -4.0 - arm_width * 0.5;
    let left_arm_x = 4.0 + arm_width * 0.5;
    let right_arm_overlay_x = right_arm_x;
    let left_arm_overlay_x = left_arm_x;
    let right_leg_x = -1.9;
    let left_leg_x = 1.9;
    vec![
        hat_spec(),
        PartSpec {
            part: BodyPart::Body,
            center: Vec3::new(0.0, 18.0, 0.0),
            size: BODY_OVERLAY_SIZE,
            pivot: Vec3::new(0.0, 18.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(20.0, 32.0, 8.0, 4.0),
                bottom: UvRect::new(28.0, 32.0, 8.0, 4.0),
                right: UvRect::new(16.0, 36.0, 4.0, 12.0),
                front: UvRect::new(20.0, 36.0, 8.0, 12.0),
                left: UvRect::new(28.0, 36.0, 4.0, 12.0),
                back: UvRect::new(32.0, 36.0, 8.0, 12.0),
            },
            overlay: true,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::RightArm,
            center: Vec3::new(right_arm_overlay_x, 18.0, 0.0),
            size: arm_overlay_size,
            pivot: Vec3::new(-5.0, 22.0, 0.0),
            uvs: right_sleeve_uv(variant),
            overlay: true,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::LeftArm,
            center: Vec3::new(left_arm_overlay_x, 18.0, 0.0),
            size: arm_overlay_size,
            pivot: Vec3::new(5.0, 22.0, 0.0),
            uvs: left_sleeve_uv(variant),
            overlay: true,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::RightLeg,
            center: Vec3::new(right_leg_x, 6.0, 0.0),
            size: LEG_OVERLAY_SIZE,
            pivot: Vec3::new(right_leg_x, 12.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(4.0, 32.0, 4.0, 4.0),
                bottom: UvRect::new(8.0, 32.0, 4.0, 4.0),
                right: UvRect::new(0.0, 36.0, 4.0, 12.0),
                front: UvRect::new(4.0, 36.0, 4.0, 12.0),
                left: UvRect::new(8.0, 36.0, 4.0, 12.0),
                back: UvRect::new(12.0, 36.0, 4.0, 12.0),
            },
            overlay: true,
            inflate: 1.0,
            hidden_top: false,
        },
        PartSpec {
            part: BodyPart::LeftLeg,
            center: Vec3::new(left_leg_x, 6.0, 0.0),
            size: LEG_OVERLAY_SIZE,
            pivot: Vec3::new(left_leg_x, 12.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(4.0, 48.0, 4.0, 4.0),
                bottom: UvRect::new(8.0, 48.0, 4.0, 4.0),
                right: UvRect::new(0.0, 52.0, 4.0, 12.0),
                front: UvRect::new(4.0, 52.0, 4.0, 12.0),
                left: UvRect::new(8.0, 52.0, 4.0, 12.0),
                back: UvRect::new(12.0, 52.0, 4.0, 12.0),
            },
            overlay: true,
            inflate: 1.0,
            hidden_top: false,
        },
    ]
}

fn skinview3d_uvs(
    format: SkinFormat,
    variant: ModelVariant,
    part: BodyPart,
    overlay: bool,
) -> FaceUvs {
    match (part, overlay) {
        (BodyPart::Head, false) => head_uv(),
        (BodyPart::Head, true) => hat_spec().uvs,
        (BodyPart::Body, false) => body_uv(),
        (BodyPart::Body, true) => FaceUvs {
            top: UvRect::new(20.0, 32.0, 8.0, 4.0),
            bottom: UvRect::new(28.0, 32.0, 8.0, 4.0),
            right: UvRect::new(16.0, 36.0, 4.0, 12.0),
            front: UvRect::new(20.0, 36.0, 8.0, 12.0),
            left: UvRect::new(28.0, 36.0, 4.0, 12.0),
            back: UvRect::new(32.0, 36.0, 8.0, 12.0),
        },
        (BodyPart::RightArm, false) => right_arm_uv(variant),
        (BodyPart::RightArm, true) => right_sleeve_uv(variant),
        (BodyPart::LeftArm, false) => {
            if matches!(format, SkinFormat::Modern64x64) {
                left_arm_uv(variant)
            } else {
                right_arm_uv(variant)
            }
        }
        (BodyPart::LeftArm, true) => left_sleeve_uv(variant),
        (BodyPart::RightLeg, false) => leg_uv(),
        (BodyPart::RightLeg, true) => FaceUvs {
            top: UvRect::new(4.0, 32.0, 4.0, 4.0),
            bottom: UvRect::new(8.0, 32.0, 4.0, 4.0),
            right: UvRect::new(0.0, 36.0, 4.0, 12.0),
            front: UvRect::new(4.0, 36.0, 4.0, 12.0),
            left: UvRect::new(8.0, 36.0, 4.0, 12.0),
            back: UvRect::new(12.0, 36.0, 4.0, 12.0),
        },
        (BodyPart::LeftLeg, false) => {
            if matches!(format, SkinFormat::Modern64x64) {
                FaceUvs {
                    top: UvRect::new(20.0, 48.0, 4.0, 4.0),
                    bottom: UvRect::new(24.0, 48.0, 4.0, 4.0),
                    right: UvRect::new(16.0, 52.0, 4.0, 12.0),
                    front: UvRect::new(20.0, 52.0, 4.0, 12.0),
                    left: UvRect::new(24.0, 52.0, 4.0, 12.0),
                    back: UvRect::new(28.0, 52.0, 4.0, 12.0),
                }
            } else {
                leg_uv()
            }
        }
        (BodyPart::LeftLeg, true) => FaceUvs {
            top: UvRect::new(4.0, 48.0, 4.0, 4.0),
            bottom: UvRect::new(8.0, 48.0, 4.0, 4.0),
            right: UvRect::new(0.0, 52.0, 4.0, 12.0),
            front: UvRect::new(4.0, 52.0, 4.0, 12.0),
            left: UvRect::new(8.0, 52.0, 4.0, 12.0),
            back: UvRect::new(12.0, 52.0, 4.0, 12.0),
        },
        (BodyPart::Cape, _) => body_uv(),
    }
}

fn hat_spec() -> PartSpec {
    PartSpec {
        part: BodyPart::Head,
        center: Vec3::new(0.0, 28.0, 0.0),
        size: HEAD_OVERLAY_SIZE,
        pivot: Vec3::new(0.0, 24.0, 0.0),
        uvs: FaceUvs {
            top: UvRect::new(40.0, 0.0, 8.0, 8.0),
            bottom: UvRect::new(48.0, 0.0, 8.0, 8.0),
            right: UvRect::new(32.0, 8.0, 8.0, 8.0),
            front: UvRect::new(40.0, 8.0, 8.0, 8.0),
            left: UvRect::new(48.0, 8.0, 8.0, 8.0),
            back: UvRect::new(56.0, 8.0, 8.0, 8.0),
        },
        overlay: true,
        inflate: 1.0,
        hidden_top: false,
    }
}

fn head_uv() -> FaceUvs {
    FaceUvs {
        top: UvRect::new(8.0, 0.0, 8.0, 8.0),
        bottom: UvRect::new(16.0, 0.0, 8.0, 8.0),
        right: UvRect::new(0.0, 8.0, 8.0, 8.0),
        front: UvRect::new(8.0, 8.0, 8.0, 8.0),
        left: UvRect::new(16.0, 8.0, 8.0, 8.0),
        back: UvRect::new(24.0, 8.0, 8.0, 8.0),
    }
}

fn body_uv() -> FaceUvs {
    FaceUvs {
        top: UvRect::new(20.0, 16.0, 8.0, 4.0),
        bottom: UvRect::new(28.0, 16.0, 8.0, 4.0),
        right: UvRect::new(16.0, 20.0, 4.0, 12.0),
        front: UvRect::new(20.0, 20.0, 8.0, 12.0),
        left: UvRect::new(28.0, 20.0, 4.0, 12.0),
        back: UvRect::new(32.0, 20.0, 8.0, 12.0),
    }
}

fn leg_uv() -> FaceUvs {
    FaceUvs {
        top: UvRect::new(4.0, 16.0, 4.0, 4.0),
        bottom: UvRect::new(8.0, 16.0, 4.0, 4.0),
        right: UvRect::new(0.0, 20.0, 4.0, 12.0),
        front: UvRect::new(4.0, 20.0, 4.0, 12.0),
        left: UvRect::new(8.0, 20.0, 4.0, 12.0),
        back: UvRect::new(12.0, 20.0, 4.0, 12.0),
    }
}

#[inline]
fn arm_width(variant: ModelVariant) -> f32 {
    match variant {
        ModelVariant::Classic => 4.0,
        ModelVariant::Slim => 3.0,
    }
}

fn right_arm_uv(variant: ModelVariant) -> FaceUvs {
    arm_uv(
        variant,
        ArmUvLayout {
            top_x: 44.0,
            bottom_x: 48.0,
            right_x: 40.0,
            front_x: 44.0,
            left_x: 48.0,
            back_x: 52.0,
            cap_y: 16.0,
            side_y: 20.0,
        },
    )
}

fn left_arm_uv(variant: ModelVariant) -> FaceUvs {
    arm_uv(
        variant,
        ArmUvLayout {
            top_x: 36.0,
            bottom_x: 40.0,
            right_x: 32.0,
            front_x: 36.0,
            left_x: 40.0,
            back_x: 44.0,
            cap_y: 48.0,
            side_y: 52.0,
        },
    )
}

fn right_sleeve_uv(variant: ModelVariant) -> FaceUvs {
    arm_uv(
        variant,
        ArmUvLayout {
            top_x: 44.0,
            bottom_x: 48.0,
            right_x: 40.0,
            front_x: 44.0,
            left_x: 48.0,
            back_x: 52.0,
            cap_y: 32.0,
            side_y: 36.0,
        },
    )
}

fn left_sleeve_uv(variant: ModelVariant) -> FaceUvs {
    arm_uv(
        variant,
        ArmUvLayout {
            top_x: 52.0,
            bottom_x: 56.0,
            right_x: 48.0,
            front_x: 52.0,
            left_x: 56.0,
            back_x: 60.0,
            cap_y: 48.0,
            side_y: 52.0,
        },
    )
}

#[inline]
fn arm_uv(variant: ModelVariant, layout: ArmUvLayout) -> FaceUvs {
    let width = arm_width(variant);
    FaceUvs {
        top: UvRect::new(layout.top_x, layout.cap_y, width, 4.0),
        bottom: UvRect::new(layout.bottom_x - (4.0 - width), layout.cap_y, width, 4.0),
        right: UvRect::new(layout.right_x, layout.side_y, 4.0, 12.0),
        front: UvRect::new(layout.front_x, layout.side_y, width, 12.0),
        left: UvRect::new(layout.left_x - (4.0 - width), layout.side_y, 4.0, 12.0),
        back: UvRect::new(layout.back_x - (4.0 - width), layout.side_y, width, 12.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_pixel_uv_to_normalized_webgl_uv() {
        let uv = UvRect::new(8.0, 8.0, 8.0, 8.0).normalized(64.0, 64.0);
        assert!((uv.left - ((8.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((uv.right - ((16.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((uv.top - ((8.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((uv.bottom - ((16.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
    }

    #[test]
    fn uv_inset_avoids_neighbor_bleed_without_shrinking_to_texel_centers() {
        let uv = UvRect::new(8.0, 8.0, 8.0, 8.0).normalized(64.0, 64.0);

        assert!(uv.left > 8.0 / 64.0);
        assert!(uv.right < 16.0 / 64.0);
        assert!(uv.left < 8.01 / 64.0);
        assert!(uv.right > 15.99 / 64.0);
    }

    #[test]
    fn cuboid_generation_has_expected_vertex_count() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        assert_eq!(meshes[0].vertex_count, 36);
        assert_eq!(meshes[0].vertices.len(), 36 * VERTEX_STRIDE);
    }

    #[test]
    fn cuboid_faces_are_front_facing_for_webgl_backface_culling() {
        let meshes = build_player_meshes_for_style(
            SkinFormat::Modern64x64,
            ModelVariant::Classic,
            PlayerModelStyle::Skinview3d1To1,
        );

        for mesh in meshes {
            for face in mesh.vertices.chunks_exact(6 * VERTEX_STRIDE) {
                let a = vertex_position(&face[0..VERTEX_STRIDE]);
                let b = vertex_position(&face[VERTEX_STRIDE..VERTEX_STRIDE * 2]);
                let c = vertex_position(&face[VERTEX_STRIDE * 2..VERTEX_STRIDE * 3]);
                let winding_normal = cross(sub(b, a), sub(c, a));
                let stored_normal = vertex_normal(&face[0..VERTEX_STRIDE]);
                let dot = winding_normal.x * stored_normal.x
                    + winding_normal.y * stored_normal.y
                    + winding_normal.z * stored_normal.z;

                assert!(
                    dot > 0.0,
                    "mesh {:?} overlay={} has reversed face winding",
                    mesh.part,
                    mesh.overlay
                );
            }
        }
    }

    #[test]
    fn base_head_mesh_is_exactly_eight_by_eight_by_eight() {
        let bounds = debug_head_bounds();

        assert_skin_pixel_vec3_close(bounds.size, Vec3::new(8.0, 8.0, 8.0));
        assert_skin_pixel_vec3_close(bounds.center, Vec3::new(0.0, 28.0, 0.0));
        assert_skin_pixel_vec3_close(bounds.min, Vec3::new(-4.0, 24.0, -4.0));
        assert_skin_pixel_vec3_close(bounds.max, Vec3::new(4.0, 32.0, 4.0));
    }

    #[test]
    fn base_head_front_face_uses_eight_by_eight_uv_region() {
        assert_eq!(head_uv().front, UvRect::new(8.0, 8.0, 8.0, 8.0));

        let normalized = head_uv().front.normalized(64.0, 64.0);
        assert!((normalized.left - ((8.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((normalized.right - ((16.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((normalized.top - ((8.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((normalized.bottom - ((16.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
    }

    #[test]
    fn base_head_front_face_spans_full_visual_height() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let head = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::Head && !mesh.overlay)
            .expect("head mesh exists");
        let front_face_start = 18 * VERTEX_STRIDE;
        let front_face_end = 24 * VERTEX_STRIDE;
        let (min_y, max_y) = head.vertices[front_face_start..front_face_end]
            .chunks_exact(VERTEX_STRIDE)
            .map(|vertex| vertex[1])
            .fold((f32::MAX, f32::MIN), |(min_y, max_y), y| {
                (min_y.min(y), max_y.max(y))
            });

        assert!((min_y - skin_pixels_to_meters(24.0)).abs() < 0.0001);
        assert!((max_y - skin_pixels_to_meters(32.0)).abs() < 0.0001);
    }

    #[test]
    fn cape_generation_has_expected_vertex_count() {
        let cape = build_cape_mesh(64.0, 32.0);
        assert_eq!(cape.vertex_count, 36);
        assert_eq!(cape.vertices.len(), 36 * VERTEX_STRIDE);
        assert_eq!(cape.part, BodyPart::Cape);
    }

    #[test]
    fn cape_outer_back_uses_visible_cape_region() {
        let cape = build_cape_mesh(64.0, 32.0);
        let back_face_first_vertex = 30 * VERTEX_STRIDE;
        let u = cape.vertices[back_face_first_vertex + 3];
        let v = cape.vertices[back_face_first_vertex + 4];

        assert!((u - ((1.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((v - ((17.0 - UV_EDGE_INSET_TEXELS) / 32.0)).abs() < 0.0001);
    }

    #[test]
    fn cape_front_rests_just_behind_body_overlay() {
        let cape = build_cape_mesh(64.0, 32.0);
        let bounds = mesh_debug_bounds(&cape);

        assert!((bounds.max.z - skin_pixels_to_meters(-2.26)).abs() < 0.0001);
        assert!(bounds.max.z < skin_pixels_to_meters(-2.25));
        assert_skin_pixel_vec3_close(cape.pivot, Vec3::new(0.0, 24.0, -2.25));
    }

    #[test]
    fn slim_arm_uses_three_pixel_width() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Slim);
        let right_arm = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightArm && !mesh.overlay)
            .expect("right arm mesh exists");
        let xs = right_arm
            .vertices
            .chunks_exact(VERTEX_STRIDE)
            .map(|vertex| vertex[0]);
        let (min_x, max_x) = xs.fold((f32::MAX, f32::MIN), |(min_x, max_x), x| {
            (min_x.min(x), max_x.max(x))
        });
        assert!(((max_x - min_x) - skin_pixels_to_meters(3.0)).abs() < 0.0001);
    }

    #[test]
    fn classic_and_slim_variants_only_change_arm_widths() {
        let classic = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let slim = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Slim);
        let classic_right_arm = mesh(&classic, BodyPart::RightArm, false);
        let slim_right_arm = mesh(&slim, BodyPart::RightArm, false);
        let classic_left_arm = mesh(&classic, BodyPart::LeftArm, false);
        let slim_left_arm = mesh(&slim, BodyPart::LeftArm, false);

        assert_skin_pixel_vec3_close(dimensions(classic_right_arm), Vec3::new(4.0, 12.0, 4.0));
        assert_skin_pixel_vec3_close(dimensions(classic_left_arm), Vec3::new(4.0, 12.0, 4.0));
        assert_skin_pixel_vec3_close(dimensions(slim_right_arm), Vec3::new(3.0, 12.0, 4.0));
        assert_skin_pixel_vec3_close(dimensions(slim_left_arm), Vec3::new(3.0, 12.0, 4.0));
    }

    #[test]
    fn head_and_hat_are_unchanged_between_classic_and_slim() {
        let classic = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let slim = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Slim);

        assert_vec3_close(
            dimensions(mesh(&classic, BodyPart::Head, false)),
            dimensions(mesh(&slim, BodyPart::Head, false)),
        );
        assert_vec3_close(
            dimensions(mesh(&classic, BodyPart::Head, true)),
            dimensions(mesh(&slim, BodyPart::Head, true)),
        );
        assert_skin_pixel_vec3_close(
            dimensions(mesh(&slim, BodyPart::Head, false)),
            Vec3::new(8.0, 8.0, 8.0),
        );
        assert_skin_pixel_vec3_close(
            dimensions(mesh(&slim, BodyPart::Head, true)),
            Vec3::new(9.0, 9.0, 9.0),
        );
    }

    #[test]
    fn player_bounds_include_all_classic_overlays() {
        let bounds = player_debug_bounds(SkinFormat::Modern64x64, ModelVariant::Classic);

        assert_skin_pixel_vec3_close(bounds.min, Vec3::new(-8.25, -0.25, -4.5));
        assert_skin_pixel_vec3_close(bounds.max, Vec3::new(8.25, 32.5, 4.5));
        assert_skin_pixel_vec3_close(bounds.size, Vec3::new(16.5, 32.75, 9.0));
        assert_skin_pixel_vec3_close(bounds.center, Vec3::new(0.0, 16.125, 0.0));
    }

    #[test]
    fn player_bounds_include_all_slim_overlays() {
        let bounds = player_debug_bounds(SkinFormat::Modern64x64, ModelVariant::Slim);

        assert_skin_pixel_vec3_close(bounds.min, Vec3::new(-7.25, -0.25, -4.5));
        assert_skin_pixel_vec3_close(bounds.max, Vec3::new(7.25, 32.5, 4.5));
        assert_skin_pixel_vec3_close(bounds.size, Vec3::new(14.5, 32.75, 9.0));
        assert_skin_pixel_vec3_close(bounds.center, Vec3::new(0.0, 16.125, 0.0));
    }

    #[test]
    fn limb_meshes_are_closed_cuboids() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let right_leg = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightLeg && !mesh.overlay)
            .expect("right leg mesh exists");
        let right_arm = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightArm && !mesh.overlay)
            .expect("right arm mesh exists");

        assert_eq!(right_leg.vertex_count, 36);
        assert_eq!(right_arm.vertex_count, 36);
    }

    #[test]
    fn head_layers_match_vanilla_dimensions_and_pivot() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let head = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::Head && !mesh.overlay)
            .expect("head mesh exists");
        let headwear = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::Head && mesh.overlay)
            .expect("headwear mesh exists");

        assert_skin_pixel_vec3_close(dimensions(head), Vec3::new(8.0, 8.0, 8.0));
        assert_skin_pixel_vec3_close(dimensions(headwear), Vec3::new(9.0, 9.0, 9.0));
        assert_skin_pixel_vec3_close(center(head), Vec3::new(0.0, 28.0, 0.0));
        assert_skin_pixel_vec3_close(center(headwear), Vec3::new(0.0, 28.0, 0.0));
        assert_eq!(head.pivot, headwear.pivot);
        assert_eq!(
            head.pivot,
            skin_pixel_vec3_to_meters(Vec3::new(0.0, 24.0, 0.0))
        );
    }

    #[test]
    fn outer_layers_use_real_dilation_dimensions() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let body = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::Body && mesh.overlay)
            .expect("jacket mesh exists");
        let right_arm = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightArm && mesh.overlay)
            .expect("right sleeve mesh exists");
        let right_leg = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightLeg && mesh.overlay)
            .expect("right pants mesh exists");

        assert_skin_pixel_vec3_close(dimensions(body), Vec3::new(8.5, 12.5, 4.5));
        assert_skin_pixel_vec3_close(dimensions(right_arm), Vec3::new(4.5, 12.5, 4.5));
        assert_skin_pixel_vec3_close(dimensions(right_leg), Vec3::new(4.5, 12.5, 4.5));
    }

    #[test]
    fn slim_outer_arms_expand_by_half_a_pixel() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Slim);
        let right_arm = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightArm && mesh.overlay)
            .expect("right sleeve mesh exists");

        assert_skin_pixel_vec3_close(dimensions(right_arm), Vec3::new(3.5, 12.5, 4.5));
    }

    #[test]
    fn headwear_uses_modern_second_layer_uv_region() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let headwear = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::Head && mesh.overlay)
            .expect("headwear mesh exists");
        let (min_u, max_u, min_v, max_v) = uv_bounds(headwear);

        assert!((min_u - ((32.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((max_u - ((64.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((min_v - ((0.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((max_v - ((16.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
    }

    #[test]
    fn legacy_64x32_only_builds_head_overlay_layer() {
        let meshes = build_player_meshes(SkinFormat::Legacy64x32, ModelVariant::Classic);
        let overlay_parts = meshes
            .iter()
            .filter(|mesh| mesh.overlay)
            .map(|mesh| mesh.part)
            .collect::<Vec<_>>();

        assert_eq!(overlay_parts, vec![BodyPart::Head]);
    }

    #[test]
    fn transparent_legacy_hat_layer_is_not_visible() {
        let skin = transparent_skin(64, 32, SkinFormat::Legacy64x32);

        assert!(!overlay_part_has_visible_pixels(&skin, BodyPart::Head));
        assert!(!overlay_part_has_visible_pixels(&skin, BodyPart::Body));
    }

    #[test]
    fn visible_overlay_pixels_enable_only_their_part() {
        let mut skin = transparent_skin(64, 64, SkinFormat::Modern64x64);
        fill_alpha(&mut skin, 40, 8, 8, 8, 255);
        fill_alpha(&mut skin, 20, 36, 8, 8, 255);

        assert!(overlay_part_has_visible_pixels(&skin, BodyPart::Head));
        assert!(overlay_part_has_visible_pixels(&skin, BodyPart::Body));
        assert!(!overlay_part_has_visible_pixels(&skin, BodyPart::RightArm));
    }

    #[test]
    fn sparse_overlay_pixels_do_not_enable_whole_part() {
        let mut skin = transparent_skin(64, 64, SkinFormat::Modern64x64);
        fill_alpha(&mut skin, 20, 38, 8, 4, 255);
        fill_alpha(&mut skin, 20, 44, 8, 1, 255);

        assert!(!overlay_part_has_visible_pixels(&skin, BodyPart::Body));
    }

    #[test]
    fn sparse_headwear_faces_are_pruned_without_removing_dense_faces() {
        let mut skin = transparent_skin(64, 64, SkinFormat::Modern64x64);
        fill_alpha(&mut skin, 40, 8, 8, 4, 255);
        fill_alpha(&mut skin, 42, 12, 2, 1, 255);
        fill_alpha(&mut skin, 49, 0, 1, 1, 255);
        fill_alpha(&mut skin, 54, 0, 1, 1, 255);
        fill_alpha(&mut skin, 49, 7, 1, 1, 255);
        fill_alpha(&mut skin, 54, 7, 1, 1, 255);

        let mut meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        prune_sparse_overlay_faces(&mut meshes, &skin);
        let headwear = mesh(&meshes, BodyPart::Head, true);

        assert_eq!(headwear.vertex_count, 6);
        let (min_u, max_u, min_v, max_v) = uv_bounds(headwear);
        assert!((min_u - ((40.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((max_u - ((48.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((min_v - ((8.0 + UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
        assert!((max_v - ((16.0 - UV_EDGE_INSET_TEXELS) / 64.0)).abs() < 0.0001);
    }

    #[test]
    fn leg_meshes_use_real_visual_centers() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let right_base = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightLeg && !mesh.overlay)
            .expect("right leg mesh exists");
        let left_base = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::LeftLeg && !mesh.overlay)
            .expect("left leg mesh exists");
        let right_overlay = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightLeg && mesh.overlay)
            .expect("right pants mesh exists");
        let left_overlay = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::LeftLeg && mesh.overlay)
            .expect("left pants mesh exists");

        assert_skin_pixel_vec3_close(center(right_base), Vec3::new(-1.9, 6.0, 0.0));
        assert_skin_pixel_vec3_close(center(left_base), Vec3::new(1.9, 6.0, 0.0));
        assert_skin_pixel_vec3_close(center(right_overlay), Vec3::new(-1.9, 6.0, 0.0));
        assert_skin_pixel_vec3_close(center(left_overlay), Vec3::new(1.9, 6.0, 0.0));
        assert_eq!(
            right_base.pivot,
            skin_pixel_vec3_to_meters(Vec3::new(-1.9, 12.0, 0.0))
        );
        assert_eq!(
            left_base.pivot,
            skin_pixel_vec3_to_meters(Vec3::new(1.9, 12.0, 0.0))
        );
    }

    #[test]
    fn classic_arm_meshes_use_real_visual_centers_and_pivots() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64, ModelVariant::Classic);
        let right_base = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightArm && !mesh.overlay)
            .expect("right arm mesh exists");
        let left_base = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::LeftArm && !mesh.overlay)
            .expect("left arm mesh exists");
        let right_overlay = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::RightArm && mesh.overlay)
            .expect("right sleeve mesh exists");
        let left_overlay = meshes
            .iter()
            .find(|mesh| mesh.part == BodyPart::LeftArm && mesh.overlay)
            .expect("left sleeve mesh exists");

        assert_skin_pixel_vec3_close(center(right_base), Vec3::new(-6.0, 18.0, 0.0));
        assert_skin_pixel_vec3_close(center(left_base), Vec3::new(6.0, 18.0, 0.0));
        assert_skin_pixel_vec3_close(center(right_overlay), Vec3::new(-6.0, 18.0, 0.0));
        assert_skin_pixel_vec3_close(center(left_overlay), Vec3::new(6.0, 18.0, 0.0));
        assert_eq!(
            right_base.pivot,
            skin_pixel_vec3_to_meters(Vec3::new(-5.0, 22.0, 0.0))
        );
        assert_eq!(
            left_base.pivot,
            skin_pixel_vec3_to_meters(Vec3::new(5.0, 22.0, 0.0))
        );
    }

    #[test]
    fn skinview3d_hierarchy_has_skin_wrapper_translation() {
        let hierarchy = skinview3d_hierarchy(ModelVariant::Classic);

        assert_vec3_close(hierarchy.skin_position, Vec3::new(0.0, 8.0, 0.0));
    }

    #[test]
    fn skinview3d_head_mesh_local_position_is_four() {
        let hierarchy = skinview3d_hierarchy(ModelVariant::Classic);
        let head = skinview3d_node(&hierarchy, BodyPart::Head, false);

        assert_vec3_close(head.mesh_local_position, Vec3::new(0.0, 4.0, 0.0));
        assert_vec3_close(head.size, Vec3::new(8.0, 8.0, 8.0));
    }

    #[test]
    fn skinview3d_body_mesh_local_position_is_minus_six() {
        let hierarchy = skinview3d_hierarchy(ModelVariant::Classic);
        let body = skinview3d_node(&hierarchy, BodyPart::Body, false);

        assert_vec3_close(body.mesh_local_position, Vec3::new(0.0, -6.0, 0.0));
        assert_vec3_close(body.size, Vec3::new(8.0, 12.0, 4.0));
    }

    #[test]
    fn skinview3d_leg_group_positions_match_skinview3d() {
        let hierarchy = skinview3d_hierarchy(ModelVariant::Classic);
        let right_leg = skinview3d_node(&hierarchy, BodyPart::RightLeg, false);
        let left_leg = skinview3d_node(&hierarchy, BodyPart::LeftLeg, false);

        assert_vec3_close(right_leg.group_position, Vec3::new(-1.9, -12.0, -0.1));
        assert_vec3_close(left_leg.group_position, Vec3::new(1.9, -12.0, -0.1));
    }

    #[test]
    fn skinview3d_head_and_hat_are_identical_between_classic_and_slim() {
        let classic = skinview3d_hierarchy(ModelVariant::Classic);
        let slim = skinview3d_hierarchy(ModelVariant::Slim);

        for overlay in [false, true] {
            let classic_node = skinview3d_node(&classic, BodyPart::Head, overlay);
            let slim_node = skinview3d_node(&slim, BodyPart::Head, overlay);
            assert_vec3_close(classic_node.group_position, slim_node.group_position);
            assert_vec3_close(
                classic_node.mesh_local_position,
                slim_node.mesh_local_position,
            );
            assert_vec3_close(classic_node.size, slim_node.size);
            assert_vec3_close(classic_node.pivot, slim_node.pivot);
        }
    }

    #[test]
    fn skinview3d_slim_only_changes_arm_and_sleeve_widths() {
        let classic = skinview3d_hierarchy(ModelVariant::Classic);
        let slim = skinview3d_hierarchy(ModelVariant::Slim);

        for classic_node in &classic.parts {
            let slim_node = skinview3d_node(&slim, classic_node.part, classic_node.overlay);
            if matches!(classic_node.part, BodyPart::RightArm | BodyPart::LeftArm) {
                let expected_width = if classic_node.overlay { 3.5 } else { 3.0 };
                assert!((slim_node.size.x - expected_width).abs() < 0.0001);
                assert_vec3_close(
                    Vec3::new(0.0, slim_node.size.y, slim_node.size.z),
                    Vec3::new(0.0, classic_node.size.y, classic_node.size.z),
                );
            } else {
                assert_vec3_close(slim_node.group_position, classic_node.group_position);
                assert_vec3_close(
                    slim_node.mesh_local_position,
                    classic_node.mesh_local_position,
                );
                assert_vec3_close(slim_node.size, classic_node.size);
            }
        }
    }

    #[test]
    fn skinview3d_player_bounds_are_centered_on_skinview3d_origin() {
        let bounds = player_debug_bounds_for_style(
            SkinFormat::Modern64x64,
            ModelVariant::Classic,
            PlayerModelStyle::Skinview3d1To1,
        );

        assert_skin_pixel_vec3_close(bounds.min, Vec3::new(-8.25, -10.25, -4.5));
        assert_skin_pixel_vec3_close(bounds.max, Vec3::new(8.25, 16.5, 4.5));
        assert_skin_pixel_vec3_close(bounds.size, Vec3::new(16.5, 26.75, 9.0));
    }

    #[test]
    fn skinview3d_final_base_centers_match_expected_static_pose() {
        let parts = player_part_debug_bounds_for_style(
            SkinFormat::Modern64x64,
            ModelVariant::Classic,
            PlayerModelStyle::Skinview3d1To1,
        );

        assert_skin_pixel_vec3_close(
            part_bounds(&parts, BodyPart::Head, false).center,
            Vec3::new(0.0, 12.0, 0.0),
        );
        assert_skin_pixel_vec3_close(
            part_bounds(&parts, BodyPart::Body, false).center,
            Vec3::new(0.0, 2.0, 0.0),
        );
        assert_skin_pixel_vec3_close(
            part_bounds(&parts, BodyPart::RightLeg, false).center,
            Vec3::new(-1.9, -4.0, -0.1),
        );
        assert_skin_pixel_vec3_close(
            part_bounds(&parts, BodyPart::LeftLeg, false).center,
            Vec3::new(1.9, -4.0, -0.1),
        );
    }

    #[test]
    fn skinview3d_final_overlay_centers_follow_base_parts() {
        let parts = player_part_debug_bounds_for_style(
            SkinFormat::Modern64x64,
            ModelVariant::Classic,
            PlayerModelStyle::Skinview3d1To1,
        );

        assert_vec3_close(
            part_bounds(&parts, BodyPart::Head, true).center,
            part_bounds(&parts, BodyPart::Head, false).center,
        );
        assert_vec3_close(
            part_bounds(&parts, BodyPart::Body, true).center,
            part_bounds(&parts, BodyPart::Body, false).center,
        );
        assert_vec3_close(
            part_bounds(&parts, BodyPart::RightArm, true).center,
            part_bounds(&parts, BodyPart::RightArm, false).center,
        );
        assert_vec3_close(
            part_bounds(&parts, BodyPart::LeftArm, true).center,
            part_bounds(&parts, BodyPart::LeftArm, false).center,
        );
        assert_vec3_close(
            part_bounds(&parts, BodyPart::RightLeg, true).center,
            part_bounds(&parts, BodyPart::RightLeg, false).center,
        );
        assert_vec3_close(
            part_bounds(&parts, BodyPart::LeftLeg, true).center,
            part_bounds(&parts, BodyPart::LeftLeg, false).center,
        );
    }

    #[test]
    fn skinview3d_final_vertical_ranges_match_expected_static_pose() {
        let parts = player_part_debug_bounds_for_style(
            SkinFormat::Modern64x64,
            ModelVariant::Classic,
            PlayerModelStyle::Skinview3d1To1,
        );

        assert_skin_pixel_vec3_close(
            vertical_range(part_bounds(&parts, BodyPart::Head, false)),
            Vec3::new(0.0, 8.0, 16.0),
        );
        assert_skin_pixel_vec3_close(
            vertical_range(part_bounds(&parts, BodyPart::Head, true)),
            Vec3::new(0.0, 7.5, 16.5),
        );
        assert_skin_pixel_vec3_close(
            vertical_range(part_bounds(&parts, BodyPart::Body, false)),
            Vec3::new(0.0, -4.0, 8.0),
        );
        assert_skin_pixel_vec3_close(
            vertical_range(part_bounds(&parts, BodyPart::RightLeg, false)),
            Vec3::new(0.0, -10.0, 2.0),
        );
        assert_skin_pixel_vec3_close(
            vertical_range(part_bounds(&parts, BodyPart::LeftLeg, false)),
            Vec3::new(0.0, -10.0, 2.0),
        );
    }

    fn dimensions(mesh: &Mesh) -> Vec3 {
        let (min, max) = position_bounds(mesh);
        Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z)
    }

    fn center(mesh: &Mesh) -> Vec3 {
        let (min, max) = position_bounds(mesh);
        Vec3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            (min.z + max.z) * 0.5,
        )
    }

    fn position_bounds(mesh: &Mesh) -> (Vec3, Vec3) {
        mesh.vertices.chunks_exact(VERTEX_STRIDE).fold(
            (
                Vec3::new(f32::MAX, f32::MAX, f32::MAX),
                Vec3::new(f32::MIN, f32::MIN, f32::MIN),
            ),
            |(min, max), vertex| {
                (
                    Vec3::new(
                        min.x.min(vertex[0]),
                        min.y.min(vertex[1]),
                        min.z.min(vertex[2]),
                    ),
                    Vec3::new(
                        max.x.max(vertex[0]),
                        max.y.max(vertex[1]),
                        max.z.max(vertex[2]),
                    ),
                )
            },
        )
    }

    fn uv_bounds(mesh: &Mesh) -> (f32, f32, f32, f32) {
        mesh.vertices.chunks_exact(VERTEX_STRIDE).fold(
            (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
            |(min_u, max_u, min_v, max_v), vertex| {
                (
                    min_u.min(vertex[3]),
                    max_u.max(vertex[3]),
                    min_v.min(vertex[4]),
                    max_v.max(vertex[4]),
                )
            },
        )
    }

    fn vertex_position(vertex: &[f32]) -> Vec3 {
        Vec3::new(vertex[0], vertex[1], vertex[2])
    }

    fn vertex_normal(vertex: &[f32]) -> Vec3 {
        Vec3::new(vertex[5], vertex[6], vertex[7])
    }

    fn sub(a: Vec3, b: Vec3) -> Vec3 {
        Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
    }

    fn cross(a: Vec3, b: Vec3) -> Vec3 {
        Vec3::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!((actual.x - expected.x).abs() < 0.0001);
        assert!((actual.y - expected.y).abs() < 0.0001);
        assert!((actual.z - expected.z).abs() < 0.0001);
    }

    fn assert_skin_pixel_vec3_close(actual: Vec3, expected_skin_pixels: Vec3) {
        assert_vec3_close(actual, skin_pixel_vec3_to_meters(expected_skin_pixels));
    }

    fn transparent_skin(width: u32, height: u32, format: SkinFormat) -> SkinImage {
        SkinImage {
            width,
            height,
            format,
            model: ModelVariant::Classic,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    fn set_alpha(skin: &mut SkinImage, x: u32, y: u32, alpha: u8) {
        let offset = ((y * skin.width + x) * 4 + 3) as usize;
        skin.rgba[offset] = alpha;
    }

    fn fill_alpha(skin: &mut SkinImage, x: u32, y: u32, width: u32, height: u32, alpha: u8) {
        for py in y..y + height {
            for px in x..x + width {
                set_alpha(skin, px, py, alpha);
            }
        }
    }

    fn mesh(meshes: &[Mesh], part: BodyPart, overlay: bool) -> &Mesh {
        meshes
            .iter()
            .find(|mesh| mesh.part == part && mesh.overlay == overlay)
            .expect("mesh exists")
    }

    fn skinview3d_node(
        hierarchy: &Skinview3dHierarchy,
        part: BodyPart,
        overlay: bool,
    ) -> Skinview3dPartNode {
        hierarchy
            .parts
            .iter()
            .copied()
            .find(|node| node.part == part && node.overlay == overlay)
            .expect("skinview3d node exists")
    }

    fn part_bounds(
        parts: &[MeshPartDebugBounds],
        part: BodyPart,
        overlay: bool,
    ) -> MeshDebugBounds {
        parts
            .iter()
            .find(|bounds| bounds.part == part && bounds.overlay == overlay)
            .expect("part bounds exist")
            .bounds
    }

    fn vertical_range(bounds: MeshDebugBounds) -> Vec3 {
        Vec3::new(0.0, bounds.min.y, bounds.max.y)
    }
}
