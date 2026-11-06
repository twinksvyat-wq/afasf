//! Minimal 3D math: vectors, matrices, RNG. No external dependencies.
use std::ops::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[inline]
pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
pub const UP: V3 = v3(0.0, 1.0, 0.0);

impl V3 {
    #[inline]
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    #[inline]
    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    #[inline]
    pub fn len2(self) -> f32 {
        self.dot(self)
    }
    #[inline]
    pub fn len(self) -> f32 {
        self.len2().sqrt()
    }
    #[inline]
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            ZERO
        }
    }
    #[inline]
    pub fn norm_or(self, d: V3) -> V3 {
        let l = self.len();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            d
        }
    }
    #[inline]
    pub fn flat(self) -> V3 {
        v3(self.x, 0.0, self.z)
    }
    #[inline]
    pub fn lerp(self, o: V3, t: f32) -> V3 {
        self + (o - self) * t
    }
    #[inline]
    pub fn mul(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
    pub fn clamp_len(self, m: f32) -> V3 {
        let l = self.len();
        if l > m {
            self * (m / l)
        } else {
            self
        }
    }
    /// Rotate around an arbitrary (unit) axis (Rodrigues).
    pub fn rot_axis(self, axis: V3, ang: f32) -> V3 {
        let (s, c) = ang.sin_cos();
        self * c + axis.cross(self) * s + axis * (axis.dot(self) * (1.0 - c))
    }
    pub fn rot_y(self, ang: f32) -> V3 {
        let (s, c) = ang.sin_cos();
        v3(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
    }
    pub fn arr(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Add for V3 {
    type Output = V3;
    #[inline]
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    #[inline]
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    #[inline]
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for V3 {
    type Output = V3;
    #[inline]
    fn div(self, s: f32) -> V3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for V3 {
    type Output = V3;
    #[inline]
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for V3 {
    #[inline]
    fn add_assign(&mut self, o: V3) {
        *self = *self + o
    }
}
impl SubAssign for V3 {
    #[inline]
    fn sub_assign(&mut self, o: V3) {
        *self = *self - o
    }
}
impl MulAssign<f32> for V3 {
    #[inline]
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s
    }
}

/// Facing direction (on the ground plane) for a yaw angle. yaw=0 -> +X.
#[inline]
pub fn dir_from_yaw(yaw: f32) -> V3 {
    v3(yaw.cos(), 0.0, -yaw.sin())
}
#[inline]
pub fn yaw_from_dir(d: V3) -> f32 {
    (-d.z).atan2(d.x)
}

#[inline]
pub fn clampf(x: f32, a: f32, b: f32) -> f32 {
    x.max(a).min(b)
}
#[inline]
pub fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
#[inline]
pub fn smooth(t: f32) -> f32 {
    let t = clampf(t, 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
#[inline]
pub fn approach(x: f32, target: f32, rate: f32) -> f32 {
    if x < target {
        (x + rate).min(target)
    } else {
        (x - rate).max(target)
    }
}
pub fn wrap_angle(mut a: f32) -> f32 {
    use std::f32::consts::PI;
    while a > PI {
        a -= 2.0 * PI
    }
    while a < -PI {
        a += 2.0 * PI
    }
    a
}

/// Orthonormal frame: forward, up, right (right = forward x up).
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub o: V3,
    pub f: V3,
    pub u: V3,
    pub r: V3,
}
impl Frame {
    pub fn from_yaw(o: V3, yaw: f32) -> Frame {
        let f = dir_from_yaw(yaw);
        Frame { o, f, u: UP, r: f.cross(UP) }
    }
    /// local = (forward, up, right)
    #[inline]
    pub fn p(&self, l: V3) -> V3 {
        self.o + self.f * l.x + self.u * l.y + self.r * l.z
    }
    #[inline]
    pub fn d(&self, l: V3) -> V3 {
        self.f * l.x + self.u * l.y + self.r * l.z
    }
    pub fn rotated(&self, axis: V3, ang: f32) -> Frame {
        Frame { o: self.o, f: self.f.rot_axis(axis, ang), u: self.u.rot_axis(axis, ang), r: self.r.rot_axis(axis, ang) }
    }
    /// Build frame from up and a forward hint (orthogonalised).
    pub fn from_up_fwd(o: V3, up: V3, fwd_hint: V3) -> Frame {
        let u = up.norm_or(UP);
        let mut f = fwd_hint - u * fwd_hint.dot(u);
        f = f.norm_or(if u.y.abs() < 0.9 { UP.cross(u).cross(u).norm() } else { v3(1.0, 0.0, 0.0) });
        Frame { o, f, u, r: f.cross(u) }
    }
}

/// Column-major 4x4 (Metal / simd layout).
#[derive(Clone, Copy, Debug)]
pub struct M4(pub [f32; 16]);
impl M4 {
    pub fn mul(&self, o: &M4) -> M4 {
        let a = &self.0;
        let b = &o.0;
        let mut r = [0.0f32; 16];
        for c in 0..4 {
            for row in 0..4 {
                let mut s = 0.0;
                for k in 0..4 {
                    s += a[k * 4 + row] * b[c * 4 + k];
                }
                r[c * 4 + row] = s;
            }
        }
        M4(r)
    }
    /// Right-handed perspective with clip z in [0,1] (Metal).
    pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> M4 {
        let ys = 1.0 / (fovy * 0.5).tan();
        let xs = ys / aspect;
        let zs = far / (near - far);
        M4([xs, 0.0, 0.0, 0.0, 0.0, ys, 0.0, 0.0, 0.0, 0.0, zs, -1.0, 0.0, 0.0, zs * near, 0.0])
    }
    pub fn look_at(eye: V3, target: V3, up: V3) -> M4 {
        let z = (eye - target).norm();
        let x = up.cross(z).norm();
        let y = z.cross(x);
        M4([
            x.x, y.x, z.x, 0.0, x.y, y.y, z.y, 0.0, x.z, y.z, z.z, 0.0,
            -x.dot(eye), -y.dot(eye), -z.dot(eye), 1.0,
        ])
    }
    /// Transform point, returns clip-space (x,y,z,w)
    pub fn xform(&self, p: V3) -> [f32; 4] {
        let m = &self.0;
        [
            m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12],
            m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13],
            m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14],
            m[3] * p.x + m[7] * p.y + m[11] * p.z + m[15],
        ]
    }
}

/// xorshift64* RNG
#[derive(Clone, Debug)]
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }
    pub fn f(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16777216.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn unit(&mut self) -> V3 {
        loop {
            let v = v3(self.range(-1.0, 1.0), self.range(-1.0, 1.0), self.range(-1.0, 1.0));
            let l = v.len2();
            if l > 0.01 && l <= 1.0 {
                return v / l.sqrt();
            }
        }
    }
}

/// Closest points between segments p1-q1 and p2-q2. Returns (s, t) params.
pub fn seg_seg(p1: V3, q1: V3, p2: V3, q2: V3) -> (f32, f32) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.len2();
    let e = d2.len2();
    let f = d2.dot(r);
    let eps = 1e-8;
    if a <= eps && e <= eps {
        return (0.0, 0.0);
    }
    let (mut s, mut t);
    if a <= eps {
        s = 0.0;
        t = clampf(f / e, 0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= eps {
            t = 0.0;
            s = clampf(-c / a, 0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            s = if denom > eps { clampf((b * f - c * e) / denom, 0.0, 1.0) } else { 0.0 };
            t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = clampf(-c / a, 0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = clampf((b - c) / a, 0.0, 1.0);
            }
        }
    }
    (s, t)
}

/// Two-bone IK. Returns middle joint position.
pub fn ik2(root: V3, target: V3, a: f32, b: f32, pole: V3) -> (V3, V3) {
    let to = target - root;
    let mut d = to.len();
    let dir = to.norm_or(v3(0.0, -1.0, 0.0));
    let maxd = (a + b) * 0.999;
    let mind = (a - b).abs() + 0.01;
    d = clampf(d, mind, maxd);
    let end = root + dir * d;
    let cos_a = clampf((a * a + d * d - b * b) / (2.0 * a * d), -1.0, 1.0);
    let sin_a = (1.0 - cos_a * cos_a).sqrt();
    let mut perp = pole - dir * pole.dot(dir);
    perp = perp.norm_or(dir.cross(v3(0.0, 0.0, 1.0)).norm_or(v3(1.0, 0.0, 0.0)));
    (root + dir * (a * cos_a) + perp * (a * sin_a), end)
}
