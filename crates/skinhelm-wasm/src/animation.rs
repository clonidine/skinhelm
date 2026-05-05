#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkPose {
    pub right_leg_x: f32,
    pub left_leg_x: f32,
    pub right_arm_x: f32,
    pub left_arm_x: f32,
    pub body_y: f32,
    pub head_y: f32,
}

pub fn walk_pose(phase: f32) -> WalkPose {
    let swing = phase.sin() * 0.7;
    let bob = phase.cos().abs() * 0.35;
    WalkPose {
        right_leg_x: swing,
        left_leg_x: -swing,
        right_arm_x: -swing,
        left_arm_x: swing,
        body_y: bob,
        head_y: bob * 0.35,
    }
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
    }

    #[test]
    fn pose_at_phase_pi_over_two() {
        let pose = walk_pose(std::f32::consts::FRAC_PI_2);
        assert!((pose.right_leg_x - 0.7).abs() < 0.0001);
        assert!((pose.left_leg_x + 0.7).abs() < 0.0001);
        assert!((pose.right_arm_x + 0.7).abs() < 0.0001);
        assert!((pose.left_arm_x - 0.7).abs() < 0.0001);
    }
}
