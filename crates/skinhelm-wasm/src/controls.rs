use crate::math::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct OrbitControls {
    yaw: f32,
    pitch: f32,
    distance: f32,
    dragging: bool,
    last_x: f32,
    last_y: f32,
}

impl OrbitControls {
    pub fn new() -> Self {
        Self {
            yaw: 0.35,
            pitch: 0.25,
            distance: 58.0,
            dragging: false,
            last_x: 0.0,
            last_y: 0.0,
        }
    }

    pub fn pointer_down(&mut self, x: f32, y: f32) {
        self.dragging = true;
        self.last_x = x;
        self.last_y = y;
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) {
        if !self.dragging {
            return;
        }
        let dx = x - self.last_x;
        let dy = y - self.last_y;
        self.last_x = x;
        self.last_y = y;
        self.yaw += dx * 0.01;
        self.pitch = (self.pitch + dy * 0.01).clamp(-1.2, 1.2);
    }

    pub fn pointer_up(&mut self) {
        self.dragging = false;
    }

    pub fn zoom(&mut self, delta_y: f32) {
        self.distance = (self.distance + delta_y * 0.04).clamp(28.0, 95.0);
    }

    pub fn view_matrix(&self) -> Mat4 {
        let target = Vec3::new(0.0, 15.0, 0.0);
        let cos_pitch = self.pitch.cos();
        let eye = Vec3::new(
            self.distance * self.yaw.sin() * cos_pitch,
            target.y + self.distance * self.pitch.sin(),
            self.distance * self.yaw.cos() * cos_pitch,
        );
        Mat4::look_at(eye, target, Vec3::new(0.0, 1.0, 0.0))
    }
}

impl Default for OrbitControls {
    fn default() -> Self {
        Self::new()
    }
}
