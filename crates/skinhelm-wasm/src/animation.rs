#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkPose {
    pub right_leg_x: f32,
    pub left_leg_x: f32,
    pub right_arm_x: f32,
    pub left_arm_x: f32,
    pub cape_x: f32,
    pub body_y: f32,
    pub head_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapeMotion {
    Stopped,
    Idle,
    Walking,
    Running,
}

const LEG_SWING_RADIANS: f32 = 0.42;
const ARM_SWING_RADIANS: f32 = 0.55;
const BODY_BOB_PIXELS: f32 = 0.18;
const CAPE_REST_RADIANS: f32 = std::f32::consts::PI * 0.06;
const CAPE_REACTIVE_MIN_WALK_RADIANS: f32 = std::f32::consts::PI * 0.09;

pub fn static_pose() -> WalkPose {
    WalkPose {
        right_leg_x: 0.0,
        left_leg_x: 0.0,
        right_arm_x: 0.0,
        left_arm_x: 0.0,
        cape_x: cape_rotation_x(CapeMotion::Stopped, 0.0),
        body_y: 0.0,
        head_y: 0.0,
    }
}

pub fn walk_pose(phase: f32) -> WalkPose {
    let leg_swing = phase.sin() * LEG_SWING_RADIANS;
    let arm_swing = phase.sin() * ARM_SWING_RADIANS;
    let bob = phase.cos().abs() * BODY_BOB_PIXELS;
    WalkPose {
        right_leg_x: leg_swing,
        left_leg_x: -leg_swing,
        right_arm_x: -arm_swing,
        left_arm_x: arm_swing,
        cape_x: cape_rotation_x(CapeMotion::Walking, phase),
        body_y: bob,
        head_y: bob * 0.35,
    }
}

pub fn cape_reactive_walk_pose(phase: f32) -> WalkPose {
    let mut pose = walk_pose(phase);
    pose.cape_x = cape_reactive_walk_rotation_x(pose);
    pose
}

pub fn cape_rotation_x(motion: CapeMotion, t: f32) -> f32 {
    match motion {
        CapeMotion::Stopped => CAPE_REST_RADIANS,
        CapeMotion::Idle => t.sin() * 0.01 + CAPE_REST_RADIANS,
        CapeMotion::Walking => (t / 1.5).sin() * 0.06 + CAPE_REST_RADIANS,
        CapeMotion::Running => (t * 2.0).sin() * 0.1 + std::f32::consts::PI * 0.3,
    }
}

fn cape_reactive_walk_rotation_x(pose: WalkPose) -> f32 {
    let arm_contact = pose.right_arm_x.max(pose.left_arm_x).max(0.0);
    let leg_contact = pose.right_leg_x.max(pose.left_leg_x).max(0.0);
    let contact_push = arm_contact * 0.42 + leg_contact * 0.16;
    (pose.cape_x + contact_push).max(CAPE_REACTIVE_MIN_WALK_RADIANS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pose_at_phase_zero() {
        let pose = walk_pose(0.0);
        assert_eq!(pose.right_leg_x, 0.0);
        assert_eq!(pose.left_leg_x, -0.0);
        assert_eq!(pose.right_arm_x, -0.0);
        assert_eq!(pose.left_arm_x, 0.0);
        assert!((pose.cape_x - CAPE_REST_RADIANS).abs() < 0.0001);
    }

    #[test]
    fn pose_at_phase_pi_over_two() {
        let pose = walk_pose(std::f32::consts::FRAC_PI_2);
        assert!((pose.right_leg_x - LEG_SWING_RADIANS).abs() < 0.0001);
        assert!((pose.left_leg_x + LEG_SWING_RADIANS).abs() < 0.0001);
        assert!((pose.right_arm_x + ARM_SWING_RADIANS).abs() < 0.0001);
        assert!((pose.left_arm_x - ARM_SWING_RADIANS).abs() < 0.0001);
    }

    #[test]
    fn idle_cape_rotation_is_subtle() {
        let rotation = cape_rotation_x(CapeMotion::Idle, std::f32::consts::FRAC_PI_2);

        assert!((rotation - (CAPE_REST_RADIANS + 0.01)).abs() < 0.0001);
    }

    #[test]
    fn walking_cape_rotation_uses_slow_swing() {
        let rotation = cape_rotation_x(CapeMotion::Walking, 1.5 * std::f32::consts::FRAC_PI_2);

        assert!((rotation - (CAPE_REST_RADIANS + 0.06)).abs() < 0.0001);
    }

    #[test]
    fn stopped_cape_pose_does_not_depend_on_time() {
        assert_eq!(
            cape_rotation_x(CapeMotion::Stopped, 0.0),
            cape_rotation_x(CapeMotion::Stopped, 128.0)
        );
    }

    #[test]
    fn cape_reactive_walk_keeps_limb_swing_unchanged() {
        let normal = walk_pose(std::f32::consts::FRAC_PI_2);
        let reactive = cape_reactive_walk_pose(std::f32::consts::FRAC_PI_2);

        assert_eq!(reactive.right_arm_x, normal.right_arm_x);
        assert_eq!(reactive.left_arm_x, normal.left_arm_x);
        assert_eq!(reactive.right_leg_x, normal.right_leg_x);
        assert_eq!(reactive.left_leg_x, normal.left_leg_x);
    }

    #[test]
    fn cape_reactive_walk_pushes_cape_back_from_backward_limb() {
        let normal = walk_pose(std::f32::consts::FRAC_PI_2);
        let reactive = cape_reactive_walk_pose(std::f32::consts::FRAC_PI_2);

        assert!(reactive.cape_x > normal.cape_x);
        assert!(reactive.cape_x >= CAPE_REACTIVE_MIN_WALK_RADIANS);
    }
}
