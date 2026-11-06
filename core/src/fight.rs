//! The fight world: physics stepping, body-vs-body collisions, hit detection,
//! damage, grabs/throws, knockdowns, rounds.
use crate::ai::Ai;
use crate::catalog::*;
use crate::fighter::*;
use crate::fx::Fx;
use crate::math::*;

pub const SUBSTEPS: usize = 4;
pub const ITERS: usize = 3;
pub const FRAME: f32 = 1.0 / 60.0;
pub const ROUND_TIME: f32 = 45.0;
pub const ROUNDS: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Part {
    Head = 0,
    Body = 1,
    Arm = 2,
    Leg = 3,
}

/// Collision capsules: (a, b, radius scale, part)
pub const CAPS: [(usize, usize, f32, Part); 17] = [
    (HEAD, HEAD, 1.0, Part::Head),
    (PELVIS, SPINE, 0.15, Part::Body),
    (SPINE, CHEST, 0.16, Part::Body),
    (LSH, RSH, 0.075, Part::Body),
    (LSH, LEL, 0.055, Part::Arm),
    (LEL, LHA, 0.05, Part::Arm),
    (RSH, REL, 0.055, Part::Arm),
    (REL, RHA, 0.05, Part::Arm),
    (LHA, LHA, 0.065, Part::Arm),
    (RHA, RHA, 0.065, Part::Arm),
    (LHIP, LKN, 0.08, Part::Leg),
    (LKN, LFT, 0.06, Part::Leg),
    (RHIP, RKN, 0.08, Part::Leg),
    (RKN, RFT, 0.06, Part::Leg),
    (LFT, LTOE, 0.045, Part::Leg),
    (RFT, RTOE, 0.045, Part::Leg),
    (NECK, NECK, 0.06, Part::Head),
];

fn cap_radius(f: &Fighter, c: usize) -> f32 {
    let (_, _, r, _) = CAPS[c];
    if c == 0 {
        f.dims.head_r
    } else {
        r * f.dims.w.sqrt()
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub fighter: u32,
    pub a: f32,
    pub b: f32,
}
pub const EV_HIT: u32 = 1;
pub const EV_BLOCK: u32 = 2;
pub const EV_PARRY: u32 = 3;
pub const EV_KNOCKDOWN: u32 = 4;
pub const EV_KO: u32 = 5;
pub const EV_ROUND_START: u32 = 6;
pub const EV_ROUND_END: u32 = 7;
pub const EV_FIGHT_END: u32 = 8;
pub const EV_THROW: u32 = 9;
pub const EV_THUD: u32 = 10;
pub const EV_FINISHER: u32 = 12;
pub const EV_COUNTER: u32 = 13;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Intro { t: f32 },
    Fight,
    RoundEnd { t: f32 },
    Ko { t: f32 },
    Celebrate { t: f32 },
    Over,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Method {
    None = 0,
    Ko = 1,
    Tko = 2,
    Decision = 3,
    Finisher = 4,
}

pub struct Fight {
    pub f: [Fighter; 2],
    pub ai: Ai,
    pub fx: Fx,
    pub events: Vec<Event>,
    pub rng: Rng,
    pub phase: Phase,
    pub round: u32,
    pub round_t: f32,
    pub time: f32,
    pub slowmo: f32,
    pub slowmo_t: f32,
    pub winner: Option<usize>,
    pub method: Method,
    pub opp_index: usize,
    pub banner: u32,
    pub banner_t: f32,
    pub player_celebration: u32,
    pub ai_celebration: u32,
    pub hitstop: f32,
    pub escape_input: bool,
}

impl Fight {
    pub fn new(player_arch: Archetype, player_look: Look, opp_index: usize, player_celebration: u32, seed: u64) -> Fight {
        let o = &OPPONENTS[opp_index.min(OPPONENTS.len() - 1)];
        let oa = ARCHETYPES[o.archetype as usize];
        let ol = Look { items: o.look };
        let p = Fighter::new(player_arch, player_look, v3(-2.6, 0.0, 0.0), 0.0, seed ^ 0xA1);
        let mut e = Fighter::new(oa, ol, v3(2.6, 0.0, 0.0), std::f32::consts::PI, seed ^ 0xB2);
        // AI fighters get a slight power scaling with skill
        e.arch.power *= 0.75 + 0.35 * o.skill;
        let mut rng = Rng::new(seed);
        let ai_cel = rng.below(CELEBRATIONS.len() as u32);
        Fight {
            f: [p, e],
            ai: Ai::new(o.skill, seed ^ 0x51),
            fx: Fx::new(seed ^ 0x77),
            events: Vec::new(),
            rng,
            phase: Phase::Intro { t: 0.0 },
            round: 1,
            round_t: ROUND_TIME,
            time: 0.0,
            slowmo: 1.0,
            slowmo_t: 0.0,
            winner: None,
            method: Method::None,
            opp_index,
            banner: 1,
            banner_t: 0.0,
            player_celebration,
            ai_celebration: ai_cel,
            hitstop: 0.0,
            escape_input: false,
        }
    }

    fn ev(&mut self, kind: u32, fighter: usize, a: f32, b: f32) {
        if self.events.len() < 128 {
            self.events.push(Event { kind, fighter: fighter as u32, a, b });
        }
    }

    fn set_banner(&mut self, b: u32) {
        self.banner = b;
        self.banner_t = 0.0;
    }

    /// Advance one fixed frame (game time already scaled).
    pub fn step(&mut self, player: Intent, cam_pos: V3) {
        let dt = FRAME;
        self.time += dt;
        self.banner_t += dt;
        if self.hitstop > 0.0 {
            self.hitstop -= dt;
            self.fx.update(dt * 0.2);
            return;
        }
        let mut intents = [player, Intent::default()];
        let fighting = matches!(self.phase, Phase::Fight);
        if fighting {
            let (a, b) = self.f.split_at_mut(1);
            intents[1] = self.ai.intent(&b[0], &a[0], dt);
        } else {
            intents[0] = Intent::default();
        }
        // phase logic
        match self.phase {
            Phase::Intro { t } => {
                let t = t + dt;
                if t > 1.3 && self.banner == 1 {
                    self.set_banner(2);
                    self.ev(EV_ROUND_START, 0, self.round as f32, 0.0);
                }
                // walk toward each other
                for i in 0..2 {
                    let to = (self.f[1 - i].root - self.f[i].root).flat();
                    if to.len() > 2.0 {
                        intents[i].mv = to.norm() * 0.6;
                    }
                }
                self.phase = if t > 2.2 { Phase::Fight } else { Phase::Intro { t } };
            }
            Phase::Fight => {
                self.round_t -= dt;
                if self.round_t <= 0.0 {
                    self.round_t = 0.0;
                    self.ev(EV_ROUND_END, 0, self.round as f32, 0.0);
                    if self.round >= ROUNDS {
                        // decision
                        let s0 = self.score(0);
                        let s1 = self.score(1);
                        let w = if s0 >= s1 { 0 } else { 1 };
                        self.finish(w, Method::Decision);
                        self.set_banner(5);
                    } else {
                        self.phase = Phase::RoundEnd { t: 0.0 };
                        self.set_banner(9);
                    }
                }
            }
            Phase::RoundEnd { t } => {
                let t = t + dt;
                if t > 2.5 {
                    self.round += 1;
                    self.round_t = ROUND_TIME;
                    for (i, x) in [-2.6f32, 2.6].iter().enumerate() {
                        let f = &mut self.f[i];
                        f.respawn(v3(*x, 0.0, 0.0), if i == 0 { 0.0 } else { std::f32::consts::PI });
                        f.stamina = 100.0;
                        f.stun = 0.0;
                        f.balance = 1.0;
                        f.hp_head = (f.hp_head + 15.0).min(100.0);
                        f.hp_body = (f.hp_body + 15.0).min(100.0);
                        f.hp_legs = (f.hp_legs + 20.0).min(100.0);
                        f.grabbed_by_other = false;
                    }
                    self.phase = Phase::Intro { t: 0.0 };
                    self.set_banner(1);
                } else {
                    self.phase = Phase::RoundEnd { t };
                }
            }
            Phase::Ko { t } => {
                let t = t + dt;
                if t > 2.0 {
                    let w = self.winner.unwrap_or(0);
                    let kind = if w == 0 { self.player_celebration } else { self.ai_celebration };
                    self.f[w].state = FState::Celebrate { kind, t: 0.0 };
                    self.f[w].action = Action::Idle;
                    self.f[w].spin = 0.0;
                    let at = self.f[w].pos(HEAD);
                    self.fx.confetti(at + UP * 0.8, 120);
                    self.phase = Phase::Celebrate { t: 0.0 };
                } else {
                    self.phase = Phase::Ko { t };
                }
            }
            Phase::Celebrate { t } => {
                let t = t + dt;
                if t > 5.0 {
                    self.phase = Phase::Over;
                    let w = self.winner.unwrap_or(0);
                    self.ev(EV_FIGHT_END, 0, if w == 0 { 1.0 } else { 0.0 }, self.method as u32 as f32);
                } else {
                    self.phase = Phase::Celebrate { t };
                }
            }
            Phase::Over => {}
        }

        // clinch escape input from the defender
        for i in 0..2 {
            if self.f[i].grabbed_by_other {
                let pressing = intents[i].grab || intents[i].block || intents[i].punch;
                let add = if pressing { 0.35 } else { dt * 0.25 };
                self.f[i].escape += add;
                if self.f[i].escape > 1.0 {
                    self.f[i].escape = 0.0;
                    self.f[i].grabbed_by_other = false;
                    if let Action::Clinch { .. } = self.f[1 - i].action {
                        self.f[1 - i].action = Action::Hurt { t: 0.3 };
                    }
                    let push = (self.f[1 - i].pos(CHEST) - self.f[i].pos(CHEST)).flat().norm() * 2.5;
                    for p in [CHEST, SPINE, NECK] {
                        self.f[1 - i].add_impulse(p, push, FRAME / SUBSTEPS as f32);
                    }
                }
            } else {
                self.f[i].escape = 0.0;
            }
        }

        // think
        let views = [self.f[0].view(), self.f[1].view()];
        for i in 0..2 {
            let opp = views[1 - i];
            self.f[i].think(&intents[i], Some(&opp), dt);
        }
        // grab release bookkeeping
        for i in 0..2 {
            let attached = matches!(self.f[i].action, Action::Clinch { attached: true, .. })
                || matches!(self.f[i].action, Action::Strike { mv: M_KNEE, .. } | Action::Strike { mv: M_UPPER, .. }) && self.f[1 - i].grabbed_by_other;
            if !attached && !matches!(self.f[i].action, Action::Throw { thrown: false, .. }) {
                self.f[1 - i].grabbed_by_other = false;
            }
            if !self.f[1 - i].standing() {
                self.f[1 - i].grabbed_by_other = false;
            }
        }
        let views = [self.f[0].view(), self.f[1].view()];
        for i in 0..2 {
            self.f[i].compute_targets(Some(&views[1 - i]), cam_pos);
        }

        // throws
        for i in 0..2 {
            if let Action::Throw { t, thrown: false } = self.f[i].action {
                if t >= 0.32 {
                    self.f[i].action = Action::Throw { t, thrown: true };
                    let d = (self.f[1 - i].pos(PELVIS) - self.f[i].pos(PELVIS)).flat().len();
                    let ok = self.f[1 - i].grabbed_by_other || (self.f[i].special_throw && d < 1.3);
                    if ok && self.f[1 - i].standing() {
                        self.do_throw(i);
                    }
                    self.f[i].special_throw = false;
                }
            }
        }

        // physics substeps
        let sdt = dt / SUBSTEPS as f32;
        for _ in 0..SUBSTEPS {
            for f in self.f.iter_mut() {
                f.integrate(sdt, 0.996);
                f.apply_muscles();
            }
            for _ in 0..ITERS {
                for f in self.f.iter_mut() {
                    f.solve_sticks();
                }
                self.solve_grabs();
                self.collide_bodies();
            }
            for i in 0..2 {
                let (imp, at) = self.f[i].ground();
                if imp > 2.6 && self.f[i].is_down() {
                    self.fx.dust(at, imp);
                    self.fx.shake = (self.fx.shake + imp * 0.06).min(1.0);
                    self.ev(EV_THUD, i, imp, 0.0);
                    if !self.f[i].is_ko() {
                        self.f[i].hp_body -= (imp - 2.6) * 1.5;
                    }
                }
            }
            self.check_hits(sdt);
            self.check_clinch();
        }
        // NaN safety net
        for i in 0..2 {
            if !self.f[i].is_finite() {
                let r = self.f[i].root;
                let y = self.f[i].yaw;
                self.f[i].respawn(if r.is_finite() { r } else { ZERO }, y);
            }
        }
        if self.slowmo_t > 0.0 {
            self.slowmo_t -= dt / self.slowmo.max(0.05);
            if self.slowmo_t <= 0.0 {
                self.slowmo = 1.0;
            }
        }
        self.fx.update(dt);
    }

    fn score(&self, i: usize) -> f32 {
        let me = &self.f[i];
        let o = &self.f[1 - i];
        me.stats.damage + o.knockdowns as f32 * 25.0 + me.stats.landed as f32 * 0.5
    }

    fn finish(&mut self, winner: usize, m: Method) {
        if self.winner.is_some() {
            return;
        }
        self.winner = Some(winner);
        self.method = m;
        self.phase = Phase::Ko { t: 0.0 };
        if m != Method::Decision {
            self.slowmo = 0.25;
            self.slowmo_t = 1.2;
        }
        self.ev(EV_KO, 1 - winner, m as u32 as f32, 0.0);
    }

    fn solve_grabs(&mut self) {
        for i in 0..2 {
            let attached = self.f[1 - i].grabbed_by_other;
            if !attached {
                continue;
            }
            let (a, b) = if i == 0 {
                let (x, y) = self.f.split_at_mut(1);
                (&mut x[0], &mut y[0])
            } else {
                let (x, y) = self.f.split_at_mut(1);
                (&mut y[0], &mut x[0])
            };
            for (h, side) in [(LHA, -1.0f32), (RHA, 1.0)] {
                let r = (b.body[RSH].p - b.body[LSH].p).norm();
                let anchor = b.body[NECK].p + r * (0.08 * side);
                let d = anchor - a.body[h].p;
                let l = d.len();
                let rest = 0.06;
                if l > rest {
                    let wa = a.body[h].inv_m;
                    let wb = b.body[NECK].inv_m;
                    let c = d * ((l - rest) / l / (wa + wb) * 0.6);
                    a.body[h].p += c * wa;
                    b.body[NECK].p -= c * wb;
                }
            }
        }
    }

    fn collide_bodies(&mut self) {
        let (x, y) = self.f.split_at_mut(1);
        let a = &mut x[0];
        let b = &mut y[0];
        // arms only clash physically when someone is actively guarding
        let arm_clash = a.blocking() || b.blocking();
        for ca in 0..CAPS.len() {
            let (a0, a1, _, pa_part) = CAPS[ca];
            let ra = cap_radius(a, ca);
            for cb in 0..CAPS.len() {
                let (b0, b1, _, pb_part) = CAPS[cb];
                if !arm_clash && pa_part == Part::Arm && pb_part == Part::Arm {
                    continue;
                }
                let rb = cap_radius(b, cb);
                let (s, t) = seg_seg(a.body[a0].p, a.body[a1].p, b.body[b0].p, b.body[b1].p);
                let pa = a.body[a0].p.lerp(a.body[a1].p, s);
                let pb = b.body[b0].p.lerp(b.body[b1].p, t);
                let d = pb - pa;
                let dist2 = d.len2();
                let rr = ra + rb;
                if dist2 >= rr * rr {
                    continue;
                }
                let dist = dist2.sqrt();
                let n = if dist > 1e-5 { d / dist } else { (b.body[PELVIS].p - a.body[PELVIS].p).norm_or(v3(1.0, 0.0, 0.0)) };
                let pen = rr - dist;
                let (ca0, ca1) = if a0 == a1 { (1.0, 0.0) } else { (1.0 - s, s) };
                let (cb0, cb1) = if b0 == b1 { (1.0, 0.0) } else { (1.0 - t, t) };
                let (ia0, ia1, ib0, ib1) = (a.body[a0].inv_m, a.body[a1].inv_m, b.body[b0].inv_m, b.body[b1].inv_m);
                let wsum = ca0 * ca0 * ia0 + ca1 * ca1 * ia1 + cb0 * cb0 * ib0 + cb1 * cb1 * ib1;
                if wsum <= 1e-8 {
                    continue;
                }
                let lam = pen / wsum * 0.8;
                let (wa0, wa1, wb0, wb1) = (ca0 * ia0, ca1 * ia1, cb0 * ib0, cb1 * ib1);
                a.body[a0].p -= n * (lam * wa0);
                if a0 != a1 {
                    a.body[a1].p -= n * (lam * wa1);
                }
                b.body[b0].p += n * (lam * wb0);
                if b0 != b1 {
                    b.body[b1].p += n * (lam * wb1);
                }
            }
        }
    }

    fn check_clinch(&mut self) {
        for i in 0..2 {
            if let Action::Clinch { t, attached: false } = self.f[i].action {
                if !self.f[1 - i].standing() {
                    continue;
                }
                let neck = self.f[1 - i].pos(NECK);
                let dl = (self.f[i].pos(LHA) - neck).len();
                let dr = (self.f[i].pos(RHA) - neck).len();
                let reach = 0.3 * self.f[i].arch.grapple.sqrt();
                if dl < reach && dr < reach {
                    let blocked = self.f[1 - i].blocking() && self.rng.chance(0.35);
                    if !blocked {
                        self.f[i].action = Action::Clinch { t, attached: true };
                        self.f[1 - i].grabbed_by_other = true;
                        self.f[1 - i].escape = 0.0;
                        if matches!(self.f[1 - i].action, Action::Strike { .. }) {
                            self.f[1 - i].action = Action::Idle;
                        }
                    }
                }
            }
        }
    }

    fn do_throw(&mut self, i: usize) {
        let grap = self.f[i].arch.grapple;
        let back = -self.f[i].facing();
        let special = self.f[i].special_throw;
        let rocked = self.f[1 - i].rocked();
        self.f[1 - i].grabbed_by_other = false;
        let d = &mut self.f[1 - i];
        d.knock_down(2.0);
        let up = d.pos(PELVIS).y;
        for k in 0..NP {
            let above = d.body[k].p.y > up;
            let v = UP * 4.8 + back * 2.6 + if above { back * 2.2 + UP * 0.5 } else { -back * 1.8 + UP * 2.6 };
            d.add_impulse(k, v * (0.75 + 0.25 * grap), FRAME / SUBSTEPS as f32);
        }
        let dmg = if special { 26.0 } else { 11.0 * grap };
        d.hp_body -= dmg;
        d.hp_head -= if special { 12.0 } else { 3.0 };
        d.balance = 0.0;
        d.bruise[1] += 0.15;
        self.f[i].stats.throws += 1;
        self.f[i].stats.damage += dmg;
        self.f[i].stats.knockdowns += 1;
        self.f[i].special = (self.f[i].special + dmg * 1.2).min(100.0);
        self.ev(EV_THROW, 1 - i, dmg, 0.0);
        self.ev(EV_KNOCKDOWN, 1 - i, 0.0, 0.0);
        self.set_banner(6);
        self.fx.shake = (self.fx.shake + 0.4).min(1.0);
        if special && rocked {
            self.f[i].stats.finisher = true;
            self.f[1 - i].ko();
            self.ev(EV_FINISHER, i, 3.0, 0.0);
            self.finish(i, Method::Finisher);
            self.set_banner(3);
        } else if self.f[1 - i].knockdowns >= 3 || self.f[1 - i].hp_head <= 0.0 {
            self.f[1 - i].ko();
            self.finish(i, Method::Tko);
            self.set_banner(4);
        }
    }

    fn check_hits(&mut self, sdt: f32) {
        for i in 0..2 {
            let Action::Strike { mv, t, hit: false, finisher, power, .. } = self.f[i].action else { continue };
            let dm = MOVES[mv];
            let start = dm.windup;
            let end = dm.windup + dm.active;
            if t < start || t > end {
                continue;
            }
            if !(self.f[1 - i].standing() || matches!(self.f[1 - i].state, FState::GetUp { .. })) {
                continue;
            }
            let e = dm.end;
            let ep = self.f[i].pos(e);
            let er = match e {
                LHA | RHA => 0.075,
                RKN | LKN => 0.08,
                _ => 0.07,
            };
            // optional shin sample for kicks
            let shin = if dm.kick && (e == LFT || e == RFT) {
                let k = if e == LFT { LKN } else { RKN };
                Some(self.f[i].pos(k).lerp(ep, 0.6))
            } else {
                None
            };
            let def = &self.f[1 - i];
            let aim_part = match dm.aim {
                Aim::Head => Part::Head,
                Aim::Body => Part::Body,
                Aim::Legs => Part::Leg,
            };
            let guarding = def.blocking();
            let mut best: Option<(f32, usize, V3)> = None;
            let mut best_arm: Option<(f32, usize, V3)> = None;
            for c in 0..CAPS.len() {
                let (a0, a1, _, part) = CAPS[c];
                let rc = cap_radius(def, c);
                let margin = if part == aim_part && !guarding { 0.07 } else { 0.03 };
                for sp in [Some(ep), shin].into_iter().flatten() {
                    let pa = def.pos(a0);
                    let pb = def.pos(a1);
                    let ab = pb - pa;
                    let tt = if a0 == a1 { 0.0 } else { clampf((sp - pa).dot(ab) / ab.len2(), 0.0, 1.0) };
                    let cp = pa + ab * tt;
                    let dist = (sp - cp).len() - rc - er - margin;
                    if dist < 0.0 {
                        let slot = if part == Part::Arm { &mut best_arm } else { &mut best };
                        let score = dist - if part == aim_part { 0.03 } else { 0.0 };
                        if slot.map(|b| score < b.0).unwrap_or(true) {
                            *slot = Some((score, c, cp.lerp(sp, 0.5)));
                        }
                    }
                }
            }
            // an active guard catches the strike on the arms first
            if guarding && best_arm.is_some() {
                best = best_arm;
            } else if best.is_none() && t > start + dm.active * 0.8 {
                // the strike was smothered by the guard (glancing hit)
                best = best_arm;
            }
            if let Some((_, c, at)) = best {
                if let Action::Strike { hit, .. } = &mut self.f[i].action {
                    *hit = true;
                }
                self.apply_hit(i, mv, c, at, finisher, power, sdt);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_hit(&mut self, ai: usize, mv: usize, cap: usize, at: V3, finisher: bool, power: f32, sdt: f32) {
        let di = 1 - ai;
        let dm = MOVES[mv];
        let part = CAPS[cap].3;
        let vel_end = self.f[ai].body[dm.end].vel;
        let vel_def = self.f[di].body[CAPS[cap].0].vel;
        let rel = vel_end - vel_def;
        let speed = rel.len();
        let refv = if dm.kick { 8.0 } else { 6.5 };
        let sf = clampf(speed / refv, 0.5, 1.35);
        let mut dmg = power * sf;
        let hit_dir = (rel.norm_or(self.f[ai].facing()) * 0.6 + (self.f[di].pos(PELVIS) - self.f[ai].pos(PELVIS)).flat().norm() * 0.4).norm();
        let defender_facing = self.f[di].facing();
        let in_front = defender_facing.dot(-hit_dir) > 0.2;
        let curved = matches!(dm.style, Style::Hook | Style::Upper | Style::Round | Style::HighKick | Style::Haymaker | Style::SpinKick);
        let blocking = self.f[di].blocking();
        let mut blocked = false;
        if blocking && in_front && part != Part::Leg {
            blocked = part == Part::Arm || !curved || self.rng.chance(0.45);
        }
        if finisher {
            blocked = false;
        }
        if blocked {
            let parry = self.f[di].parry_window > 0.0 && !dm.special;
            self.f[di].stamina -= dmg * 0.9;
            self.f[di].stats.blocked += 1;
            self.fx.impact(at, -hit_dir, dmg * 0.4);
            for p in [LHA, RHA, LEL, REL] {
                self.f[di].add_impulse(p, hit_dir * (dm.knock * 0.6), sdt);
            }
            self.f[di].root_vel += hit_dir.flat() * (dm.knock * 0.12);
            if parry {
                self.f[ai].action = Action::Hurt { t: 0.55 };
                self.f[ai].balance -= 0.35;
                self.ev(EV_PARRY, di, dmg, 0.0);
            } else {
                self.ev(EV_BLOCK, di, dmg, 0.0);
                if self.f[di].stamina <= 0.0 {
                    self.f[di].action = Action::Hurt { t: 0.4 };
                }
            }
            self.hitstop = 0.03;
            return;
        }
        // counter hit
        let counter = matches!(self.f[di].action, Action::Strike { t, mv: m2, .. } if t < MOVES[m2].windup + MOVES[m2].active);
        if counter {
            dmg *= 1.3;
            self.ev(EV_COUNTER, ai, dmg, 0.0);
            self.set_banner(8);
        }
        if part == Part::Arm {
            dmg *= 0.35;
        }
        let chin = self.f[di].arch.chin;
        let knock = dm.knock * sf;
        {
            let d = &mut self.f[di];
            match part {
                Part::Head => {
                    d.hp_head -= dmg * 1.6 / chin;
                    d.stun += dmg * 3.0 / chin;
                    d.bruise[0] = (d.bruise[0] + dmg * 0.012).min(1.0);
                    d.blood = (d.blood + dmg * 0.012).min(1.0);
                    d.balance -= dmg * 0.022;
                    d.add_impulse(HEAD, hit_dir * (knock * 2.4), sdt);
                    d.add_impulse(NECK, hit_dir * (knock * 1.2), sdt);
                    d.add_impulse(CHEST, hit_dir * (knock * 0.4), sdt);
                }
                Part::Body => {
                    d.hp_body -= dmg;
                    d.stun += dmg * 0.5;
                    d.bruise[1] = (d.bruise[1] + dmg * 0.01).min(1.0);
                    d.balance -= dmg * 0.018;
                    for p in [SPINE, CHEST, PELVIS] {
                        d.add_impulse(p, hit_dir * (knock * 1.1), sdt);
                    }
                }
                Part::Leg => {
                    d.hp_legs -= dmg * 1.3;
                    d.balance -= dmg * 0.03;
                    let leg = if cap >= 12 && cap != 16 { 4 } else { 5 };
                    d.bruise[leg.min(5)] = (d.bruise[leg.min(5)] + dmg * 0.015).min(1.0);
                    for p in [LKN, RKN, LFT, RFT] {
                        d.add_impulse(p, (hit_dir + UP * 0.3) * (knock * 0.9), sdt);
                    }
                }
                Part::Arm => {
                    let arm = if cap == 4 || cap == 5 || cap == 8 { 2 } else { 3 };
                    d.bruise[arm] = (d.bruise[arm] + dmg * 0.01).min(1.0);
                    for p in [LHA, RHA] {
                        d.add_impulse(p, hit_dir * knock, sdt);
                    }
                }
            }
            if dm.style == Style::Teep {
                for k in 0..NP {
                    d.add_impulse(k, hit_dir.flat() * (knock * 0.5), sdt);
                }
                d.root_vel += hit_dir.flat() * (knock * 0.35);
            }
            d.root_vel += hit_dir.flat() * (knock * 0.18);
            d.last_hit_dir = hit_dir;
            if !d.is_ko() {
                d.action = Action::Hurt { t: 0.12 + dmg * 0.012 };
            }
        }
        {
            let a = &mut self.f[ai];
            a.special = (a.special + dmg * 1.1).min(100.0);
            a.stats.landed += 1;
            if dm.kick {
                a.stats.kicks += 1;
            } else {
                a.stats.punches += 1;
            }
            a.stats.damage += dmg;
        }
        // FX
        self.fx.impact(at, -hit_dir, dmg);
        if part == Part::Head {
            let hp = self.f[di].pos(HEAD);
            self.fx.sweat(hp, hit_dir, 6 + dmg as usize / 2);
            let bl = self.f[di].blood;
            if dmg > 6.0 || bl > 0.25 {
                let n = ((dmg * 0.5) * (0.4 + bl * 1.4)) as usize;
                self.fx.blood(hp, hit_dir, n.min(40), 2.5 + dmg * 0.08);
            }
        } else if dmg > 12.0 {
            let sp = self.f[di].pos(CHEST);
            self.fx.sweat(sp, hit_dir, 6);
        }
        self.fx.shake = (self.fx.shake + dmg * 0.025).min(1.0);
        self.hitstop = if dmg > 15.0 { 0.07 } else if dmg > 8.0 { 0.035 } else { 0.0 };
        self.ev(EV_HIT, di, dmg, part as u32 as f32);

        // outcomes
        let d = &self.f[di];
        let ko = (part == Part::Head && (d.hp_head <= 0.0 || (finisher && part == Part::Head))) || (finisher && dmg > 0.0);
        if ko {
            if finisher {
                self.f[ai].stats.finisher = true;
                self.ev(EV_FINISHER, ai, 0.0, 0.0);
            }
            self.f[di].hp_head = self.f[di].hp_head.min(0.0);
            self.f[di].ko();
            for k in [HEAD, NECK] {
                self.f[di].add_impulse(k, hit_dir * (2.0 + knock), sdt);
            }
            self.fx.flash = 1.0;
            self.finish(ai, if finisher { Method::Finisher } else { Method::Ko });
            self.set_banner(3);
            return;
        }
        let (stun, bal, body, legs) = (d.stun, d.balance, d.hp_body, d.hp_legs);
        let mut knockdown = None;
        if body <= 0.0 {
            knockdown = Some(2.6);
            self.f[di].hp_body = 20.0;
        } else if stun > 100.0 || bal < 0.0 {
            knockdown = Some(1.6 + stun * 0.006);
        } else if legs <= 0.0 && part == Part::Leg {
            knockdown = Some(1.8);
            self.f[di].hp_legs = 25.0;
        }
        if let Some(dur) = knockdown {
            self.f[di].knock_down(dur);
            self.f[di].stun = 40.0;
            self.f[di].balance = 1.0;
            self.f[di].grabbed_by_other = false;
            self.f[ai].stats.knockdowns += 1;
            self.ev(EV_KNOCKDOWN, di, 0.0, 0.0);
            self.set_banner(6);
            self.slowmo = 0.4;
            self.slowmo_t = 0.35;
            if self.f[di].knockdowns >= 3 {
                self.f[di].ko();
                self.finish(ai, Method::Tko);
                self.set_banner(4);
            }
        }
    }
}
