#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn subtract(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn normalize(self) -> Self {
        let length = (self.dot(self)).sqrt();
        if length <= f32::EPSILON {
            return self;
        }
        Self::new(self.x / length, self.y / length, self.z / length)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub m: [f32; 16],
}

impl Mat4 {
    pub const fn identity() -> Self {
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        let mut matrix = Self::identity();
        matrix.m[12] = x;
        matrix.m[13] = y;
        matrix.m[14] = z;
        matrix
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_x(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0, 0.0, cos, sin, 0.0, 0.0, -sin, cos, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_y(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            m: [
                cos, 0.0, -sin, 0.0, 0.0, 1.0, 0.0, 0.0, sin, 0.0, cos, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn perspective(fovy_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fovy_radians * 0.5).tan();
        let nf = 1.0 / (near - far);
        Self {
            m: [
                f / aspect,
                0.0,
                0.0,
                0.0,
                0.0,
                f,
                0.0,
                0.0,
                0.0,
                0.0,
                (far + near) * nf,
                -1.0,
                0.0,
                0.0,
                (2.0 * far * near) * nf,
                0.0,
            ],
        }
    }

    pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let f = center.subtract(eye).normalize();
        let s = f.cross(up.normalize()).normalize();
        let u = s.cross(f);

        Self {
            m: [
                s.x,
                u.x,
                -f.x,
                0.0,
                s.y,
                u.y,
                -f.y,
                0.0,
                s.z,
                u.z,
                -f.z,
                0.0,
                -s.dot(eye),
                -u.dot(eye),
                f.dot(eye),
                1.0,
            ],
        }
    }

    pub fn multiply(self, rhs: Self) -> Self {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = self.m[row] * rhs.m[col * 4]
                    + self.m[4 + row] * rhs.m[col * 4 + 1]
                    + self.m[8 + row] * rhs.m[col * 4 + 2]
                    + self.m[12 + row] * rhs.m[col * 4 + 3];
            }
        }
        Self { m: out }
    }

    pub fn transform_point(self, point: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0] * point.x + self.m[4] * point.y + self.m[8] * point.z + self.m[12],
            self.m[1] * point.x + self.m[5] * point.y + self.m[9] * point.z + self.m[13],
            self.m[2] * point.x + self.m[6] * point.y + self.m[10] * point.z + self.m[14],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_multiplication() {
        let transform = Mat4::translation(1.0, 2.0, 3.0).multiply(Mat4::rotation_y(0.5));
        assert_eq!(Mat4::identity().multiply(transform), transform);
        assert_eq!(transform.multiply(Mat4::identity()), transform);
    }

    #[test]
    fn translation_uses_webgl_column_major_slots() {
        let matrix = Mat4::translation(1.0, 2.0, 3.0);

        assert_eq!(matrix.m[12], 1.0);
        assert_eq!(matrix.m[13], 2.0);
        assert_eq!(matrix.m[14], 3.0);
    }

    #[test]
    fn multiply_matches_column_vector_transform_order() {
        let matrix = Mat4::translation(2.0, 3.0, 4.0).multiply(Mat4::scale(5.0, 6.0, 7.0));
        let transformed = matrix.transform_point(Vec3::new(1.0, 1.0, 1.0));

        assert_eq!(transformed, Vec3::new(7.0, 9.0, 11.0));
    }

    #[test]
    fn perspective_has_no_nan_for_valid_inputs() {
        let matrix = Mat4::perspective(45.0_f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
        assert!(matrix.m.iter().all(|value| !value.is_nan()));
    }

    #[test]
    fn perspective_uses_supplied_aspect_for_horizontal_scale() {
        let square = Mat4::perspective(45.0_f32.to_radians(), 1.0, 0.1, 100.0);
        let wide = Mat4::perspective(45.0_f32.to_radians(), 2.0, 0.1, 100.0);

        assert!((wide.m[0] - square.m[0] * 0.5).abs() < 0.0001);
        assert!((wide.m[5] - square.m[5]).abs() < 0.0001);
    }
}
