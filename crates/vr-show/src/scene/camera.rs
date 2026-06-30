use glam::Quat;
use std::f32::consts::PI;

pub const FOV_DEFAULT: f32 = 75.0;
pub const FOV_MIN: f32 = 30.0;
pub const FOV_MAX: f32 = 100.0;
pub const FOV_STEP: f32 = 3.0;
pub const FOV_LERP: f32 = 0.1;
pub const FOV_LERP_THRESHOLD: f32 = 0.01;
pub const PITCH_LIMIT: f32 = PI / 2.0 - 0.01;
pub const DRAG_SENSITIVITY: f32 = 0.0035;
pub const AUTO_ROTATE_SPEED: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub current_fov: f32,
    pub target_fov: f32,
    pub auto_rotating: bool,
    pub has_fired_first_interaction: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            current_fov: FOV_DEFAULT,
            target_fov: FOV_DEFAULT,
            auto_rotating: true,
            has_fired_first_interaction: false,
        }
    }
}

impl CameraState {
    pub fn apply_drag(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * DRAG_SENSITIVITY;
        self.pitch += dy * DRAG_SENSITIVITY;
        self.pitch = self.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Returns true if first interaction fired this call.
    pub fn apply_wheel(&mut self, delta_y: f32) -> bool {
        let direction = if delta_y > 0.0 { -1.0 } else { 1.0 };
        self.target_fov = (self.target_fov + direction * FOV_STEP).clamp(FOV_MIN, FOV_MAX);
        self.fire_first_interaction()
    }

    /// Returns true if first interaction fired this call.
    pub fn fire_first_interaction(&mut self) -> bool {
        if self.has_fired_first_interaction {
            return false;
        }
        self.has_fired_first_interaction = true;
        self.auto_rotating = false;
        true
    }

    /// Advance state by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if self.auto_rotating {
            self.yaw += AUTO_ROTATE_SPEED * dt;
        }
        if (self.target_fov - self.current_fov).abs() > FOV_LERP_THRESHOLD {
            self.current_fov += (self.target_fov - self.current_fov) * FOV_LERP;
        }
    }

    /// Camera position is fixed at origin. Returns the rotation as a quaternion
    /// (yaw around world Y, then pitch around local X).
    pub fn rotation(&self) -> Quat {
        let qy = Quat::from_rotation_y(self.yaw);
        let qx = Quat::from_rotation_x(self.pitch);
        qy * qx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn default_state() {
        let c = CameraState::default();
        assert_eq!(c.yaw, 0.0);
        assert_eq!(c.pitch, 0.0);
        assert!(approx_eq(c.current_fov, FOV_DEFAULT));
        assert!(approx_eq(c.target_fov, FOV_DEFAULT));
        assert!(c.auto_rotating);
        assert!(!c.has_fired_first_interaction);
    }

    #[test]
    fn drag_accumulates_yaw_and_pitch() {
        let mut c = CameraState::default();
        c.apply_drag(100.0, 50.0);
        assert!(approx_eq(c.yaw, 100.0 * DRAG_SENSITIVITY));
        assert!(approx_eq(c.pitch, 50.0 * DRAG_SENSITIVITY));
    }

    #[test]
    fn pitch_clamps_to_positive_limit() {
        let mut c = CameraState::default();
        c.apply_drag(0.0, 1_000_000.0);
        assert!(approx_eq(c.pitch, PITCH_LIMIT));
    }

    #[test]
    fn pitch_clamps_to_negative_limit() {
        let mut c = CameraState::default();
        c.apply_drag(0.0, -1_000_000.0);
        assert!(approx_eq(c.pitch, -PITCH_LIMIT));
    }

    #[test]
    fn wheel_scroll_up_zooms_in() {
        let mut c = CameraState::default();
        c.apply_wheel(1.0);
        assert!(c.target_fov < FOV_DEFAULT);
    }

    #[test]
    fn wheel_scroll_down_zooms_out() {
        let mut c = CameraState::default();
        c.apply_wheel(-1.0);
        assert!(c.target_fov > FOV_DEFAULT);
    }

    #[test]
    fn wheel_clamps_at_min() {
        let mut c = CameraState::default();
        for _ in 0..100 {
            c.apply_wheel(1.0);
        }
        assert!(approx_eq(c.target_fov, FOV_MIN));
    }

    #[test]
    fn wheel_clamps_at_max() {
        let mut c = CameraState::default();
        for _ in 0..100 {
            c.apply_wheel(-1.0);
        }
        assert!(approx_eq(c.target_fov, FOV_MAX));
    }

    #[test]
    fn first_interaction_fires_once() {
        let mut c = CameraState::default();
        assert!(c.apply_wheel(1.0));
        assert!(!c.apply_wheel(1.0));
        assert!(!c.apply_wheel(-1.0));
        assert!(c.has_fired_first_interaction);
        assert!(!c.auto_rotating);
    }

    #[test]
    fn fov_lerp_moves_toward_target() {
        let mut c = CameraState::default();
        c.target_fov = FOV_MAX;
        let before = c.current_fov;
        c.update(0.016);
        assert!(c.current_fov > before);
        assert!(c.current_fov < FOV_MAX);
    }

    #[test]
    fn fov_lerp_settles_within_threshold() {
        let mut c = CameraState::default();
        c.target_fov = FOV_MAX;
        for _ in 0..1000 {
            c.update(0.016);
        }
        assert!((c.current_fov - c.target_fov).abs() < FOV_LERP_THRESHOLD);
    }

    #[test]
    fn auto_rotate_advances_yaw_over_time() {
        let mut c = CameraState::default();
        let before = c.yaw;
        c.update(1.0);
        assert!(approx_eq(c.yaw - before, AUTO_ROTATE_SPEED));
    }

    #[test]
    fn auto_rotate_stops_after_first_interaction() {
        let mut c = CameraState::default();
        c.fire_first_interaction();
        let before = c.yaw;
        c.update(1.0);
        assert!(approx_eq(c.yaw, before));
    }

    #[test]
    fn rotation_is_finite() {
        let c = CameraState::default();
        let q = c.rotation();
        assert!(q.x.is_finite() && q.y.is_finite() && q.z.is_finite() && q.w.is_finite());
    }
}
