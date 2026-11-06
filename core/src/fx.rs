//! Visual effects: blood/sweat droplets, impact shards, dust, floor decals.
use crate::math::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PKind {
    Blood,
    Sweat,
    Spark,
    Dust,
    Ring,
    Confetti,
}

#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub p: V3,
    pub v: V3,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub col: [f32; 3],
    pub kind: PKind,
    pub axis: V3,
}

#[derive(Clone, Copy, Debug)]
pub struct Decal {
    pub p: V3,
    pub r: f32,
    pub rot: f32,
    pub seed: u32,
    pub col: [f32; 3],
    pub grow: f32,
}

pub struct Fx {
    pub parts: Vec<Part>,
    pub decals: Vec<Decal>,
    decal_head: usize,
    pub rng: Rng,
    pub shake: f32,
    pub flash: f32,
}

pub const MAX_DECALS: usize = 260;
const MAX_PARTS: usize = 900;

impl Fx {
    pub fn new(seed: u64) -> Fx {
        Fx { parts: Vec::with_capacity(MAX_PARTS), decals: Vec::with_capacity(MAX_DECALS), decal_head: 0, rng: Rng::new(seed), shake: 0.0, flash: 0.0 }
    }
    pub fn clear(&mut self) {
        self.parts.clear();
        self.decals.clear();
        self.decal_head = 0;
        self.shake = 0.0;
        self.flash = 0.0;
    }
    fn push(parts: &mut Vec<Part>, p: Part) {
        if parts.len() < MAX_PARTS {
            parts.push(p);
        }
    }
    pub fn blood(&mut self, at: V3, dir: V3, n: usize, speed: f32) {
        for _ in 0..n {
            let v = (dir * self.rng.range(0.4, 1.0) + self.rng.unit() * 0.55 + UP * 0.25) * speed * self.rng.range(0.5, 1.2);
            let s = self.rng.range(0.012, 0.03);
            let dark = self.rng.range(0.0, 0.15);
            Self::push(&mut self.parts, Part { p: at, v, life: 0.0, max: 3.0, size: s, col: [0.62 - dark, 0.02, 0.04], kind: PKind::Blood, axis: self.rng.unit() });
        }
    }
    pub fn sweat(&mut self, at: V3, dir: V3, n: usize) {
        for _ in 0..n {
            let v = (dir * 0.8 + self.rng.unit() * 0.7 + UP * 0.3) * self.rng.range(1.5, 3.5);
            Self::push(&mut self.parts, Part { p: at, v, life: 0.0, max: 0.9, size: self.rng.range(0.008, 0.016), col: [0.72, 0.88, 1.0], kind: PKind::Sweat, axis: self.rng.unit() });
        }
    }
    pub fn impact(&mut self, at: V3, dir: V3, strength: f32) {
        let n = (6.0 + strength * 0.8) as usize;
        for _ in 0..n.min(22) {
            let d = (self.rng.unit() + dir * 0.6).norm();
            let col = if self.rng.chance(0.5) { [1.0, 0.85, 0.15] } else { [1.0, 0.45, 0.05] };
            Self::push(&mut self.parts, Part { p: at, v: d * self.rng.range(2.0, 5.0) * (0.6 + strength * 0.03), life: 0.0, max: self.rng.range(0.12, 0.22), size: self.rng.range(0.05, 0.11) * (0.7 + strength * 0.02), col, kind: PKind::Spark, axis: d });
        }
        Self::push(&mut self.parts, Part { p: at, v: ZERO, life: 0.0, max: 0.18, size: 0.12 + strength * 0.012, col: [1.0, 0.95, 0.6], kind: PKind::Ring, axis: dir });
    }
    pub fn dust(&mut self, at: V3, strength: f32) {
        let n = (4.0 + strength * 2.0) as usize;
        for _ in 0..n.min(16) {
            let mut d = self.rng.unit();
            d.y = d.y.abs() * 0.3;
            Self::push(&mut self.parts, Part { p: v3(at.x, 0.05, at.z), v: d * self.rng.range(0.6, 1.8), life: 0.0, max: self.rng.range(0.4, 0.8), size: self.rng.range(0.06, 0.14), col: [0.86, 0.86, 0.88], kind: PKind::Dust, axis: d });
        }
    }
    pub fn confetti(&mut self, at: V3, n: usize) {
        const C: [[f32; 3]; 5] = [[1.0, 0.2, 0.2], [1.0, 0.8, 0.1], [0.2, 0.6, 1.0], [0.3, 0.9, 0.3], [0.9, 0.3, 0.9]];
        for _ in 0..n {
            let mut d = self.rng.unit();
            d.y = d.y.abs() + 0.8;
            let col = C[self.rng.below(5) as usize];
            Self::push(&mut self.parts, Part { p: at, v: d * self.rng.range(2.0, 4.5), life: 0.0, max: 2.5, size: 0.035, col, kind: PKind::Confetti, axis: self.rng.unit() });
        }
    }
    pub fn splat(&mut self, p: V3, r: f32) {
        let d = Decal { p: v3(p.x, 0.0, p.z), r, rot: self.rng.range(0.0, 6.28), seed: self.rng.next_u32(), col: [0.5 + self.rng.range(0.0, 0.12), 0.01, 0.03], grow: 0.0 };
        if self.decals.len() < MAX_DECALS {
            self.decals.push(d);
        } else {
            self.decals[self.decal_head] = d;
            self.decal_head = (self.decal_head + 1) % MAX_DECALS;
        }
    }
    pub fn update(&mut self, dt: f32) {
        self.shake = (self.shake - dt * 2.2).max(0.0);
        self.flash = (self.flash - dt * 4.0).max(0.0);
        let mut splats: Vec<(V3, f32)> = Vec::new();
        for p in self.parts.iter_mut() {
            p.life += dt;
            match p.kind {
                PKind::Blood | PKind::Sweat => {
                    p.v.y -= 9.81 * dt;
                    p.p += p.v * dt;
                    if p.p.y <= 0.0 {
                        if p.kind == PKind::Blood {
                            splats.push((p.p, p.size * 2.6));
                        }
                        p.life = p.max;
                    }
                }
                PKind::Spark => {
                    p.p += p.v * dt;
                    p.v *= 0.85;
                }
                PKind::Dust => {
                    p.p += p.v * dt;
                    p.v *= 0.92;
                    p.v.y += 0.4 * dt;
                }
                PKind::Ring => {}
                PKind::Confetti => {
                    p.v.y -= 4.0 * dt;
                    p.v *= 0.97;
                    p.p += p.v * dt;
                    if p.p.y < 0.01 {
                        p.p.y = 0.01;
                        p.v = ZERO;
                    }
                }
            }
        }
        self.parts.retain(|p| p.life < p.max);
        for (p, r) in splats {
            self.splat(p, r);
        }
        for d in self.decals.iter_mut() {
            d.grow = (d.grow + dt * 6.0).min(1.0);
        }
    }
}
