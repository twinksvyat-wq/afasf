//! Top-level game: scenes (menu showcase / fight), input, camera, profile, mesh output.
use crate::camera::*;
use crate::catalog::*;
use crate::fight::*;
use crate::fighter::*;
use crate::fx::Fx;
use crate::math::*;
use crate::mesh::Mesh;
use crate::profile::*;
use crate::render;
use std::collections::VecDeque;

pub const BTN_PUNCH: u32 = 0;
pub const BTN_KICK: u32 = 1;
pub const BTN_GRAB: u32 = 2;
pub const BTN_BLOCK: u32 = 3;
pub const BTN_SPECIAL: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    Menu,
    Fight,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FightResult {
    pub valid: u32,
    pub won: u32,
    pub method: u32,
    pub coins: u32,
    pub landed: u32,
    pub thrown: u32,
    pub knockdowns: u32,
    pub damage: f32,
    pub finisher: u32,
    pub unlocked: u32,
    pub opponent: u32,
    pub rounds: u32,
}

pub struct Game {
    pub scene: Scene,
    pub profile: Profile,
    pub cam: Camera,
    pub mesh: Mesh,
    pub fight: Option<Fight>,
    pub menu_fighter: Fighter,
    pub menu_fx: Fx,
    pub events: VecDeque<Event>,
    pub stick: (f32, f32),
    pub held: u32,
    pub pressed: u32,
    acc: f32,
    pub time: f32,
    rng: Rng,
    pub view_proj: M4,
    pub eye: V3,
    pub width: f32,
    pub height: f32,
    pub paused: bool,
    pub preview_item: Option<u16>,
    pub preview_cel: Option<u32>,
    pub preview_opp: Option<usize>,
    pub preview_fighter: Option<u32>,
    pub result: FightResult,
    pub save_buf: std::ffi::CString,
    menu_key: (Look, u32, Option<usize>),
    pub shadowbox: bool,
    /// demo/test mode: an AI drives the player's fighter
    pub autoplay: Option<crate::ai::Ai>,
}

fn menu_yaw() -> f32 {
    -std::f32::consts::FRAC_PI_2 // facing +Z (toward the menu camera)
}

impl Game {
    pub fn new(seed: u64) -> Game {
        let profile = Profile::default();
        let arch = ARCHETYPES[0];
        let mut mf = Fighter::new(arch, profile.look, ZERO, menu_yaw(), seed);
        mf.menu_mode = true;
        let mut cam = Camera::new();
        cam.orbit_yaw = 0.35;
        Game {
            scene: Scene::Menu,
            menu_key: (profile.look, 0, None),
            profile,
            cam,
            mesh: Mesh::new(),
            fight: None,
            menu_fighter: mf,
            menu_fx: Fx::new(seed ^ 9),
            events: VecDeque::new(),
            stick: (0.0, 0.0),
            held: 0,
            pressed: 0,
            acc: 0.0,
            time: 0.0,
            rng: Rng::new(seed ^ 0x1234),
            view_proj: M4([0.0; 16]),
            eye: ZERO,
            width: 1920.0,
            height: 1080.0,
            paused: false,
            preview_item: None,
            preview_cel: None,
            preview_opp: None,
            preview_fighter: None,
            result: FightResult::default(),
            save_buf: std::ffi::CString::default(),
            shadowbox: true,
            autoplay: None,
        }
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        self.width = w.max(1.0);
        self.height = h.max(1.0);
        self.cam.aspect = self.width / self.height;
    }

    pub fn button(&mut self, b: u32, down: bool) {
        if b > 4 {
            return;
        }
        if down {
            self.held |= 1 << b;
            self.pressed |= 1 << b;
        } else {
            self.held &= !(1 << b);
        }
    }

    pub fn start_fight(&mut self, opp: usize) -> bool {
        if opp as u32 > self.profile.ladder || opp >= OPPONENTS.len() {
            return false;
        }
        let arch = ARCHETYPES[self.profile.fighter as usize];
        let seed = self.rng.next_u32() as u64 | ((self.rng.next_u32() as u64) << 32);
        self.fight = Some(Fight::new(arch, self.profile.look, opp, self.profile.celebration, seed));
        self.scene = Scene::Fight;
        self.cam.mode = CamMode::Side;
        self.cam.orbit_yaw = 0.0;
        self.cam.orbit_pitch = 0.0;
        self.cam.zoom = 1.0;
        self.cam.screen_shift = 0.0;
        self.cam.snap();
        self.result = FightResult::default();
        self.events.clear();
        self.held = 0;
        self.pressed = 0;
        self.paused = false;
        true
    }

    pub fn go_menu(&mut self) {
        self.scene = Scene::Menu;
        self.fight = None;
        self.paused = false;
        self.cam.orbit_yaw = 0.35;
        self.cam.orbit_pitch = 0.0;
        self.cam.zoom = 1.0;
        self.cam.snap();
        self.preview_cel = None;
        self.preview_item = None;
        self.preview_opp = None;
        self.preview_fighter = None;
        self.sync_menu_fighter(true);
    }

    fn menu_target(&self) -> (Look, u32, Option<usize>) {
        if let Some(o) = self.preview_opp {
            let op = &OPPONENTS[o];
            return (Look { items: op.look }, op.archetype, Some(o));
        }
        let mut look = self.profile.look;
        if let Some(id) = self.preview_item {
            look.set(id);
        }
        (look, self.preview_fighter.unwrap_or(self.profile.fighter), None)
    }

    pub fn sync_menu_fighter(&mut self, force: bool) {
        let key = self.menu_target();
        if key == self.menu_key && !force {
            return;
        }
        let rebuild = key.1 != self.menu_key.1 || force;
        self.menu_key = key;
        if rebuild {
            let mut mf = Fighter::new(ARCHETYPES[key.1 as usize], key.0, ZERO, menu_yaw(), self.rng.next_u32() as u64);
            mf.menu_mode = true;
            self.menu_fighter = mf;
        } else {
            self.menu_fighter.look = key.0;
        }
    }

    pub fn preview_celebration(&mut self, c: Option<u32>) {
        self.preview_cel = c;
        let f = &mut self.menu_fighter;
        match c {
            Some(k) => {
                f.state = FState::Celebrate { kind: k, t: 0.0 };
                f.action = Action::Idle;
            }
            None => {
                f.respawn(ZERO, menu_yaw());
            }
        }
    }

    /// Tap the fighter in the menu: physics shove.
    pub fn poke(&mut self, x: f32, y: f32) {
        if self.scene != Scene::Menu {
            return;
        }
        let fwd = (self.cam.target - self.cam.eye).norm();
        let right = fwd.cross(UP).norm();
        let up = right.cross(fwd);
        let th = (self.cam.fov * 0.5).tan();
        // account for screen shift: shift moves the image; approximate by offsetting x
        let x2 = x + self.cam.screen_shift * 0.45 / (th * self.cam.aspect) * 0.0;
        let dir = (fwd + right * (x2 * th * self.cam.aspect) + up * (y * th)).norm();
        let shift = right * (-self.cam.screen_shift * (self.cam.target - self.cam.eye).len() * 0.45);
        let origin = self.cam.eye + shift;
        let f = &mut self.menu_fighter;
        let mut best = (0.3f32, None);
        for i in 0..NP {
            let p = f.body[i].p - origin;
            let t = p.dot(dir);
            if t <= 0.0 {
                continue;
            }
            let d = (p - dir * t).len();
            if d < best.0 {
                best = (d, Some(i));
            }
        }
        if let Some(i) = best.1 {
            let dv = (dir.flat().norm() + UP * 0.1) * 5.0;
            f.add_impulse(i, dv, FRAME / SUBSTEPS as f32);
            if i == HEAD || i == NECK {
                f.add_impulse(NECK, dv * 0.5, FRAME / SUBSTEPS as f32);
            }
            if matches!(f.state, FState::Stand) {
                f.action = Action::Hurt { t: 0.35 };
                f.root_vel += dir.flat().norm() * 0.6;
            }
            let at = f.body[i].p;
            self.menu_fx.impact(at, -dir, 6.0);
            self.menu_fx.sweat(at, dir, 6);
            self.events.push_back(Event { kind: EV_HIT, fighter: 0, a: 5.0, b: 0.0 });
        }
    }

    pub fn update(&mut self, dt: f32) {
        let dt = clampf(dt, 0.0, 0.1);
        self.time += dt;
        if !self.paused {
            let scale = match &self.fight {
                Some(f) if self.scene == Scene::Fight => f.slowmo,
                _ => 1.0,
            };
            self.acc += dt * scale;
            let mut steps = 0;
            while self.acc >= FRAME && steps < 5 {
                self.acc -= FRAME;
                steps += 1;
                self.fixed_step();
            }
            if steps == 5 {
                self.acc = 0.0;
            }
        }
        // camera
        let goal = match (&self.scene, &self.fight) {
            (Scene::Fight, Some(f)) => {
                let a = f.f[0].pos(PELVIS);
                let b = f.f[1].pos(PELVIS);
                let mut g = self.cam.fight_goal(a, b, f.f[0].facing());
                match f.phase {
                    Phase::Ko { t } if t < 1.6 => {
                        let l = f.winner.map(|w| f.f[1 - w].pos(HEAD)).unwrap_or(b);
                        g.target = g.target.lerp(l, 0.6);
                        g.eye = g.target + (g.eye - g.target) * 0.8 + UP * 0.3;
                        g.stiffness = 3.0;
                    }
                    Phase::Celebrate { t } | Phase::Ko { t } => {
                        let w = f.winner.unwrap_or(0);
                        let c = f.f[w].pos(PELVIS);
                        let ang = t * 0.35 + 0.4;
                        let dir = v3(ang.sin(), 0.0, ang.cos());
                        g.target = v3(c.x, 1.0, c.z);
                        g.eye = g.target + dir * (3.6 * self.cam.zoom) + UP * (0.5 + self.cam.orbit_pitch * 3.0);
                        g.stiffness = 2.5;
                    }
                    _ => {}
                }
                g
            }
            _ => self.cam.menu_goal(self.menu_fighter.root),
        };
        self.cam.follow(&goal, dt);
        let shake = match &self.fight {
            Some(f) if self.scene == Scene::Fight => f.fx.shake,
            _ => self.menu_fx.shake,
        };
        let (vp, eye) = self.cam.matrix(shake, &mut self.rng);
        self.view_proj = vp;
        self.eye = eye;
        self.build_mesh();
    }

    fn player_intent(&mut self) -> Intent {
        let (sx, sy) = self.stick;
        let (right, fwd) = self.cam.basis();
        let mut mv = right * sx + fwd * sy;
        if mv.len() < 0.18 {
            mv = ZERO;
        }
        let mut it = Intent { mv: mv.clamp_len(1.0), ..Default::default() };
        if let Some(f) = &self.fight {
            let to = (f.f[1].root - f.f[0].root).flat().norm();
            let d = mv.norm().dot(to);
            it.fwd = mv.len() > 0.4 && d > 0.55;
            it.back = mv.len() > 0.4 && d < -0.55;
        }
        it.down = sy < -0.6 && sx.abs() < 0.6;
        let p = self.pressed;
        it.punch = p & (1 << BTN_PUNCH) != 0;
        it.kick = p & (1 << BTN_KICK) != 0;
        it.grab = p & (1 << BTN_GRAB) != 0;
        it.special = p & (1 << BTN_SPECIAL) != 0;
        it.block = self.held & (1 << BTN_BLOCK) != 0;
        self.pressed = 0;
        it
    }

    fn fixed_step(&mut self) {
        match self.scene {
            Scene::Fight => {
                let mut it = self.player_intent();
                let eye = self.eye;
                let Some(f) = self.fight.as_mut() else { return };
                if let Some(ai) = self.autoplay.as_mut() {
                    if matches!(f.phase, Phase::Fight) {
                        it = ai.intent(&f.f[0], &f.f[1], FRAME);
                    }
                }
                f.step(it, eye);
                for e in f.events.drain(..) {
                    if self.events.len() < 256 {
                        self.events.push_back(e);
                    }
                    if e.kind == EV_FIGHT_END && self.result.valid == 0 {
                        let p = &f.f[0];
                        let won = e.a > 0.5;
                        let method = f.method as u32;
                        let rw = self.profile.record_fight(f.opp_index, won, method, p.stats.landed, p.stats.punches, p.stats.kicks, p.stats.throws, p.stats.knockdowns, p.stats.damage, p.stats.finisher);
                        self.result = FightResult {
                            valid: 1,
                            won: won as u32,
                            method,
                            coins: rw.coins,
                            landed: p.stats.landed,
                            thrown: p.stats.thrown,
                            knockdowns: p.stats.knockdowns,
                            damage: p.stats.damage,
                            finisher: p.stats.finisher as u32,
                            unlocked: rw.unlocked_opponent as u32,
                            opponent: f.opp_index as u32,
                            rounds: f.round,
                        };
                    }
                }
            }
            Scene::Menu => {
                self.pressed = 0;
                self.sync_menu_fighter(false);
                let f = &mut self.menu_fighter;
                let mut it = Intent::default();
                if self.shadowbox && f.standing() && !f.busy() && self.preview_cel.is_none() {
                    f.shadow_t -= FRAME;
                    if f.shadow_t <= 0.0 {
                        f.shadow_t = self.rng.range(1.2, 3.5);
                        match self.rng.below(5) {
                            0 | 1 => it.punch = true,
                            2 => it.kick = true,
                            _ => {}
                        }
                    }
                }
                // keep the fighter home
                let home = (ZERO - f.root).flat();
                if home.len() > 0.15 && f.standing() && !f.busy() {
                    it.mv = home.norm() * 0.4;
                }
                let yaw_home = menu_yaw();
                if f.standing() && self.preview_cel.is_none() {
                    let diff = wrap_angle(yaw_home - f.yaw);
                    f.yaw += clampf(diff, -0.1, 0.1);
                }
                f.think(&it, None, FRAME);
                if let (Some(k), FState::Celebrate { kind, .. }) = (self.preview_cel, f.state) {
                    if kind != k {
                        f.state = FState::Celebrate { kind: k, t: 0.0 };
                    }
                }
                f.compute_targets(None, self.eye);
                let sdt = FRAME / SUBSTEPS as f32;
                for _ in 0..SUBSTEPS {
                    f.integrate(sdt, 0.996);
                    f.apply_muscles();
                    for _ in 0..ITERS {
                        f.solve_sticks();
                    }
                    f.ground();
                }
                if !f.is_finite() {
                    f.respawn(ZERO, menu_yaw());
                }
                self.menu_fx.update(FRAME);
            }
        }
    }

    fn build_mesh(&mut self) {
        let m = &mut self.mesh;
        m.clear();
        match (&self.scene, &self.fight) {
            (Scene::Fight, Some(f)) => {
                let focus = (f.f[0].pos(PELVIS) + f.f[1].pos(PELVIS)) * 0.5;
                render::floor(m, focus);
                render::decals(m, &f.fx);
                for fi in &f.f {
                    render::shadow(m, fi);
                }
                for fi in &f.f {
                    render::fighter(m, fi);
                }
                render::particles(m, &f.fx, self.eye);
            }
            _ => {
                render::floor(m, ZERO);
                render::shadow(m, &self.menu_fighter);
                render::fighter(m, &self.menu_fighter);
                render::particles(m, &self.menu_fx, self.eye);
            }
        }
    }

    pub fn save(&mut self) -> *const std::os::raw::c_char {
        let s = self.profile.serialize();
        self.save_buf = std::ffi::CString::new(s).unwrap_or_default();
        self.save_buf.as_ptr()
    }

    pub fn load(&mut self, s: &str) -> bool {
        match Profile::deserialize(s) {
            Some(p) => {
                self.profile = p;
                self.sync_menu_fighter(true);
                true
            }
            None => false,
        }
    }
}
