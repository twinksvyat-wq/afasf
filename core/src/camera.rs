//! Smooth fight camera with three presets plus free orbit offsets.
use crate::math::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CamMode {
    Side = 0,
    Iso = 1,
    Third = 2,
}

pub struct Camera {
    pub mode: CamMode,
    pub eye: V3,
    pub target: V3,
    pub fov: f32,
    pub aspect: f32,
    /// user orbit offsets (radians) and zoom multiplier
    pub orbit_yaw: f32,
    pub orbit_pitch: f32,
    pub zoom: f32,
    pub screen_shift: f32,
    pub initialized: bool,
    pub time: f32,
}

pub struct CamGoal {
    pub target: V3,
    pub eye: V3,
    pub stiffness: f32,
}

impl Camera {
    pub fn new() -> Camera {
        Camera { mode: CamMode::Side, eye: v3(0.0, 1.6, 6.0), target: v3(0.0, 1.0, 0.0), fov: 0.85, aspect: 16.0 / 9.0, orbit_yaw: 0.0, orbit_pitch: 0.0, zoom: 1.0, screen_shift: 0.0, initialized: false, time: 0.0 }
    }

    pub fn cycle(&mut self) {
        self.mode = match self.mode {
            CamMode::Side => CamMode::Iso,
            CamMode::Iso => CamMode::Third,
            CamMode::Third => CamMode::Side,
        };
        self.orbit_yaw = 0.0;
        self.orbit_pitch = 0.0;
        self.zoom = 1.0;
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.orbit_yaw += dx;
        self.orbit_pitch = clampf(self.orbit_pitch + dy, -0.35, 1.1);
    }

    /// Goal for a fight: player position, opponent position, player facing.
    pub fn fight_goal(&self, a: V3, b: V3, a_face: V3) -> CamGoal {
        let mid = (a + b) * 0.5;
        let sep = (a - b).flat().len();
        let target = v3(mid.x, 0.95, mid.z);
        // base direction (from target toward eye) on the ground plane.
        // Pick the perpendicular on the side the camera already is, so it never flips.
        let line = (b - a).flat().norm_or(v3(1.0, 0.0, 0.0));
        let perp = line.cross(UP);
        let cur = if self.initialized { (self.eye - self.target).flat().rot_y(-self.orbit_yaw).norm_or(v3(0.0, 0.0, 1.0)) } else { v3(0.0, 0.0, 1.0) };
        let side_dir = if perp.dot(cur) >= 0.0 { perp } else { -perp };
        let (dir, dist, height) = match self.mode {
            CamMode::Side => (side_dir, clampf(sep * 0.95 + 2.2, 3.1, 9.0), 0.4),
            CamMode::Iso => ((side_dir + line * 0.9).norm(), clampf(sep * 1.0 + 3.0, 4.0, 11.0), 2.6),
            CamMode::Third => ((-a_face.flat().norm_or(line) + side_dir * 0.35).norm(), clampf(sep * 0.6 + 2.4, 2.8, 7.0), 0.85),
        };
        let dir = dir.rot_y(self.orbit_yaw);
        let d = dist * self.zoom;
        let pitch_h = height + self.orbit_pitch * d;
        let eye = target + dir * d + UP * pitch_h;
        let target = if self.mode == CamMode::Third { target.lerp(b, 0.25) } else { target };
        CamGoal { target, eye, stiffness: 5.0 }
    }

    pub fn menu_goal(&self, at: V3) -> CamGoal {
        let target = at + v3(0.0, 1.05, 0.0);
        let dir = v3(0.0, 0.0, 1.0).rot_y(self.orbit_yaw);
        let d = 3.4 * self.zoom;
        let eye = target + dir * d + UP * (0.35 + self.orbit_pitch * d);
        CamGoal { target, eye, stiffness: 8.0 }
    }

    pub fn follow(&mut self, g: &CamGoal, dt: f32) {
        if !self.initialized {
            self.eye = g.eye;
            self.target = g.target;
            self.initialized = true;
            return;
        }
        let k = 1.0 - (-g.stiffness * dt).exp();
        self.eye = self.eye.lerp(g.eye, k);
        self.target = self.target.lerp(g.target, k);
        if self.eye.y < 0.25 {
            self.eye.y = 0.25;
        }
    }

    pub fn snap(&mut self) {
        self.initialized = false;
    }

    /// View-projection with shake applied and optional horizontal screen shift.
    pub fn matrix(&self, shake: f32, rng: &mut Rng) -> (M4, V3) {
        let s = shake * shake * 0.12;
        let off = v3(rng.range(-s, s), rng.range(-s, s), rng.range(-s, s));
        let eye = self.eye + off;
        let tgt = self.target + off * 0.5;
        // screen shift: move both eye and target sideways in camera space
        let fwd = (tgt - eye).norm();
        let right = fwd.cross(UP).norm();
        let shift = right * (-self.screen_shift * (tgt - eye).len() * 0.45);
        let view = M4::look_at(eye + shift, tgt + shift, UP);
        let proj = M4::perspective(self.fov, self.aspect, 0.1, 200.0);
        (proj.mul(&view), eye)
    }

    pub fn basis(&self) -> (V3, V3) {
        let fwd = (self.target - self.eye).flat().norm_or(v3(0.0, 0.0, -1.0));
        let right = fwd.cross(UP).norm();
        (right, fwd)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}
