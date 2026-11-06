//! Active-ragdoll fighter: a Verlet particle skeleton driven by "muscles" that pull
//! particles toward a procedurally generated target pose. Nothing is a canned
//! animation — every punch, stumble, fall and backflip is the result of the
//! physics simulation trying to follow an intention.
use crate::catalog::{Archetype, Look};
use crate::math::*;
use std::f32::consts::PI;

pub const PELVIS: usize = 0;
pub const SPINE: usize = 1;
pub const CHEST: usize = 2;
pub const NECK: usize = 3;
pub const HEAD: usize = 4;
pub const LSH: usize = 5;
pub const LEL: usize = 6;
pub const LHA: usize = 7;
pub const RSH: usize = 8;
pub const REL: usize = 9;
pub const RHA: usize = 10;
pub const LHIP: usize = 11;
pub const LKN: usize = 12;
pub const LFT: usize = 13;
pub const RHIP: usize = 14;
pub const RKN: usize = 15;
pub const RFT: usize = 16;
pub const LTOE: usize = 17;
pub const RTOE: usize = 18;
pub const NP: usize = 19;

#[derive(Clone, Copy, Debug, Default)]
pub struct Particle {
    pub p: V3,
    pub prev: V3,
    pub vel: V3,
    pub inv_m: f32,
    pub r: f32,
    pub grounded: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Stick {
    pub a: u8,
    pub b: u8,
    pub rest: f32,
    pub stiff: f32,
    /// 0 = equality, 1 = min distance only, 2 = max distance only
    pub kind: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Dims {
    pub h: f32,
    pub w: f32,
    pub shin: f32,
    pub thigh: f32,
    pub foot: f32,
    pub hip_w: f32,
    pub sh_w: f32,
    pub ps: f32,
    pub sc: f32,
    pub cn: f32,
    pub nh: f32,
    pub ua: f32,
    pub fa: f32,
    pub head_r: f32,
    pub stand_pelvis: f32,
}
impl Dims {
    pub fn new(h: f32, w: f32) -> Dims {
        let shin = 0.44 * h;
        let thigh = 0.44 * h;
        Dims {
            h,
            w,
            shin,
            thigh,
            foot: 0.17 * h,
            hip_w: 0.095 * w,
            sh_w: 0.19 * w,
            ps: 0.19 * h,
            sc: 0.19 * h,
            cn: 0.16 * h,
            nh: 0.13 * h,
            ua: 0.29 * h,
            fa: 0.27 * h,
            head_r: 0.115 * h,
            stand_pelvis: 0.06 + shin + thigh + 0.03 * h,
        }
    }
}

// ------------------------------------------------------------------ moves
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Jab,
    Cross,
    Hook,
    Upper,
    Round,
    HighKick,
    Teep,
    LowKick,
    Knee,
    Superman,
    SpinKick,
    Haymaker,
    FlyKnee,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    Head,
    Body,
    Legs,
}
#[derive(Clone, Copy, Debug)]
pub struct MoveDef {
    pub style: Style,
    pub end: usize, // striking particle
    pub aim: Aim,
    pub windup: f32,
    pub active: f32,
    pub recover: f32,
    pub power: f32,
    pub knock: f32,
    pub stamina: f32,
    pub kick: bool,
    pub special: bool,
}
const fn mv(style: Style, end: usize, aim: Aim, w: f32, a: f32, r: f32, power: f32, knock: f32, st: f32, kick: bool, special: bool) -> MoveDef {
    MoveDef { style, end, aim, windup: w, active: a, recover: r, power, knock, stamina: st, kick, special }
}
pub const M_JAB: usize = 0;
pub const M_CROSS: usize = 1;
pub const M_HOOK: usize = 2;
pub const M_UPPER: usize = 3;
pub const M_ROUND: usize = 4;
pub const M_HIGH: usize = 5;
pub const M_TEEP: usize = 6;
pub const M_LOW: usize = 7;
pub const M_KNEE: usize = 8;
pub const M_SUPERMAN: usize = 9;
pub const M_SPIN: usize = 10;
pub const M_HAYMAKER: usize = 11;
pub const M_FLYKNEE: usize = 12;
pub static MOVES: [MoveDef; 13] = [
    mv(Style::Jab, LHA, Aim::Head, 0.06, 0.13, 0.13, 5.0, 1.4, 4.0, false, false),
    mv(Style::Cross, RHA, Aim::Head, 0.09, 0.14, 0.18, 9.0, 2.4, 7.0, false, false),
    mv(Style::Hook, LHA, Aim::Head, 0.12, 0.15, 0.2, 11.0, 2.8, 8.0, false, false),
    mv(Style::Upper, RHA, Aim::Head, 0.13, 0.14, 0.24, 12.0, 3.2, 9.0, false, false),
    mv(Style::Round, RFT, Aim::Body, 0.16, 0.19, 0.28, 12.0, 3.5, 11.0, true, false),
    mv(Style::HighKick, RFT, Aim::Head, 0.2, 0.2, 0.32, 17.0, 4.5, 14.0, true, false),
    mv(Style::Teep, LFT, Aim::Body, 0.13, 0.15, 0.24, 7.0, 6.0, 8.0, true, false),
    mv(Style::LowKick, RFT, Aim::Legs, 0.12, 0.15, 0.22, 8.0, 1.5, 8.0, true, false),
    mv(Style::Knee, RKN, Aim::Body, 0.11, 0.15, 0.2, 10.0, 3.0, 8.0, true, false),
    mv(Style::Superman, RHA, Aim::Head, 0.28, 0.2, 0.32, 22.0, 6.5, 0.0, false, true),
    mv(Style::SpinKick, LFT, Aim::Head, 0.34, 0.2, 0.34, 24.0, 7.5, 0.0, true, true),
    mv(Style::Haymaker, RHA, Aim::Head, 0.3, 0.18, 0.36, 26.0, 7.5, 0.0, false, true),
    mv(Style::FlyKnee, RKN, Aim::Head, 0.26, 0.22, 0.36, 24.0, 6.5, 0.0, true, true),
];
pub const PUNCH_CHAIN: [usize; 4] = [M_JAB, M_CROSS, M_HOOK, M_UPPER];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Idle,
    Block,
    Strike { mv: usize, t: f32, hit: bool, aim: V3, finisher: bool, power: f32 },
    Clinch { t: f32, attached: bool },
    Throw { t: f32, thrown: bool },
    Hurt { t: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FState {
    Stand,
    Down { t: f32, dur: f32 },
    GetUp { t: f32 },
    Ko { t: f32 },
    Celebrate { kind: u32, t: f32 },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Intent {
    /// desired world-space move direction, length <= 1
    pub mv: V3,
    /// stick modifiers relative to the opponent
    pub fwd: bool,
    pub back: bool,
    pub down: bool,
    pub punch: bool,
    pub kick: bool,
    pub grab: bool,
    pub special: bool,
    pub block: bool,
}

/// What a fighter can see of its opponent.
#[derive(Clone, Copy, Debug)]
pub struct OppView {
    pub pos: [V3; NP],
    pub root: V3,
    pub standing: bool,
    pub rocked: bool,
    pub blocking: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Foot {
    pub planted: V3,
    pub from: V3,
    pub to: V3,
    pub t: f32,
    pub stepping: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FightStats {
    pub thrown: u32,
    pub landed: u32,
    pub punches: u32,
    pub kicks: u32,
    pub damage: f32,
    pub knockdowns: u32,
    pub throws: u32,
    pub blocked: u32,
    pub finisher: bool,
}

pub struct Fighter {
    pub body: [Particle; NP],
    pub sticks: Vec<Stick>,
    pub targets: [V3; NP],
    pub kmul: [f32; NP],
    pub keep: [f32; NP],
    pub dims: Dims,
    pub arch: Archetype,
    pub look: Look,
    pub root: V3,
    pub root_vel: V3,
    pub yaw: f32,
    pub feet: [Foot; 2],
    pub state: FState,
    pub action: Action,
    pub strength: f32,
    pub hp_head: f32,
    pub hp_body: f32,
    pub hp_legs: f32,
    pub stamina: f32,
    pub stun: f32,
    pub balance: f32,
    pub special: f32,
    /// bruising per visual region: head, torso, larm, rarm, lleg, rleg
    pub bruise: [f32; 6],
    pub blood: f32,
    pub combo: usize,
    pub combo_timer: f32,
    pub buf: [f32; 4], // punch, kick, grab, special press buffers
    pub knockdowns: u32,
    pub time: f32,
    pub grabbed_by_other: bool,
    pub escape: f32,
    pub parry_window: f32,
    pub stats: FightStats,
    pub rng: Rng,
    pub menu_mode: bool,
    pub shadow_t: f32,
    pub spin: f32,
    pub look_at: V3,
    pub last_hit_dir: V3,
    pub special_throw: bool,
}

pub const B_PUNCH: usize = 0;
pub const B_KICK: usize = 1;
pub const B_GRAB: usize = 2;
pub const B_SPECIAL: usize = 3;

fn masses() -> [f32; NP] {
    [12.0, 9.0, 11.0, 2.0, 5.0, 3.0, 2.0, 1.2, 3.0, 2.0, 1.2, 4.0, 3.0, 1.6, 4.0, 3.0, 1.6, 0.5, 0.5]
}

impl Fighter {
    pub fn new(arch: Archetype, look: Look, root: V3, yaw: f32, seed: u64) -> Fighter {
        let d = Dims::new(arch.height, arch.bulk);
        let fr = Frame::from_yaw(root.flat(), yaw);
        let rest = rest_pose(&d);
        let mut body = [Particle::default(); NP];
        let m = masses();
        let radii = [
            0.1, 0.1, 0.11, 0.06, d.head_r, 0.07, 0.05, 0.06, 0.07, 0.05, 0.06, 0.08, 0.06, 0.055, 0.08, 0.06, 0.055, 0.035, 0.035,
        ];
        for i in 0..NP {
            let p = fr.p(rest[i]);
            body[i] = Particle { p, prev: p, vel: ZERO, inv_m: 1.0 / (m[i] * arch.bulk.powf(1.5)), r: radii[i] * if i == HEAD { 1.0 } else { arch.bulk.sqrt() }, grounded: false };
        }
        let mut sticks = Vec::new();
        let mut add = |a: usize, b: usize, stiff: f32, kind: u8, scale: f32| {
            let rest_len = (rest[a] - rest[b]).len() * scale;
            sticks.push(Stick { a: a as u8, b: b as u8, rest: rest_len, stiff, kind });
        };
        // torso: shoulders+chest+neck rigid block
        for &(a, b) in &[(LSH, RSH), (LSH, CHEST), (RSH, CHEST), (LSH, NECK), (RSH, NECK), (CHEST, NECK)] {
            add(a, b, 1.0, 0, 1.0);
        }
        for &(a, b) in &[(PELVIS, LHIP), (PELVIS, RHIP), (LHIP, RHIP)] {
            add(a, b, 1.0, 0, 1.0);
        }
        add(PELVIS, SPINE, 1.0, 0, 1.0);
        add(SPINE, CHEST, 1.0, 0, 1.0);
        for &(a, b) in &[(SPINE, LHIP), (SPINE, RHIP), (SPINE, LSH), (SPINE, RSH)] {
            add(a, b, 0.6, 0, 1.0);
        }
        add(PELVIS, CHEST, 0.5, 0, 1.0);
        for &(a, b) in &[(LSH, LHIP), (RSH, RHIP), (LSH, RHIP), (RSH, LHIP)] {
            add(a, b, 0.25, 0, 1.0);
        }
        add(NECK, HEAD, 1.0, 0, 1.0);
        add(CHEST, HEAD, 0.4, 0, 1.0);
        add(LSH, HEAD, 0.6, 1, 0.85);
        add(RSH, HEAD, 0.6, 1, 0.85);
        // limbs
        for &(a, b) in &[(LSH, LEL), (LEL, LHA), (RSH, REL), (REL, RHA), (LHIP, LKN), (LKN, LFT), (RHIP, RKN), (RKN, RFT), (LFT, LTOE), (RFT, RTOE)] {
            add(a, b, 1.0, 0, 1.0);
        }
        add(LKN, LTOE, 0.8, 0, 1.0);
        add(RKN, RTOE, 0.8, 0, 1.0);
        // range limits
        add(LSH, LHA, 0.8, 1, 0.25);
        add(RSH, RHA, 0.8, 1, 0.25);
        add(LHIP, LFT, 0.8, 1, 0.3);
        add(RHIP, RFT, 0.8, 1, 0.3);
        add(HEAD, PELVIS, 0.8, 1, 0.75);

        let mut f = Fighter {
            body,
            sticks,
            targets: [ZERO; NP],
            kmul: [1.0; NP],
            keep: [0.5; NP],
            dims: d,
            arch,
            look,
            root: root.flat(),
            root_vel: ZERO,
            yaw,
            feet: [Foot::default(); 2],
            state: FState::Stand,
            action: Action::Idle,
            strength: 1.0,
            hp_head: 100.0,
            hp_body: 100.0,
            hp_legs: 100.0,
            stamina: 100.0,
            stun: 0.0,
            balance: 1.0,
            special: 0.0,
            bruise: [0.0; 6],
            blood: 0.0,
            combo: 0,
            combo_timer: 0.0,
            buf: [0.0; 4],
            knockdowns: 0,
            time: 0.0,
            grabbed_by_other: false,
            escape: 0.0,
            parry_window: 0.0,
            stats: FightStats::default(),
            rng: Rng::new(seed),
            menu_mode: false,
            shadow_t: 1.5,
            spin: 0.0,
            look_at: ZERO,
            last_hit_dir: ZERO,
            special_throw: false,
        };
        f.reset_feet();
        f
    }

    pub fn reset_feet(&mut self) {
        let fr = Frame::from_yaw(self.root, self.yaw);
        for i in 0..2 {
            let p = fr.p(self.stance_foot(i));
            self.feet[i] = Foot { planted: p, from: p, to: p, t: 1.0, stepping: false };
        }
    }

    /// Teleport the whole body to a standing pose at root/yaw.
    pub fn respawn(&mut self, root: V3, yaw: f32) {
        self.root = root.flat();
        self.yaw = yaw;
        self.root_vel = ZERO;
        let fr = Frame::from_yaw(self.root, yaw);
        let rest = rest_pose(&self.dims);
        for i in 0..NP {
            let p = fr.p(rest[i]);
            self.body[i].p = p;
            self.body[i].prev = p;
            self.body[i].vel = ZERO;
        }
        self.state = FState::Stand;
        self.action = Action::Idle;
        self.strength = 1.0;
        self.reset_feet();
    }

    pub fn stance_foot(&self, i: usize) -> V3 {
        let h = self.dims.h;
        if i == 0 {
            v3(0.2 * h, 0.06, -0.12 * h)
        } else {
            v3(-0.2 * h, 0.06, 0.15 * h)
        }
    }

    pub fn pos(&self, i: usize) -> V3 {
        self.body[i].p
    }
    pub fn com(&self) -> V3 {
        let mut s = ZERO;
        let mut m = 0.0;
        for p in &self.body {
            let mm = 1.0 / p.inv_m;
            s += p.p * mm;
            m += mm;
        }
        s / m
    }
    pub fn facing(&self) -> V3 {
        dir_from_yaw(self.yaw)
    }
    pub fn standing(&self) -> bool {
        matches!(self.state, FState::Stand)
    }
    pub fn is_ko(&self) -> bool {
        matches!(self.state, FState::Ko { .. })
    }
    pub fn is_down(&self) -> bool {
        matches!(self.state, FState::Down { .. } | FState::Ko { .. })
    }
    pub fn rocked(&self) -> bool {
        self.stun > 55.0 && self.standing()
    }
    pub fn blocking(&self) -> bool {
        matches!(self.action, Action::Block)
    }
    pub fn busy(&self) -> bool {
        !matches!(self.action, Action::Idle | Action::Block)
    }

    pub fn view(&self) -> OppView {
        let mut pos = [ZERO; NP];
        for i in 0..NP {
            pos[i] = self.body[i].p;
        }
        OppView { pos, root: self.root, standing: self.standing(), rocked: self.rocked(), blocking: self.blocking() }
    }

    pub fn press(&mut self, b: usize) {
        self.buf[b] = 0.28;
    }

    pub fn knock_down(&mut self, dur: f32) {
        if self.is_ko() {
            return;
        }
        self.state = FState::Down { t: 0.0, dur };
        self.action = Action::Idle;
        self.stats.knockdowns += 0; // counted by attacker
        self.knockdowns += 1;
    }
    pub fn ko(&mut self) {
        self.state = FState::Ko { t: 0.0 };
        self.action = Action::Idle;
        self.strength = 0.0;
    }

    // ---------------------------------------------------------- decision layer
    /// Update state machine & start actions. dt = fixed frame step.
    pub fn think(&mut self, it: &Intent, opp: Option<&OppView>, dt: f32) {
        self.time += dt;
        for b in self.buf.iter_mut() {
            *b = (*b - dt).max(0.0);
        }
        if it.punch {
            self.press(B_PUNCH)
        }
        if it.kick {
            self.press(B_KICK)
        }
        if it.grab {
            self.press(B_GRAB)
        }
        if it.special {
            self.press(B_SPECIAL)
        }
        self.parry_window = (self.parry_window - dt).max(0.0);
        self.combo_timer = (self.combo_timer - dt).max(0.0);
        if self.combo_timer <= 0.0 {
            self.combo = 0;
        }
        // regen
        let body_f = 0.35 + 0.65 * (self.hp_body / 100.0).max(0.0);
        let regen = if self.blocking() { 7.0 } else { 15.0 };
        self.stamina = (self.stamina + regen * body_f * dt).min(100.0);
        self.stun = (self.stun - 6.5 * dt).max(0.0);
        self.balance = (self.balance + 0.45 * dt).min(1.0);

        match self.state {
            FState::Stand => {
                self.strength = approach(self.strength, 1.0, dt * 2.0);
                if self.rocked() {
                    self.strength = 0.6 + 0.25 * (self.time * 7.0).sin();
                }
                self.stand_logic(it, opp, dt);
            }
            FState::Down { t, dur } => {
                let t = t + dt;
                self.strength = 0.0;
                self.root = self.body[PELVIS].p.flat();
                self.state = if t >= dur { FState::GetUp { t: 0.0 } } else { FState::Down { t, dur } };
                if let FState::GetUp { .. } = self.state {
                    self.root = self.body[PELVIS].p.flat();
                    if let Some(o) = opp {
                        self.yaw = yaw_from_dir((o.root - self.root).flat().norm_or(self.facing()));
                    }
                }
            }
            FState::GetUp { t } => {
                let t = t + dt;
                self.strength = smooth(t / 0.5) * 1.0;
                if t >= 1.5 {
                    self.state = FState::Stand;
                    self.balance = 1.0;
                    self.stun = self.stun.min(30.0);
                    self.reset_feet_from_body();
                } else {
                    self.state = FState::GetUp { t };
                }
            }
            FState::Ko { t } => {
                self.strength = 0.0;
                self.state = FState::Ko { t: t + dt };
            }
            FState::Celebrate { kind, t } => {
                self.strength = approach(self.strength, 1.6, dt * 3.0);
                self.state = FState::Celebrate { kind, t: t + dt };
            }
        }
    }

    fn reset_feet_from_body(&mut self) {
        let lf = self.body[LFT].p;
        let rf = self.body[RFT].p;
        self.feet[0] = Foot { planted: lf.flat() + v3(0.0, 0.06, 0.0), from: lf, to: lf, t: 1.0, stepping: false };
        self.feet[1] = Foot { planted: rf.flat() + v3(0.0, 0.06, 0.0), from: rf, to: rf, t: 1.0, stepping: false };
    }

    fn start_move(&mut self, m: usize, opp: Option<&OppView>, finisher: bool) {
        let d = &MOVES[m];
        if !d.special && self.stamina < d.stamina * 0.5 {
            return; // too gassed
        }
        let aim = self.aim_point(d.aim, opp);
        let gas = if self.stamina < 20.0 { 0.75 } else { 1.0 };
        let mut power = d.power * gas * self.arch.power;
        if d.kick {
            power *= self.arch.kick;
        }
        self.stamina = (self.stamina - d.stamina).max(0.0);
        self.action = Action::Strike { mv: m, t: 0.0, hit: false, aim, finisher, power };
        self.stats.thrown += 1;
        if d.kick {
            self.stats.kicks += 0;
        }
    }

    fn aim_point(&self, aim: Aim, opp: Option<&OppView>) -> V3 {
        match opp {
            Some(o) => match aim {
                Aim::Head => o.pos[HEAD],
                Aim::Body => o.pos[SPINE].lerp(o.pos[CHEST], 0.4),
                Aim::Legs => {
                    // their lead leg thigh
                    let a = o.pos[LKN].lerp(o.pos[LHIP], 0.35);
                    let b = o.pos[RKN].lerp(o.pos[RHIP], 0.35);
                    if (a - self.root).len2() < (b - self.root).len2() {
                        a
                    } else {
                        b
                    }
                }
            },
            None => {
                let fr = Frame::from_yaw(self.root, self.yaw);
                let h = self.dims.h;
                match aim {
                    Aim::Head => fr.p(v3(0.95, 1.62 * h, 0.0)),
                    Aim::Body => fr.p(v3(0.95, 1.2 * h, 0.0)),
                    Aim::Legs => fr.p(v3(0.9, 0.55 * h, 0.0)),
                }
            }
        }
    }

    fn stand_logic(&mut self, it: &Intent, opp: Option<&OppView>, dt: f32) {
        // face opponent
        let spd_mul = self.arch.speed * (0.55 + 0.45 * (self.hp_legs / 100.0).max(0.0));
        if let Some(o) = opp {
            let to = (o.pos[PELVIS] - self.root).flat();
            if to.len() > 0.05 && !matches!(self.action, Action::Throw { .. }) {
                let want = yaw_from_dir(to);
                let diff = wrap_angle(want - self.yaw);
                let rate = if matches!(self.action, Action::Strike { .. }) { 4.0 } else { 9.0 };
                self.yaw += clampf(diff, -rate * dt, rate * dt);
            }
        }
        self.yaw = wrap_angle(self.yaw);
        // timers inside actions
        let dist = opp.map(|o| (o.pos[PELVIS] - self.body[PELVIS].p).flat().len()).unwrap_or(1.2);
        match self.action {
            Action::Strike { mv, t, hit, aim, finisher, power } => {
                let d = &MOVES[mv];
                let speed = if d.special { 1.0 } else { self.arch.speed.sqrt() * if self.stamina < 15.0 { 0.75 } else { 1.0 } };
                let t2 = t + dt * speed;
                // limited homing during windup
                let mut aim2 = aim;
                if t2 < d.windup {
                    let live = self.aim_point(d.aim, opp);
                    aim2 = aim.lerp(live, 0.12);
                }
                let total = d.windup + d.active + d.recover;
                // combo cancel: punches can chain during recovery
                if !d.kick && !d.special && t2 > d.windup + d.active * 0.8 && self.buf[B_PUNCH] > 0.0 {
                    self.buf[B_PUNCH] = 0.0;
                    let next = self.next_punch(it);
                    self.start_move(next, opp, false);
                    return;
                }
                if t2 >= total {
                    self.action = Action::Idle;
                    if d.kick {
                        // re-plant the kicking foot
                        let fi = if d.end == LFT || d.end == LKN { 0 } else { 1 };
                        let p = self.body[if fi == 0 { LFT } else { RFT }].p;
                        self.feet[fi] = Foot { planted: p, from: p, to: p, t: 1.0, stepping: false };
                    }
                } else {
                    self.action = Action::Strike { mv, t: t2, hit, aim: aim2, finisher, power };
                }
                // lunges
                let f = self.facing();
                match d.style {
                    Style::Superman | Style::FlyKnee if t2 < d.windup + d.active * 0.5 => {
                        self.root_vel = f * (3.2 * smooth(t2 / d.windup));
                    }
                    Style::Haymaker if t2 > d.windup && t2 < d.windup + d.active => {
                        self.root_vel = f * 1.5;
                    }
                    _ => {}
                }
            }
            Action::Hurt { t } => {
                let t = t - dt;
                self.action = if t <= 0.0 { Action::Idle } else { Action::Hurt { t } };
            }
            Action::Clinch { t, attached } => {
                let t = t + dt;
                if (!attached && t > 0.45) || t > 3.2 || dist > 1.3 {
                    self.action = Action::Idle;
                } else {
                    self.action = Action::Clinch { t, attached };
                    if attached && t > 0.15 {
                        if self.buf[B_KICK] > 0.0 {
                            self.buf[B_KICK] = 0.0;
                            self.start_move(M_KNEE, opp, false);
                            // keep the clinch after the knee
                            if let Action::Strike { .. } = self.action {
                                self.stats.kicks += 0;
                            }
                        } else if self.buf[B_GRAB] > 0.0 {
                            self.buf[B_GRAB] = 0.0;
                            self.action = Action::Throw { t: 0.0, thrown: false };
                        } else if self.buf[B_PUNCH] > 0.0 {
                            self.buf[B_PUNCH] = 0.0;
                            self.start_move(M_UPPER, opp, false);
                        }
                    }
                }
            }
            Action::Throw { t, thrown } => {
                let t = t + dt;
                self.action = if t > 0.9 { Action::Idle } else { Action::Throw { t, thrown } };
            }
            Action::Idle | Action::Block => {
                self.action = if it.block && self.stamina > 2.0 { Action::Block } else { Action::Idle };
                if it.block {
                    // fresh block press opens a parry window
                    if !self.blocking() {
                        self.parry_window = 0.15;
                    }
                }
                let opp_rocked = opp.map(|o| o.rocked || !o.standing).unwrap_or(false);
                if self.buf[B_SPECIAL] > 0.0 && self.special >= 100.0 {
                    self.buf[B_SPECIAL] = 0.0;
                    self.special = 0.0;
                    let fin = opp_rocked;
                    match self.arch.finisher {
                        0 => self.start_move(M_SUPERMAN, opp, fin),
                        1 => self.start_move(M_SPIN, opp, fin),
                        2 => self.start_move(M_HAYMAKER, opp, fin),
                        3 => {
                            if dist < 1.2 {
                                self.action = Action::Throw { t: 0.0, thrown: false };
                                self.special_throw = true;
                            } else {
                                self.start_move(M_SUPERMAN, opp, fin);
                            }
                        }
                        _ => self.start_move(M_FLYKNEE, opp, fin),
                    }
                } else if self.buf[B_PUNCH] > 0.0 {
                    self.buf[B_PUNCH] = 0.0;
                    let m = self.next_punch(it);
                    self.start_move(m, opp, false);
                } else if self.buf[B_KICK] > 0.0 {
                    self.buf[B_KICK] = 0.0;
                    let m = if it.down {
                        M_LOW
                    } else if it.fwd {
                        M_TEEP
                    } else if it.back {
                        M_HIGH
                    } else if dist < 0.75 {
                        M_KNEE
                    } else {
                        M_ROUND
                    };
                    self.start_move(m, opp, false);
                } else if self.buf[B_GRAB] > 0.0 {
                    self.buf[B_GRAB] = 0.0;
                    if self.stamina > 8.0 {
                        self.stamina -= 6.0;
                        self.action = Action::Clinch { t: 0.0, attached: false };
                    }
                }
            }
        }

        // locomotion
        let can_move = matches!(self.action, Action::Idle | Action::Block | Action::Clinch { .. });
        let mut want = if can_move { it.mv.clamp_len(1.0) * (2.3 * spd_mul) } else { ZERO };
        if self.blocking() {
            want *= 0.55;
        }
        if let Some(o) = opp {
            // don't walk into the opponent
            let to = (o.root - self.root).flat();
            let d = to.len();
            if d < 0.62 {
                let n = to.norm();
                let into = want.dot(n);
                if into > 0.0 {
                    want -= n * into;
                }
            }
        }
        let is_lunge = matches!(self.action, Action::Strike { mv, .. } if MOVES[mv].special);
        if !is_lunge {
            let k = (12.0 * dt).min(1.0);
            self.root_vel = self.root_vel.lerp(want, k);
        } else {
            self.root_vel *= 0.97;
        }
        if self.rocked() {
            // drunk wobble
            let wob = v3((self.time * 2.3).sin(), 0.0, (self.time * 1.7).cos()) * 0.6;
            self.root_vel += wob * dt * 4.0;
        }
        self.root += self.root_vel * dt;
        // follow the real pelvis so knockback persists
        let pf = self.body[PELVIS].p.flat();
        self.root = self.root.lerp(pf, (2.5 * dt).min(1.0));
        self.update_feet(dt, spd_mul);
    }

    fn next_punch(&mut self, it: &Intent) -> usize {
        let m = if it.down {
            M_UPPER
        } else if it.back {
            M_HOOK
        } else if it.fwd && self.combo == 0 {
            M_CROSS
        } else {
            PUNCH_CHAIN[self.combo % 4]
        };
        self.combo += 1;
        self.combo_timer = 0.7;
        m
    }

    fn update_feet(&mut self, dt: f32, spd: f32) {
        let fr = Frame::from_yaw(self.root, self.yaw);
        let lead = self.root_vel * 0.14;
        let ideal = [fr.p(self.stance_foot(0)) + lead, fr.p(self.stance_foot(1)) + lead];
        let any_step = self.feet[0].stepping || self.feet[1].stepping;
        if !any_step {
            let e0 = (ideal[0] - self.feet[0].planted).flat().len();
            let e1 = (ideal[1] - self.feet[1].planted).flat().len();
            let (i, e) = if e0 > e1 { (0, e0) } else { (1, e1) };
            if e > 0.09 {
                let f = &mut self.feet[i];
                f.stepping = true;
                f.from = f.planted;
                f.t = 0.0;
            }
        }
        let dur = 0.13 / spd.max(0.5);
        for i in 0..2 {
            let f = &mut self.feet[i];
            if f.stepping {
                f.to = ideal[i];
                f.t += dt / dur;
                if f.t >= 1.0 {
                    f.t = 1.0;
                    f.stepping = false;
                    f.planted = f.to;
                }
            }
        }
    }

    fn foot_target(&self, i: usize) -> V3 {
        let f = &self.feet[i];
        if f.stepping {
            let s = smooth(f.t);
            f.from.lerp(f.to, s) + UP * ((f.t * PI).sin() * 0.09)
        } else {
            f.planted
        }
    }

    // ---------------------------------------------------------- pose layer
    pub fn compute_targets(&mut self, opp: Option<&OppView>, cam: V3) {
        let d = self.dims;
        let h = d.h;
        let t = self.time;
        self.kmul = [1.0; NP];
        self.keep = [0.5; NP];
        let mut yaw = self.yaw + self.spin;
        let mut origin = self.root;
        let mut pelvis_h = d.stand_pelvis - 0.07 * h + (t * 2.0 * PI * 1.6).sin() * 0.012;
        let mut lean = 0.14;
        let mut twist = -0.35;
        let mut hip_twist = -0.15;
        let mut side = 0.0;
        let mut head_tilt = 0.12;
        let mut flip: Option<(V3, f32)> = None; // (axis(local r), angle)
        let mut lift = 0.0;
        if self.rocked() {
            side = 0.12 * (t * 3.1).sin();
            lean = 0.05;
            head_tilt = -0.2 + 0.25 * (t * 2.0).sin();
        }
        let mut feet = [self.foot_target(0), self.foot_target(1)];
        let mut knee_out = [0.25, 0.25];
        let mut toe_dirs: [Option<V3>; 2] = [None, None];
        let opp_head = opp.map(|o| o.pos[HEAD]);

        // per-state base adjustments (before torso)
        enum HandMode {
            Guard,
            Block,
            Custom([V3; 2], [V3; 2]),
        }
        let mut hand_mode = HandMode::Guard;
        let mut knee_override: [Option<V3>; 2] = [None, None];

        let fr0 = Frame::from_yaw(origin, yaw);
        match self.state {
            FState::Stand => {}
            FState::GetUp { t: gt } => {
                let k = smooth((gt - 0.35) / 1.0);
                pelvis_h = lerpf(0.5 * h, d.stand_pelvis - 0.07 * h, k);
                lean = lerpf(0.7, 0.14, k);
                let kneel = [fr0.p(v3(0.35 * h, 0.06, -0.12 * h)), fr0.p(v3(-0.45 * h, 0.06, 0.14 * h))];
                for i in 0..2 {
                    let stand = fr0.p(self.stance_foot(i));
                    feet[i] = kneel[i].lerp(stand, k);
                }
                hand_mode = HandMode::Custom(
                    [fr0.p(v3(0.35, 0.5 * h + 0.6 * k, -0.25)), fr0.p(v3(0.3, 0.5 * h + 0.6 * k, 0.25))],
                    [v3(0.0, -1.0, 0.0), v3(0.0, -1.0, 0.0)],
                );
            }
            FState::Celebrate { kind, t: ct } => {
                // face the camera
                let to_cam = (cam - self.root).flat();
                yaw = yaw_from_dir(to_cam.norm_or(self.facing()));
                self.yaw = yaw;
                twist = 0.0;
                hip_twist = 0.0;
                lean = 0.0;
                head_tilt = -0.1;
                let fr = Frame::from_yaw(origin, yaw);
                let stand_feet = [fr.p(v3(0.0, 0.06, -0.15 * h)), fr.p(v3(0.0, 0.06, 0.15 * h))];
                feet = stand_feet;
                pelvis_h = d.stand_pelvis - 0.02;
                let c = celebrate_pose(kind, ct, h, &fr, &mut pelvis_h, &mut lean, &mut twist, &mut side, &mut head_tilt, &mut feet, &mut lift, &mut flip, &mut origin, &mut yaw);
                hand_mode = HandMode::Custom(c.0, c.1);
                for i in 0..NP {
                    self.kmul[i] = 1.6;
                }
            }
            _ => {}
        }

        // actions
        let mut strike_info: Option<(usize, f32, f32, f32, V3)> = None; // (mv, windup prog, ext, recover, aim)
        if matches!(self.state, FState::Stand) {
            match self.action {
                Action::Block => {
                    hand_mode = HandMode::Block;
                    lean = 0.25;
                    head_tilt = 0.35;
                    pelvis_h -= 0.04;
                }
                Action::Hurt { .. } => {
                    head_tilt = -0.45;
                    lean = -0.12;
                }
                Action::Strike { mv, t: st, aim, .. } => {
                    let dm = &MOVES[mv];
                    let w = clampf(st / dm.windup, 0.0, 1.0);
                    let a = clampf((st - dm.windup) / dm.active, 0.0, 1.0);
                    let r = clampf((st - dm.windup - dm.active) / dm.recover, 0.0, 1.0);
                    let ext = if st < dm.windup { -0.15 * w } else if r <= 0.0 { smooth(a * 1.6) } else { 1.0 - smooth(r) };
                    strike_info = Some((mv, w, ext, r, aim));
                    match dm.style {
                        Style::Jab => {
                            twist = -0.35 - 0.3 * ext.max(0.0);
                            lean = 0.18 + 0.1 * ext.max(0.0);
                        }
                        Style::Cross => {
                            twist = lerpf(-0.35, 0.55, ext.max(0.0));
                            hip_twist = lerpf(-0.15, 0.35, ext.max(0.0));
                            lean = 0.2 + 0.1 * ext.max(0.0);
                        }
                        Style::Hook => {
                            twist = if st < dm.windup { -0.35 + 0.35 * w } else { lerpf(0.0, -0.85, ext.max(0.0)) };
                            hip_twist = twist * 0.5;
                        }
                        Style::Upper => {
                            pelvis_h -= 0.08 * w * (1.0 - ext.max(0.0)) - 0.02 * ext.max(0.0);
                            twist = lerpf(-0.2, 0.45, ext.max(0.0));
                            lean = lerpf(0.25, 0.0, ext.max(0.0));
                        }
                        Style::Round | Style::HighKick | Style::LowKick => {
                            let e = ext.max(0.0);
                            hip_twist = -0.2 - 1.1 * (w * 0.4 + e * 0.6);
                            twist = hip_twist * 0.6;
                            lean = -0.1 - if dm.style == Style::HighKick { 0.35 } else { 0.2 } * e;
                            side = -0.25 * e;
                        }
                        Style::Teep => {
                            lean = -0.25 * ext.max(0.0) - 0.05;
                            twist = -0.2;
                        }
                        Style::Knee => {
                            lean = 0.25 - 0.4 * ext.max(0.0);
                            twist = 0.1;
                            hip_twist = 0.25 * ext.max(0.0);
                        }
                        Style::Superman => {
                            lift = 0.28 * (if st < dm.windup { smooth(w) } else { 1.0 - a }).max(0.0);
                            twist = lerpf(-0.6, 0.75, ext.max(0.0));
                            lean = 0.35;
                        }
                        Style::SpinKick => {
                            let sp = if st < dm.windup { smooth(w) * PI * 0.9 } else if r <= 0.0 { PI * 0.9 + a * PI * 0.6 } else { PI * 1.5 + smooth(r) * PI * 0.5 };
                            yaw = self.yaw + sp;
                            lean = -0.3 * ext.max(0.0);
                            side = 0.2 * ext.max(0.0);
                        }
                        Style::Haymaker => {
                            twist = if st < dm.windup { lerpf(-0.35, -1.0, smooth(w)) } else { lerpf(-1.0, 1.0, ext.max(0.0)) };
                            hip_twist = twist * 0.5;
                            lean = 0.3;
                        }
                        Style::FlyKnee => {
                            lift = if st < dm.windup { -0.1 * w } else if r <= 0.0 { 0.45 * (a * PI * 0.5 + 0.5).sin() } else { 0.4 * (1.0 - r) };
                            lean = 0.1;
                        }
                    }
                }
                Action::Clinch { .. } => {
                    lean = 0.35;
                    head_tilt = 0.3;
                }
                Action::Throw { t: tt, .. } => {
                    let k = clampf(tt / 0.3, 0.0, 1.0);
                    pelvis_h -= 0.18 * k * (1.0 - clampf((tt - 0.3) / 0.4, 0.0, 1.0));
                    twist = lerpf(0.0, 2.4, smooth((tt - 0.2) / 0.5));
                    hip_twist = twist * 0.6;
                    lean = lerpf(0.3, -0.5, smooth((tt - 0.2) / 0.4));
                }
                Action::Idle => {
                    if self.menu_mode {
                        // shadow boxing in the menu
                        self.shadow_t -= 1.0 / 60.0;
                    }
                }
            }
        }

        // -------- torso frames
        let base = Frame::from_yaw(origin, yaw);
        let pelvis = base.p(v3(0.0, pelvis_h + lift, 0.0));
        let hipf = Frame::from_yaw(pelvis, yaw + hip_twist);
        let mut chest_f = Frame::from_yaw(pelvis, yaw + twist);
        chest_f = chest_f.rotated(chest_f.r, -lean);
        chest_f = chest_f.rotated(chest_f.f, side);
        let mid_up = (UP + chest_f.u).norm();
        let spine = pelvis + mid_up * d.ps;
        let chest = spine + chest_f.u * d.sc;
        let neck = chest + chest_f.u * d.cn;
        let mut head_up = chest_f.u.rot_axis(chest_f.r, -head_tilt);
        if let Some(oh) = opp_head {
            if matches!(self.state, FState::Stand) {
                let to = (oh - neck).norm_or(chest_f.f);
                head_up = (head_up + to * 0.15).norm();
            }
        }
        let head = neck + head_up * d.nh;
        let lsh = neck - chest_f.u * (0.045 * h) - chest_f.r * d.sh_w;
        let rsh = neck - chest_f.u * (0.045 * h) + chest_f.r * d.sh_w;
        let lhip = pelvis - hipf.u * (0.03 * h) - hipf.r * d.hip_w;
        let rhip = pelvis - hipf.u * (0.03 * h) + hipf.r * d.hip_w;
        let cf = Frame { o: chest, ..chest_f };

        // -------- hands
        let guard_l = cf.p(v3(0.3 * h, 0.15 * h, -0.1 * h));
        let guard_r = cf.p(v3(0.18 * h, 0.17 * h, 0.13 * h));
        let pole_l = cf.d(v3(-0.2, -1.0, -0.6));
        let pole_r = cf.d(v3(-0.2, -1.0, 0.6));
        let (mut hands, mut epoles) = match hand_mode {
            HandMode::Guard => ([guard_l, guard_r], [pole_l, pole_r]),
            HandMode::Block => (
                [cf.p(v3(0.2 * h, 0.33 * h, -0.07 * h)), cf.p(v3(0.19 * h, 0.34 * h, 0.07 * h))],
                [cf.d(v3(0.3, -1.0, -0.15)), cf.d(v3(0.3, -1.0, 0.15))],
            ),
            HandMode::Custom(hh, pp) => (hh, pp),
        };
        if self.rocked() {
            hands[0] = hands[0] - UP * 0.15;
            hands[1] = hands[1] - UP * 0.15;
        }
        if let Action::Hurt { .. } = self.action {
            hands[0] = hands[0] - UP * 0.1 + cf.r * -0.08;
            hands[1] = hands[1] - UP * 0.1 + cf.r * 0.08;
        }
        if let (FState::Stand, Action::Clinch { .. }) = (self.state, self.action) {
            if let Some(o) = opp {
                let n = o.pos[NECK];
                let r = (o.pos[RSH] - o.pos[LSH]).norm_or(cf.r);
                hands = [n - r * 0.08, n + r * 0.08];
                epoles = [cf.d(v3(0.0, -1.0, -0.5)), cf.d(v3(0.0, -1.0, 0.5))];
            }
        }
        if let (FState::Stand, Action::Throw { .. }) = (self.state, self.action) {
            hands = [cf.p(v3(0.35, 0.05, -0.12)), cf.p(v3(0.35, 0.05, 0.12))];
        }

        let mut strike_end: Option<usize> = None;
        if let Some((mvi, w, ext, _r, aim)) = strike_info {
            let dm = &MOVES[mvi];
            strike_end = Some(dm.end);
            let e = ext.max(0.0);
            let shoulder = |i: usize| if i == LHA { lsh } else { rsh };
            let over = |from: V3, to: V3, amt: f32| to + (to - from).norm() * amt;
            match dm.style {
                Style::Jab | Style::Cross | Style::Superman => {
                    let hi = if dm.end == LHA { 0 } else { 1 };
                    let s = shoulder(dm.end);
                    let tgt = over(s, aim, 0.22);
                    hands[hi] = if ext < 0.0 { hands[hi] + (hands[hi] - tgt).norm() * (-ext * 0.3) } else { hands[hi].lerp(tgt, e) };
                    epoles[hi] = cf.d(v3(0.0, -0.6, if hi == 0 { -1.0 } else { 1.0 }));
                }
                Style::Hook => {
                    let tgt = over(lsh, aim, 0.12) - cf.r * (0.45 * (1.0 - e));
                    hands[0] = if ext < 0.0 { hands[0] - cf.r * (0.15 * -ext * 6.0) } else { hands[0].lerp(tgt, e) };
                    epoles[0] = cf.d(v3(0.0, 0.5, -1.0));
                }
                Style::Upper => {
                    let low = cf.p(v3(0.15 * h, -0.08 * h, 0.12 * h));
                    let tgt = aim + UP * 0.12;
                    hands[1] = if ext < 0.0 { hands[1].lerp(low, w) } else { low.lerp(tgt, e) };
                    epoles[1] = cf.d(v3(0.4, -1.0, 0.3));
                }
                Style::Haymaker => {
                    let back = cf.p(v3(-0.3 * h, 0.15 * h, 0.4 * h));
                    let tgt = over(rsh, aim, 0.15) + cf.r * (0.5 * (1.0 - e));
                    hands[1] = if ext < 0.0 { hands[1].lerp(back, w) } else { back.lerp(tgt, e) };
                    epoles[1] = cf.d(v3(-0.2, 0.3, 1.0));
                }
                Style::Round | Style::HighKick | Style::LowKick => {
                    let hipp = rhip;
                    let chamber = hipf.p(v3(-0.05, -0.15 * h, 0.45 * h)) + UP * 0.0;
                    let side_v = Frame::from_yaw(origin, self.yaw).r;
                    let tgt = over(hipp, aim, 0.2) + side_v * (0.55 * (1.0 - e)) - UP * (0.2 * (1.0 - e));
                    let fp = if ext < 0.0 { feet[1].lerp(chamber, smooth(w)) } else { chamber.lerp(tgt, e) };
                    feet[1] = fp;
                    knee_out[1] = 1.0;
                    toe_dirs[1] = Some((fp - hipp).norm());
                    hands[1] = cf.p(v3(-0.25 * h, -0.35 * h, 0.35 * h)).lerp(hands[1], 1.0 - e);
                }
                Style::Teep => {
                    let chamber = hipf.p(v3(0.3 * h, -0.25 * h, -0.12 * h));
                    let tgt = over(lhip, aim, 0.18);
                    let fp = if ext < 0.0 { feet[0].lerp(chamber, smooth(w)) } else { chamber.lerp(tgt, e) };
                    feet[0] = fp;
                    toe_dirs[0] = Some(UP.lerp(hipf.f, 0.3).norm());
                }
                Style::Knee | Style::FlyKnee => {
                    let tgt = aim;
                    let kn = if ext < 0.0 { rhip + hipf.d(v3(0.1, -0.3, 0.0)) } else { (rhip + hipf.d(v3(0.0, -0.35, 0.05))).lerp(tgt, e) };
                    knee_override[1] = Some(kn);
                    feet[1] = kn + hipf.d(v3(-0.3, -0.32, 0.0));
                    toe_dirs[1] = Some(-UP);
                    if dm.style == Style::FlyKnee {
                        feet[0] = feet[0].lerp(lhip + hipf.d(v3(-0.35, -0.6, 0.0)), e);
                        hands = [cf.p(v3(0.2, 0.45, -0.25)), cf.p(v3(0.2, 0.45, 0.25))];
                    }
                    if let Some(o) = opp {
                        if let Action::Clinch { .. } | Action::Strike { .. } = self.action {
                            let _ = o;
                        }
                    }
                }
                Style::SpinKick => {
                    let tgt = over(lhip, aim, 0.15);
                    let chamber = hipf.p(v3(0.0, -0.2 * h, -0.4 * h));
                    let fp = if ext < 0.0 || w < 1.0 && _r <= 0.0 && e <= 0.0 { feet[0].lerp(chamber, smooth(w)) } else { chamber.lerp(tgt, e) };
                    feet[0] = fp;
                    toe_dirs[0] = Some((fp - lhip).norm());
                    knee_out[0] = 1.0;
                }
            }
        }

        // -------- write targets with IK
        let mut tg = [ZERO; NP];
        tg[PELVIS] = pelvis;
        tg[SPINE] = spine;
        tg[CHEST] = chest;
        tg[NECK] = neck;
        tg[HEAD] = head;
        tg[LSH] = lsh;
        tg[RSH] = rsh;
        tg[LHIP] = lhip;
        tg[RHIP] = rhip;
        let (le, lh) = ik2(lsh, hands[0], d.ua, d.fa, epoles[0]);
        let (re, rh) = ik2(rsh, hands[1], d.ua, d.fa, epoles[1]);
        tg[LEL] = le;
        tg[LHA] = lh;
        tg[REL] = re;
        tg[RHA] = rh;
        let kp = |out: f32, s: f32| hipf.d(v3(1.0, 0.0, s * out)).norm();
        let (lk, lf) = ik2(lhip, feet[0], d.thigh, d.shin, kp(knee_out[0], -0.35));
        let (rk, rf) = ik2(rhip, feet[1], d.thigh, d.shin, kp(knee_out[1], 0.35));
        tg[LKN] = knee_override[0].unwrap_or(lk);
        tg[LFT] = lf;
        tg[RKN] = knee_override[1].unwrap_or(rk);
        tg[RFT] = rf;
        let tdir = |i: usize, s: f32| toe_dirs[i].unwrap_or(hipf.f.rot_y(s * 0.35));
        tg[LTOE] = lf + tdir(0, 1.0) * d.foot - UP * 0.02;
        tg[RTOE] = rf + tdir(1, -1.0) * d.foot - UP * 0.02;

        // whole-body rotation (flips)
        if let Some((axis, ang)) = flip {
            let pivot = tg[PELVIS].lerp(tg[CHEST], 0.3);
            for p in tg.iter_mut() {
                *p = pivot + (*p - pivot).rot_axis(axis, ang);
            }
        }
        for p in tg.iter_mut() {
            p.y = p.y.max(0.02);
        }
        self.targets = tg;
        self.look_at = opp_head.unwrap_or(head + base.f);

        // muscle emphasis
        if let Some(end) = strike_end {
            let limb: &[usize] = match end {
                LHA => &[LEL, LHA],
                RHA => &[REL, RHA],
                LFT => &[LKN, LFT, LTOE],
                RFT => &[RKN, RFT, RTOE],
                RKN => &[RKN, RFT, RTOE],
                _ => &[LKN, LFT, LTOE],
            };
            for &i in limb {
                self.kmul[i] = 2.6;
                self.keep[i] = 0.12;
            }
        }
        // planted feet grip
        for (i, &fi) in [LFT, RFT].iter().enumerate() {
            if !self.feet[i].stepping && strike_end.map(|e| e != fi && !(fi == RFT && e == RKN)).unwrap_or(true) {
                self.kmul[fi] *= 1.4;
                self.keep[fi] = 0.9;
            }
        }
        if self.grabbed_by_other {
            for k in self.kmul.iter_mut() {
                *k *= 0.6;
            }
        }
    }

    /// Muscles: pull particles toward targets. Called every substep.
    pub fn apply_muscles(&mut self) {
        if self.strength <= 0.001 {
            return;
        }
        const BASE: [f32; NP] = [0.14, 0.13, 0.14, 0.12, 0.12, 0.13, 0.1, 0.14, 0.13, 0.1, 0.14, 0.14, 0.16, 0.32, 0.14, 0.16, 0.32, 0.25, 0.25];
        for i in 0..NP {
            let k = clampf(BASE[i] * self.kmul[i] * self.strength, 0.0, 0.85);
            let p = &mut self.body[i];
            let d = (self.targets[i] - p.p) * k;
            p.p += d;
            p.prev += d * self.keep[i];
        }
    }

    pub fn integrate(&mut self, dt: f32, damping: f32) {
        let g = v3(0.0, -9.81, 0.0) * (dt * dt);
        for p in self.body.iter_mut() {
            let v = (p.p - p.prev) * damping;
            p.vel = v / dt;
            p.prev = p.p;
            p.p += v + g;
        }
    }

    pub fn solve_sticks(&mut self) {
        for s in &self.sticks {
            let (a, b) = (s.a as usize, s.b as usize);
            let pa = self.body[a].p;
            let pb = self.body[b].p;
            let dv = pb - pa;
            let l = dv.len();
            if l < 1e-6 {
                continue;
            }
            let diff = match s.kind {
                1 if l >= s.rest => continue,
                2 if l <= s.rest => continue,
                _ => (l - s.rest) / l,
            };
            let wa = self.body[a].inv_m;
            let wb = self.body[b].inv_m;
            let w = wa + wb;
            let corr = dv * (diff * s.stiff / w);
            self.body[a].p += corr * wa;
            self.body[b].p -= corr * wb;
        }
        // knees bend forward only (hinge-ish)
        let right = (self.body[RHIP].p - self.body[LHIP].p).norm();
        let up = (self.body[CHEST].p - self.body[PELVIS].p).norm();
        let fwd = up.cross(right).norm();
        for &(hip, kn, ft) in &[(LHIP, LKN, LFT), (RHIP, RKN, RFT)] {
            let mid = (self.body[hip].p + self.body[ft].p) * 0.5;
            let off = (self.body[kn].p - mid).dot(fwd);
            if off < -0.01 {
                self.body[kn].p += fwd * (-off * 0.5);
            }
        }
    }

    /// Ground collision + friction. Returns strongest impact speed of a heavy part.
    pub fn ground(&mut self) -> (f32, V3) {
        let mut best = (0.0, ZERO);
        for (i, p) in self.body.iter_mut().enumerate() {
            let floor = p.r;
            p.grounded = false;
            if p.p.y < floor {
                let vy = (p.p.y - p.prev.y).min(0.0);
                if matches!(i, PELVIS | SPINE | CHEST | HEAD) && -p.vel.y > best.0 {
                    best = (-p.vel.y, p.p);
                }
                p.p.y = floor;
                p.prev.y = floor + vy * 0.15; // tiny bounce
                // friction
                let fr = if i == LFT || i == RFT || i == LTOE || i == RTOE { 0.5 } else { 0.18 };
                let dx = p.p.x - p.prev.x;
                let dz = p.p.z - p.prev.z;
                p.prev.x += dx * fr;
                p.prev.z += dz * fr;
                p.grounded = true;
            }
        }
        best
    }

    pub fn add_impulse(&mut self, i: usize, dv: V3, dt: f32) {
        self.body[i].prev -= dv * dt;
    }

    pub fn is_finite(&self) -> bool {
        self.body.iter().all(|p| p.p.is_finite() && p.prev.is_finite())
    }
}

pub fn rest_pose(d: &Dims) -> [V3; NP] {
    let h = d.h;
    let foot_y = 0.06;
    let knee_y = foot_y + d.shin;
    let hip_y = knee_y + d.thigh;
    let pel = hip_y + 0.03 * h;
    let spine = pel + d.ps;
    let chest = spine + d.sc;
    let neck = chest + d.cn;
    let head = neck + d.nh;
    let sh = neck - 0.045 * h;
    let mut p = [ZERO; NP];
    p[PELVIS] = v3(0.0, pel, 0.0);
    p[SPINE] = v3(0.0, spine, 0.0);
    p[CHEST] = v3(0.0, chest, 0.0);
    p[NECK] = v3(0.0, neck, 0.0);
    p[HEAD] = v3(0.0, head, 0.0);
    p[LSH] = v3(0.0, sh, -d.sh_w);
    p[RSH] = v3(0.0, sh, d.sh_w);
    p[LEL] = v3(0.0, sh - d.ua, -d.sh_w - 0.03);
    p[REL] = v3(0.0, sh - d.ua, d.sh_w + 0.03);
    p[LHA] = v3(0.0, sh - d.ua - d.fa, -d.sh_w - 0.05);
    p[RHA] = v3(0.0, sh - d.ua - d.fa, d.sh_w + 0.05);
    p[LHIP] = v3(0.0, hip_y, -d.hip_w);
    p[RHIP] = v3(0.0, hip_y, d.hip_w);
    p[LKN] = v3(0.01, knee_y, -d.hip_w);
    p[RKN] = v3(0.01, knee_y, d.hip_w);
    p[LFT] = v3(0.0, foot_y, -d.hip_w);
    p[RFT] = v3(0.0, foot_y, d.hip_w);
    p[LTOE] = v3(d.foot, 0.04, -d.hip_w);
    p[RTOE] = v3(d.foot, 0.04, d.hip_w);
    p
}

/// Returns (hands, elbow poles). Mutates pose params.
#[allow(clippy::too_many_arguments)]
fn celebrate_pose(
    kind: u32,
    t: f32,
    h: f32,
    fr: &Frame,
    pelvis_h: &mut f32,
    lean: &mut f32,
    twist: &mut f32,
    side: &mut f32,
    head_tilt: &mut f32,
    feet: &mut [V3; 2],
    lift: &mut f32,
    flip: &mut Option<(V3, f32)>,
    origin: &mut V3,
    yaw: &mut f32,
) -> ([V3; 2], [V3; 2]) {
    let ph = *pelvis_h;
    let up_hands = [fr.p(v3(0.0, 2.05 * h, -0.3)), fr.p(v3(0.0, 2.05 * h, 0.3))];
    let down_poles = [fr.d(v3(0.0, -1.0, -0.5)), fr.d(v3(0.0, -1.0, 0.5))];
    match kind {
        // Flex: double biceps
        0 => {
            let pulse = (t * 5.0).sin() * 0.03;
            *pelvis_h = ph - 0.06 + pulse;
            *head_tilt = -0.25;
            feet[0] = fr.p(v3(0.0, 0.06, -0.24 * h));
            feet[1] = fr.p(v3(0.0, 0.06, 0.24 * h));
            ([fr.p(v3(0.05, 1.62 * h, -0.33 * h)), fr.p(v3(0.05, 1.62 * h, 0.33 * h))], [fr.d(v3(-0.3, 0.2, -1.0)), fr.d(v3(-0.3, 0.2, 1.0))])
        }
        // Backflip
        1 => {
            let c = t % 2.4;
            if c < 0.35 {
                let k = smooth(c / 0.35);
                *pelvis_h = ph - 0.3 * k;
                *lean = 0.4 * k;
                ([fr.p(v3(-0.35, 1.0 * h, -0.25)), fr.p(v3(-0.35, 1.0 * h, 0.25))], down_poles)
            } else if c < 1.15 {
                let a = (c - 0.35) / 0.8;
                *lift = (a * PI).sin() * 0.95;
                *flip = Some((fr.r, -smooth(a) * 2.0 * PI));
                *pelvis_h = ph - 0.15;
                // tuck
                feet[0] = fr.p(v3(0.15, 0.45 * h, -0.12)) + UP * *lift;
                feet[1] = fr.p(v3(0.15, 0.45 * h, 0.12)) + UP * *lift;
                ([fr.p(v3(0.35, 0.9 * h + *lift, -0.2)), fr.p(v3(0.35, 0.9 * h + *lift, 0.2))], down_poles)
            } else if c < 1.45 {
                let k = 1.0 - smooth((c - 1.15) / 0.3);
                *pelvis_h = ph - 0.25 * k;
                *lean = 0.3 * k;
                ([fr.p(v3(0.4, 1.2 * h, -0.4)), fr.p(v3(0.4, 1.2 * h, 0.4))], down_poles)
            } else {
                (up_hands, down_poles)
            }
        }
        // Shuffle
        2 => {
            let s = (t * 2.0 * PI * 2.2).sin();
            let bounce = (t * 2.0 * PI * 4.4).sin().abs() * 0.05;
            *pelvis_h = ph - 0.06 + bounce;
            feet[0] = fr.p(v3(0.18 * s, 0.06 + 0.12 * s.max(0.0), -0.16 * h));
            feet[1] = fr.p(v3(-0.18 * s, 0.06 + 0.12 * (-s).max(0.0), 0.16 * h));
            *twist = 0.25 * s;
            ([fr.p(v3(0.25, 1.35 * h + 0.2 * s, -0.3)), fr.p(v3(0.25, 1.35 * h - 0.2 * s, 0.3))], down_poles)
        }
        // Robot
        3 => {
            let step = (t / 0.4) as i32;
            let poses = [
                ([v3(0.35, 1.25, -0.3), v3(0.0, 0.9, 0.35)], 0.0),
                ([v3(0.0, 0.9, -0.35), v3(0.35, 1.25, 0.3)], 0.4),
                ([v3(0.2, 1.7, -0.35), v3(0.2, 1.7, 0.35)], -0.4),
                ([v3(0.4, 1.3, -0.2), v3(0.4, 1.3, 0.2)], 0.0),
            ];
            let (hh, tw) = poses[(step as usize) % 4];
            *twist = tw;
            *head_tilt = -0.1 + tw * 0.2;
            ([fr.p(v3(hh[0].x, hh[0].y * h, hh[0].z)), fr.p(v3(hh[1].x, hh[1].y * h, hh[1].z))], [fr.d(v3(-1.0, -0.2, -0.3)), fr.d(v3(-1.0, -0.2, 0.3))])
        }
        // Helicopter
        4 => {
            *yaw += t * 9.0;
            let f2 = Frame::from_yaw(fr.o, *yaw);
            feet[0] = f2.p(v3(0.0, 0.06, -0.14));
            feet[1] = f2.p(v3(0.15, 0.35 + 0.05 * (t * 9.0).sin(), 0.2));
            ([f2.p(v3(0.0, 1.45 * h, -0.85)), f2.p(v3(0.0, 1.45 * h, 0.85))], [f2.d(v3(0.0, -1.0, 0.0)), f2.d(v3(0.0, -1.0, 0.0))])
        }
        // Gorilla chest pound
        5 => {
            *pelvis_h = ph - 0.2;
            *lean = 0.2;
            *head_tilt = -0.45;
            feet[0] = fr.p(v3(0.0, 0.06, -0.3 * h));
            feet[1] = fr.p(v3(0.0, 0.06, 0.3 * h));
            let a = (t * 2.0 * PI * 3.0).sin();
            let l = fr.p(v3(0.22 + 0.15 * a.max(0.0), 1.32 * h, -0.1));
            let r = fr.p(v3(0.22 + 0.15 * (-a).max(0.0), 1.32 * h, 0.1));
            ([l, r], [fr.d(v3(0.0, -0.5, -1.0)), fr.d(v3(0.0, -0.5, 1.0))])
        }
        // Moonwalk
        6 => {
            let s = (t * 2.0 * PI * 1.2).sin();
            let back = -fr.f * ((t * 0.6) % 2.0 - 1.0);
            *origin += back * 0.0;
            feet[0] = fr.p(v3(-0.15 * s, 0.06 + 0.06 * s.max(0.0), -0.1)) - fr.f * (t * 0.25 % 1.0);
            feet[1] = fr.p(v3(0.15 * s, 0.06 + 0.06 * (-s).max(0.0), 0.1)) - fr.f * (t * 0.25 % 1.0);
            *origin = fr.o - fr.f * (t * 0.25 % 1.0);
            *head_tilt = -0.2;
            ([fr.p(v3(0.1, 1.95 * h, -0.05)), fr.p(v3(0.2, 1.0 * h, 0.35))], down_poles)
        }
        // Floss
        _ => {
            let s = (t * 2.0 * PI * 2.0).sin();
            *side = 0.12 * s;
            *pelvis_h = ph - 0.04;
            let hz = 0.35 * s;
            let fl = if (t * 4.0) as i32 % 2 == 0 { 0.25 } else { -0.25 };
            ([fr.p(v3(fl, 0.9 * h, -0.15 + hz)), fr.p(v3(-fl, 0.9 * h, 0.15 + hz))], down_poles)
        }
    }
}
