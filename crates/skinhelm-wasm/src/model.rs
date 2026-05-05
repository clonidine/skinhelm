use crate::animation::WalkPose;
use crate::math::{Mat4, Vec3};
use crate::skin::SkinFormat;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UvRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn normalized(self, skin_width: f32, skin_height: f32) -> NormalizedUv {
        NormalizedUv {
            left: self.x / skin_width,
            right: (self.x + self.w) / skin_width,
            top: 1.0 - self.y / skin_height,
            bottom: 1.0 - (self.y + self.h) / skin_height,
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
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<f32>,
    pub vertex_count: i32,
    pub part: BodyPart,
    pub overlay: bool,
    pub pivot: Vec3,
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

#[derive(Debug, Clone, Copy)]
struct PartSpec {
    part: BodyPart,
    center: Vec3,
    size: Vec3,
    pivot: Vec3,
    uvs: FaceUvs,
    overlay: bool,
    inflate: f32,
}

pub fn build_player_meshes(format: SkinFormat) -> Vec<Mesh> {
    let skin_height = match format {
        SkinFormat::Modern64x64 => 64.0,
        SkinFormat::Legacy64x32 => 32.0,
    };
    let mut specs = base_specs(format);
    if matches!(format, SkinFormat::Modern64x64) {
        specs.extend(overlay_specs());
    } else {
        specs.push(hat_spec());
    }

    specs
        .into_iter()
        .map(|spec| build_mesh(spec, 64.0, skin_height))
        .collect()
}

pub fn part_model_matrix(part: BodyPart, pivot: Vec3, pose: WalkPose) -> Mat4 {
    let angle = match part {
        BodyPart::RightLeg => pose.right_leg_x,
        BodyPart::LeftLeg => pose.left_leg_x,
        BodyPart::RightArm => pose.right_arm_x,
        BodyPart::LeftArm => pose.left_arm_x,
        BodyPart::Head | BodyPart::Body => 0.0,
    };
    let bob = match part {
        BodyPart::Head => pose.body_y + pose.head_y,
        BodyPart::Body | BodyPart::RightArm | BodyPart::LeftArm => pose.body_y,
        BodyPart::RightLeg | BodyPart::LeftLeg => 0.0,
    };

    Mat4::translation(0.0, bob, 0.0)
        .multiply(Mat4::translation(pivot.x, pivot.y, pivot.z))
        .multiply(Mat4::rotation_x(angle))
        .multiply(Mat4::translation(-pivot.x, -pivot.y, -pivot.z))
}

fn build_mesh(spec: PartSpec, skin_width: f32, skin_height: f32) -> Mesh {
    let mut vertices = Vec::with_capacity(36 * 5);
    let half = Vec3::new(
        spec.size.x * spec.inflate * 0.5,
        spec.size.y * spec.inflate * 0.5,
        spec.size.z * spec.inflate * 0.5,
    );
    let min = Vec3::new(
        spec.center.x - half.x,
        spec.center.y - half.y,
        spec.center.z - half.z,
    );
    let max = Vec3::new(
        spec.center.x + half.x,
        spec.center.y + half.y,
        spec.center.z + half.z,
    );

    push_face(
        &mut vertices,
        [
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, max.y, max.z),
        ],
        spec.uvs.top.normalized(skin_width, skin_height),
    );
    push_face(
        &mut vertices,
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        spec.uvs.bottom.normalized(skin_width, skin_height),
    );
    push_face(
        &mut vertices,
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, min.y, max.z),
        ],
        spec.uvs.right.normalized(skin_width, skin_height),
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
    );
    push_face(
        &mut vertices,
        [
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        spec.uvs.left.normalized(skin_width, skin_height),
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
    );

    Mesh {
        vertex_count: (vertices.len() / 5) as i32,
        vertices,
        part: spec.part,
        overlay: spec.overlay,
        pivot: spec.pivot,
    }
}

fn push_face(vertices: &mut Vec<f32>, positions: [Vec3; 4], uv: NormalizedUv) {
    let uvs = [
        [uv.left, uv.bottom],
        [uv.left, uv.top],
        [uv.right, uv.top],
        [uv.right, uv.bottom],
    ];
    for index in [0_usize, 1, 2, 0, 2, 3] {
        let position = positions[index];
        vertices.extend_from_slice(&[
            position.x,
            position.y,
            position.z,
            uvs[index][0],
            uvs[index][1],
        ]);
    }
}

fn base_specs(format: SkinFormat) -> Vec<PartSpec> {
    let left_arm_uv = if matches!(format, SkinFormat::Modern64x64) {
        FaceUvs {
            top: UvRect::new(36.0, 48.0, 4.0, 4.0),
            bottom: UvRect::new(40.0, 48.0, 4.0, 4.0),
            right: UvRect::new(32.0, 52.0, 4.0, 12.0),
            front: UvRect::new(36.0, 52.0, 4.0, 12.0),
            left: UvRect::new(40.0, 52.0, 4.0, 12.0),
            back: UvRect::new(44.0, 52.0, 4.0, 12.0),
        }
    } else {
        arm_uv()
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
            center: Vec3::new(0.0, 24.0, 0.0),
            size: Vec3::new(8.0, 8.0, 8.0),
            pivot: Vec3::new(0.0, 20.0, 0.0),
            uvs: head_uv(),
            overlay: false,
            inflate: 1.0,
        },
        PartSpec {
            part: BodyPart::Body,
            center: Vec3::new(0.0, 14.0, 0.0),
            size: Vec3::new(8.0, 12.0, 4.0),
            pivot: Vec3::new(0.0, 14.0, 0.0),
            uvs: body_uv(),
            overlay: false,
            inflate: 1.0,
        },
        PartSpec {
            part: BodyPart::RightArm,
            center: Vec3::new(-6.0, 14.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(-6.0, 20.0, 0.0),
            uvs: arm_uv(),
            overlay: false,
            inflate: 1.0,
        },
        PartSpec {
            part: BodyPart::LeftArm,
            center: Vec3::new(6.0, 14.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(6.0, 20.0, 0.0),
            uvs: left_arm_uv,
            overlay: false,
            inflate: 1.0,
        },
        PartSpec {
            part: BodyPart::RightLeg,
            center: Vec3::new(-2.0, 6.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(-2.0, 12.0, 0.0),
            uvs: leg_uv(),
            overlay: false,
            inflate: 1.0,
        },
        PartSpec {
            part: BodyPart::LeftLeg,
            center: Vec3::new(2.0, 6.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(2.0, 12.0, 0.0),
            uvs: left_leg_uv,
            overlay: false,
            inflate: 1.0,
        },
    ]
}

fn overlay_specs() -> Vec<PartSpec> {
    vec![
        hat_spec(),
        PartSpec {
            part: BodyPart::Body,
            center: Vec3::new(0.0, 14.0, 0.0),
            size: Vec3::new(8.0, 12.0, 4.0),
            pivot: Vec3::new(0.0, 14.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(20.0, 32.0, 8.0, 4.0),
                bottom: UvRect::new(28.0, 32.0, 8.0, 4.0),
                right: UvRect::new(16.0, 36.0, 4.0, 12.0),
                front: UvRect::new(20.0, 36.0, 8.0, 12.0),
                left: UvRect::new(28.0, 36.0, 4.0, 12.0),
                back: UvRect::new(32.0, 36.0, 8.0, 12.0),
            },
            overlay: true,
            inflate: 1.03125,
        },
        PartSpec {
            part: BodyPart::RightArm,
            center: Vec3::new(-6.0, 14.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(-6.0, 20.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(44.0, 32.0, 4.0, 4.0),
                bottom: UvRect::new(48.0, 32.0, 4.0, 4.0),
                right: UvRect::new(40.0, 36.0, 4.0, 12.0),
                front: UvRect::new(44.0, 36.0, 4.0, 12.0),
                left: UvRect::new(48.0, 36.0, 4.0, 12.0),
                back: UvRect::new(52.0, 36.0, 4.0, 12.0),
            },
            overlay: true,
            inflate: 1.03125,
        },
        PartSpec {
            part: BodyPart::LeftArm,
            center: Vec3::new(6.0, 14.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(6.0, 20.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(52.0, 48.0, 4.0, 4.0),
                bottom: UvRect::new(56.0, 48.0, 4.0, 4.0),
                right: UvRect::new(48.0, 52.0, 4.0, 12.0),
                front: UvRect::new(52.0, 52.0, 4.0, 12.0),
                left: UvRect::new(56.0, 52.0, 4.0, 12.0),
                back: UvRect::new(60.0, 52.0, 4.0, 12.0),
            },
            overlay: true,
            inflate: 1.03125,
        },
        PartSpec {
            part: BodyPart::RightLeg,
            center: Vec3::new(-2.0, 6.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(-2.0, 12.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(4.0, 32.0, 4.0, 4.0),
                bottom: UvRect::new(8.0, 32.0, 4.0, 4.0),
                right: UvRect::new(0.0, 36.0, 4.0, 12.0),
                front: UvRect::new(4.0, 36.0, 4.0, 12.0),
                left: UvRect::new(8.0, 36.0, 4.0, 12.0),
                back: UvRect::new(12.0, 36.0, 4.0, 12.0),
            },
            overlay: true,
            inflate: 1.03125,
        },
        PartSpec {
            part: BodyPart::LeftLeg,
            center: Vec3::new(2.0, 6.0, 0.0),
            size: Vec3::new(4.0, 12.0, 4.0),
            pivot: Vec3::new(2.0, 12.0, 0.0),
            uvs: FaceUvs {
                top: UvRect::new(4.0, 48.0, 4.0, 4.0),
                bottom: UvRect::new(8.0, 48.0, 4.0, 4.0),
                right: UvRect::new(0.0, 52.0, 4.0, 12.0),
                front: UvRect::new(4.0, 52.0, 4.0, 12.0),
                left: UvRect::new(8.0, 52.0, 4.0, 12.0),
                back: UvRect::new(12.0, 52.0, 4.0, 12.0),
            },
            overlay: true,
            inflate: 1.03125,
        },
    ]
}

fn hat_spec() -> PartSpec {
    PartSpec {
        part: BodyPart::Head,
        center: Vec3::new(0.0, 24.0, 0.0),
        size: Vec3::new(8.0, 8.0, 8.0),
        pivot: Vec3::new(0.0, 20.0, 0.0),
        uvs: FaceUvs {
            top: UvRect::new(40.0, 0.0, 8.0, 8.0),
            bottom: UvRect::new(48.0, 0.0, 8.0, 8.0),
            right: UvRect::new(32.0, 8.0, 8.0, 8.0),
            front: UvRect::new(40.0, 8.0, 8.0, 8.0),
            left: UvRect::new(48.0, 8.0, 8.0, 8.0),
            back: UvRect::new(56.0, 8.0, 8.0, 8.0),
        },
        overlay: true,
        inflate: 1.0625,
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

fn arm_uv() -> FaceUvs {
    FaceUvs {
        top: UvRect::new(44.0, 16.0, 4.0, 4.0),
        bottom: UvRect::new(48.0, 16.0, 4.0, 4.0),
        right: UvRect::new(40.0, 20.0, 4.0, 12.0),
        front: UvRect::new(44.0, 20.0, 4.0, 12.0),
        left: UvRect::new(48.0, 20.0, 4.0, 12.0),
        back: UvRect::new(52.0, 20.0, 4.0, 12.0),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_pixel_uv_to_normalized_webgl_uv() {
        let uv = UvRect::new(8.0, 8.0, 8.0, 8.0).normalized(64.0, 64.0);
        assert!((uv.left - 0.125).abs() < 0.0001);
        assert!((uv.right - 0.25).abs() < 0.0001);
        assert!((uv.top - 0.875).abs() < 0.0001);
        assert!((uv.bottom - 0.75).abs() < 0.0001);
    }

    #[test]
    fn cuboid_generation_has_expected_vertex_count() {
        let meshes = build_player_meshes(SkinFormat::Modern64x64);
        assert_eq!(meshes[0].vertex_count, 36);
        assert_eq!(meshes[0].vertices.len(), 36 * 5);
    }
}
