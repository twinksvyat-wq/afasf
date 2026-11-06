//! Opponent AI: range management, reactions, combos, clinch and finishers.
//! Drives the same Intent interface as the touch controls.
use crate::fighter::*;
use crate::math::*;

pub struct Ai {
    pub skill: f32,
    rng: Rng,
    think_t: f32,
    react_t: f32,
    pending_block: f32,
    hold_block: f32,
    strafe: f32,
    plan: Option<Plan>,
    clinch_escape_t: f32,
}

#[derive(Clone, Copy)]
enum Plan {
    Punch(u8, bool, bool),
    Kick(bool, bool, bool),
    Grab,
}

impl Ai {
    pub fn new(skill: f32, seed: u64) -> Ai {
        Ai { skill, rng: Rng::new(seed), think_t: 0.5, react_t: 0.0, pending_block: 0.0, hold_block: 0.0, strafe: 1.0, plan: None, clinch_escape_t: 0.0 }
    }

    pub fn intent(&mut self, me: &Fighter, opp: &Fighter, dt: f32) -> Intent {
        let mut it = Intent::default();
        if !me.standing() {
            return it;
        }
        let s = self.skill;
        let to = (opp.pos(PELVIS) - me.pos(PELVIS)).flat();
        let dist = to.len();
        let dir = to.norm_or(me.facing());
        let side = dir.cross(UP);

        // escaping a clinch
        if me.grabbed_by_other {
            self.clinch_escape_t += dt;
            if self.clinch_escape_t > lerpf(1.6, 0.5, s) && self.rng.chance(0.2) {
                it.grab = true;
                it.block = true;
            }
        } else {
            self.clinch_escape_t = 0.0;
        }

        // ---- defence: react to the opponent's attacks
        let incoming = matches!(opp.action, Action::Strike { t, mv, .. } if t < MOVES[mv].windup + MOVES[mv].active * 0.5);
        if incoming && self.pending_block <= 0.0 && self.hold_block <= 0.0 && dist < 1.8 {
            if self.rng.chance(lerpf(0.04, 0.55, s)) {
                self.pending_block = lerpf(0.28, 0.06, s);
            } else {
                self.pending_block = -0.4; // decided not to block this one
            }
        }
        if !incoming && self.pending_block < 0.0 {
            self.pending_block = 0.0;
        }
        if self.pending_block > 0.0 {
            self.pending_block -= dt;
            if self.pending_block <= 0.0 {
                self.hold_block = self.rng.range(0.25, 0.6);
                self.pending_block = 0.0;
            }
        }
        if self.hold_block > 0.0 {
            self.hold_block -= dt;
            it.block = true;
        }

        // ---- range management
        let ideal = if me.arch.kick > 1.1 { 1.15 } else { 1.0 };
        let low_stam = me.stamina < 25.0;
        let mut mv = ZERO;
        let want = if low_stam { ideal + 0.9 } else if opp.is_down() { 2.0 } else { ideal };
        if dist > want + 0.15 {
            mv += dir;
        } else if dist < want - 0.25 {
            mv -= dir;
        }
        self.think_t -= dt;
        if self.think_t <= 0.0 {
            self.think_t = self.rng.range(0.6, 1.6);
            if self.rng.chance(0.5) {
                self.strafe = -self.strafe;
            }
        }
        mv += side * (self.strafe * 0.45);
        it.mv = mv.clamp_len(1.0);

        // ---- offence
        if opp.is_down() || me.busy() || it.block {
            return it;
        }
        // special / finisher
        if me.special >= 100.0 && (opp.rocked() || self.rng.chance(0.01 + 0.02 * s)) && dist < 1.8 {
            it.special = true;
            return it;
        }
        self.react_t -= dt;
        if self.react_t > 0.0 {
            return it;
        }
        if let Some(p) = self.plan.take() {
            match p {
                Plan::Punch(n, fwd, down) => {
                    it.punch = true;
                    it.fwd = fwd;
                    it.down = down;
                    if n > 1 {
                        self.plan = Some(Plan::Punch(n - 1, false, false));
                        self.react_t = lerpf(0.32, 0.17, s);
                    } else {
                        self.react_t = lerpf(1.2, 0.35, s) * self.rng.range(0.7, 1.4);
                    }
                }
                Plan::Kick(fwd, back, down) => {
                    it.kick = true;
                    it.fwd = fwd;
                    it.back = back;
                    it.down = down;
                    self.react_t = lerpf(1.4, 0.5, s) * self.rng.range(0.7, 1.3);
                }
                Plan::Grab => {
                    it.grab = true;
                    self.react_t = 0.5;
                }
            }
            return it;
        }
        if let Action::Clinch { attached: true, t, .. } = me.action {
            if t > 0.4 {
                if self.rng.chance(0.5) {
                    it.kick = true;
                } else if self.rng.chance(0.4 * me.arch.grapple) {
                    it.grab = true;
                } else {
                    it.punch = true;
                }
                self.react_t = lerpf(0.8, 0.35, s);
            }
            return it;
        }
        if me.stamina < 12.0 {
            return it;
        }
        let in_range = dist < 1.35;
        let aggression = lerpf(0.012, 0.05, s) * if opp.rocked() { 3.0 } else { 1.0 };
        if in_range && self.rng.chance(aggression * 60.0 * dt) {
            let r = self.rng.f();
            let kick_bias = (me.arch.kick - 0.8) * 0.5;
            let plan = if r < 0.55 - kick_bias {
                let n = 1 + self.rng.below(if s > 0.5 { 4 } else { 2 }) as u8;
                Plan::Punch(n, self.rng.chance(0.3), self.rng.chance(0.15))
            } else if r < 0.88 {
                let k = self.rng.f();
                Plan::Kick(k < 0.2, k > 0.75, k > 0.5 && k <= 0.75)
            } else if dist < 1.0 {
                Plan::Grab
            } else {
                Plan::Punch(2, true, false)
            };
            self.plan = Some(plan);
            self.react_t = 0.0;
        }
        it
    }
}
