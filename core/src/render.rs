//! Scene assembly: infinite white floor, fighters with cosmetics and damage,
//! shadows, blood decals and particle effects — all as flat-shaded low-poly.
use crate::catalog::*;
use crate::fighter::*;
use crate::fx::*;
use crate::mesh::*;
use crate::math::*;

pub const FLOOR: [f32; 3] = [0.972, 0.973, 0.978];
const BRUISE: [f32; 3] = [0.42, 0.18, 0.32];
const BLOOD: [f32; 3] = [0.6, 0.02, 0.05];
const SHADOW: [f32; 3] = [0.80, 0.81, 0.86];

pub fn floor(m: &mut Mesh, focus: V3) {
    let c = v3((focus.x / 4.0).round() * 4.0, 0.0, (focus.z / 4.0).round() * 4.0);
    let s = 220.0;
    m.quad_n(c + v3(-s, 0.0, -s), c + v3(s, 0.0, -s), c + v3(s, 0.0, s), c + v3(-s, 0.0, s), UP, unlit(FLOOR));
    // faint grid fading out with distance
    let range = 14i32;
    let gc = v3(focus.x.round(), 0.0, focus.z.round());
    let w = 0.011;
    for i in -range..=range {
        for seg in -range..range {
            for axis in 0..2 {
                let (a, b) = if axis == 0 {
                    (gc + v3(seg as f32, 0.001, i as f32), gc + v3(seg as f32 + 1.0, 0.001, i as f32))
                } else {
                    (gc + v3(i as f32, 0.001, seg as f32), gc + v3(i as f32, 0.001, seg as f32 + 1.0))
                };
                let mid = (a + b) * 0.5;
                let d = (mid - focus).flat().len() / range as f32;
                if d >= 1.0 {
                    continue;
                }
                let major = i % 5 == 0;
                let k = (1.0 - d) * (1.0 - d) * if major { 0.10 } else { 0.05 };
                let col = mix3(FLOOR, [0.55, 0.57, 0.65], k);
                let side = if axis == 0 { v3(0.0, 0.0, w) } else { v3(w, 0.0, 0.0) };
                m.quad_n(a - side, b - side, b + side, a + side, UP, unlit(col));
            }
        }
    }
}

pub fn shadow(m: &mut Mesh, f: &Fighter) {
    for &(i, s) in &[(PELVIS, 2.2), (CHEST, 2.0), (HEAD, 1.3), (LFT, 1.6), (RFT, 1.6), (LKN, 1.4), (RKN, 1.4), (LHA, 1.4), (RHA, 1.4), (SPINE, 2.1)] {
        let p = f.pos(i);
        let r = f.body[i].r * s * 1.35 / (1.0 + p.y * 0.3);
        let k = clampf(1.0 - p.y / 2.2, 0.0, 1.0);
        if k <= 0.0 {
            continue;
        }
        m.disc(v3(p.x, 0.004, p.z), r * (0.6 + 0.4 * k), 10, unlit(mix3(FLOOR, SHADOW, 0.4 + 0.6 * k)));
    }
}

pub fn decals(m: &mut Mesh, fx: &Fx) {
    for (k, d) in fx.decals.iter().enumerate() {
        let mut rng = Rng::new(d.seed as u64);
        let n = 9;
        let r = d.r * (0.4 + 0.6 * d.grow);
        let c = v3(d.p.x, 0.006 + (k % 4) as f32 * 0.0007, d.p.z);
        let mut pts = [ZERO; 9];
        for (i, p) in pts.iter_mut().enumerate() {
            let a = d.rot + i as f32 / n as f32 * std::f32::consts::TAU;
            let rr = r * rng.range(0.6, 1.25);
            *p = c + v3(a.cos() * rr, 0.0, a.sin() * rr);
        }
        let col = unlit(d.col);
        for i in 0..n {
            m.tri_n(c, pts[i], pts[(i + 1) % n], UP, col);
        }
        // satellites
        for _ in 0..2 {
            let a = rng.range(0.0, 6.28);
            let dd = r * rng.range(1.3, 2.0);
            let sc = c + v3(a.cos() * dd, 0.0, a.sin() * dd);
            m.disc(sc, r * rng.range(0.15, 0.3), 5, col);
        }
    }
}

pub fn particles(m: &mut Mesh, fx: &Fx, cam: V3) {
    for p in &fx.parts {
        let k = p.life / p.max;
        match p.kind {
            PKind::Blood | PKind::Sweat => {
                let col = if p.kind == PKind::Blood { unlit(p.col) } else { lit(p.col) };
                m.octa(p.p, p.size, p.v * 0.6, col);
            }
            PKind::Spark => {
                let len = p.size * (1.0 - k) * 2.2;
                let w = p.size * 0.22 * (1.0 - k);
                let ax = p.axis;
                let to_cam = (cam - p.p).norm();
                let side = ax.cross(to_cam).norm_or(UP) * w;
                let a = p.p + ax * (len * 0.3);
                let b = p.p + ax * (len * 1.3);
                m.tri_n(a - side, a + side, b, to_cam, unlit(p.col));
            }
            PKind::Ring => {
                let r = p.size * (0.4 + k * 1.6);
                let th = p.size * 0.25 * (1.0 - k);
                let to_cam = (cam - p.p).norm();
                let x = to_cam.cross(UP).norm_or(v3(1.0, 0.0, 0.0));
                let y = x.cross(to_cam);
                let n = 12;
                for i in 0..n {
                    let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
                    let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
                    let d0 = x * a0.cos() + y * a0.sin();
                    let d1 = x * a1.cos() + y * a1.sin();
                    m.quad_n(p.p + d0 * r, p.p + d1 * r, p.p + d1 * (r + th), p.p + d0 * (r + th), to_cam, unlit(p.col));
                }
            }
            PKind::Dust => {
                let s = p.size * (0.5 + k) * (1.0 - k * k);
                m.octa(p.p, s, ZERO, lit(p.col));
            }
            PKind::Confetti => {
                let s = p.size;
                let spin = p.life * 9.0;
                let a = p.axis.rot_axis(UP, spin).norm_or(v3(1.0, 0.0, 0.0));
                let b = a.cross(UP).norm_or(v3(0.0, 0.0, 1.0)).rot_axis(a, spin * 0.7);
                m.quad_n(p.p - a * s - b * s * 0.5, p.p + a * s - b * s * 0.5, p.p + a * s + b * s * 0.5, p.p - a * s + b * s * 0.5, a.cross(b).norm_or(UP), unlit(p.col));
            }
        }
    }
}

struct Pal {
    skin: [f32; 3],
    hair: [f32; 3],
}

fn bruised(skin: [f32; 3], b: f32) -> [f32; 3] {
    let red = mix3(skin, [skin[0] * 1.05 + 0.08, skin[1] * 0.75, skin[2] * 0.75], (b * 2.0).min(1.0));
    mix3(red, BRUISE, (b - 0.35).max(0.0) * 0.9)
}

/// Six-point torso ring.
fn ring6(c: V3, r: V3, f: V3, hw: f32, d: f32) -> [V3; 6] {
    [
        c + r * hw,
        c + r * (hw * 0.62) + f * d,
        c - r * (hw * 0.62) + f * d,
        c - r * hw,
        c - r * (hw * 0.6) - f * (d * 0.9),
        c + r * (hw * 0.6) - f * (d * 0.9),
    ]
}

pub fn fighter(m: &mut Mesh, f: &Fighter) {
    let lk = &f.look;
    let d = &f.dims;
    let _h = d.h;
    let w = d.w;
    let skin_i = lk.get(Slot::Skin);
    let pal = Pal { skin: skin_i.c1, hair: lk.get(Slot::HairColor).c1 };
    let top = lk.get(Slot::Top);
    let bottom = lk.get(Slot::Bottom);
    let gloves = lk.get(Slot::Gloves);
    let shoes = lk.get(Slot::Shoes);
    let p = |i: usize| f.pos(i);

    // body frame from particles
    let right_h = (p(RHIP) - p(LHIP)).norm_or(v3(0.0, 0.0, 1.0));
    let right_s = (p(RSH) - p(LSH)).norm_or(right_h);
    let up = (p(NECK) - p(PELVIS)).norm_or(UP);
    let fwd_h = up.cross(right_h).norm_or(v3(1.0, 0.0, 0.0));
    let fwd_s = up.cross(right_s).norm_or(fwd_h);

    let skin_torso = bruised(pal.skin, f.bruise[1]);
    let torso_col = |seg: usize| -> [f32; 3] {
        match (seg, top.style) {
            (_, 0) => skin_torso,
            (_, 1) | (_, 2) | (_, 3) => top.c1,
            (_, _) => {
                if seg == 2 {
                    top.c2
                } else {
                    top.c1
                }
            }
        }
    };
    // ---------------- torso
    let hips_c = (p(LHIP) + p(RHIP)) * 0.5 - up * 0.03;
    let waist_c = p(PELVIS).lerp(p(SPINE), 0.7);
    let belt_c = p(PELVIS).lerp(p(SPINE), 0.95);
    let chest_c = p(SPINE).lerp(p(CHEST), 0.75);
    let sh_c = p(CHEST).lerp(p(NECK), 0.62);
    let rh = ring6(hips_c, right_h, fwd_h, d.hip_w + 0.075 * w, 0.1 * w);
    let rw = ring6(waist_c, right_h.lerp(right_s, 0.3).norm(), fwd_h, d.hip_w + 0.06 * w, 0.095 * w);
    let rb = ring6(belt_c, right_h.lerp(right_s, 0.5).norm(), fwd_h.lerp(fwd_s, 0.5).norm(), d.hip_w + 0.065 * w, 0.1 * w);
    let rc = ring6(chest_c, right_s, fwd_s, d.sh_w * 0.86, 0.125 * w);
    let rs = ring6(sh_c, right_s, fwd_s, d.sh_w + 0.035, 0.085 * w);
    let center = p(SPINE);
    let short_col = bottom.c1;
    m.ring_strip(&rh, &rw, center, lit(short_col));
    m.cap(&rh, center, lit(short_col));
    let band = if bottom.style == 1 { bottom.c2 } else { short_col };
    m.ring_strip(&rw, &rb, center, lit(band));
    m.ring_strip(&rb, &rc, center, lit(torso_col(1)));
    m.ring_strip(&rc, &rs, center, lit(torso_col(2)));
    m.cap(&rs, center, lit(torso_col(2)));
    // pecs / abs detail for bare torso: a subtle center line
    if top.style == 0 {
        let a = p(SPINE).lerp(p(CHEST), 0.2) + fwd_s * (0.112 * w);
        let b = p(CHEST) + fwd_s * (0.122 * w);
        m.prism(a, b, 0.006, 0.006, 4, fwd_s, lit(scale3(pal.skin, 0.85)));
    }
    if top.style == 3 {
        // hood bump on the back
        let hc = p(NECK) - fwd_s * 0.11 - up * 0.02;
        m.ico(&Frame::from_up_fwd(hc, up, fwd_s), v3(0.06, 0.07, 0.12), |_| Some(lit(top.c2)));
    }

    // ---------------- neck & head
    let head_up = (p(HEAD) - p(NECK)).norm_or(up);
    m.prism(p(NECK) - up * 0.04, p(HEAD) - head_up * 0.05, 0.055 * w, 0.05 * w, 6, fwd_s, lit(bruised(pal.skin, f.bruise[0] * 0.3)));
    draw_head(m, f, &pal, head_up, fwd_s);

    // ---------------- arms
    let sleeve = |upper: bool| -> Option<[f32; 3]> {
        match top.style {
            2 if upper => Some(top.c1),
            3 => Some(top.c1),
            4 => Some(top.c2),
            _ => None,
        }
    };
    for (s, e, hnd, bi) in [(LSH, LEL, LHA, 2usize), (RSH, REL, RHA, 3usize)] {
        let skin_a = bruised(pal.skin, f.bruise[bi]);
        let ref_up = fwd_s;
        let up_col = sleeve(true).unwrap_or(skin_a);
        if top.style == 2 {
            let mid = p(s).lerp(p(e), 0.5);
            m.prism(p(s), mid, 0.068 * w, 0.06 * w, 6, ref_up, lit(up_col));
            m.prism(mid, p(e), 0.052 * w, 0.046 * w, 6, ref_up, lit(skin_a));
        } else {
            m.prism(p(s), p(e), 0.058 * w, 0.047 * w, 6, ref_up, lit(up_col));
        }
        let fa_col = sleeve(false).unwrap_or(skin_a);
        let wrist = p(e).lerp(p(hnd), 0.82);
        m.prism(p(e), wrist, 0.046 * w, 0.037 * w, 6, ref_up, lit(fa_col));
        draw_glove(m, gloves, p(e), p(hnd), fwd_s, &pal, w);
    }

    // ---------------- legs
    for (hip, kn, ft, toe, bi) in [(LHIP, LKN, LFT, LTOE, 4usize), (RHIP, RKN, RFT, RTOE, 5usize)] {
        let skin_l = bruised(pal.skin, f.bruise[bi]);
        let ref_up = fwd_h;
        match bottom.style {
            2 => {
                m.prism(p(hip), p(kn), 0.092 * w, 0.068 * w, 6, ref_up, lit(bottom.c1));
                m.prism(p(kn), p(ft), 0.065 * w, 0.055 * w, 6, ref_up, lit(bottom.c1));
            }
            _ => {
                let cut = if bottom.style == 1 { 0.35 } else { 0.55 };
                let mid = p(hip).lerp(p(kn), cut);
                m.prism(p(hip) - up * 0.02, mid, 0.1 * w, 0.088 * w, 6, ref_up, lit(bottom.c1));
                if bottom.style == 0 {
                    // side stripe
                    let side = (p(hip) - p(PELVIS)).norm_or(right_h);
                    m.prism(p(hip) + side * 0.09 * w, mid + side * 0.08 * w, 0.012, 0.012, 4, ref_up, lit(bottom.c2));
                }
                m.prism(mid, p(kn), 0.078 * w, 0.06 * w, 6, ref_up, lit(skin_l));
                if shoes.style == 2 {
                    let bt = p(kn).lerp(p(ft), 0.45);
                    m.prism(p(kn), bt, 0.058 * w, 0.054 * w, 6, ref_up, lit(skin_l));
                    m.prism(bt, p(ft), 0.06 * w, 0.058 * w, 6, ref_up, lit(shoes.c1));
                } else {
                    m.prism(p(kn), p(ft), 0.058 * w, 0.042 * w, 6, ref_up, lit(skin_l));
                }
            }
        }
        // foot
        let heel = p(ft) - (p(toe) - p(ft)).norm_or(fwd_h) * 0.04;
        let fdir = (p(toe) - heel).norm_or(fwd_h);
        let side = fdir.cross(UP).norm_or(right_h);
        let fup = side.cross(fdir).norm_or(UP);
        let len = (p(toe) - heel).len() * 0.5 + 0.02;
        let fc = heel.lerp(p(toe), 0.5);
        match shoes.style {
            0 => m.obox(fc, fdir, fup, side, v3(len, 0.03, 0.042), lit(skin_l)),
            1 => {
                m.obox(fc + fup * 0.012, fdir, fup, side, v3(len, 0.036, 0.05), lit(shoes.c1));
                m.obox(fc - fup * 0.03, fdir, fup, side, v3(len + 0.008, 0.012, 0.054), lit(shoes.c2));
            }
            2 => {
                m.obox(fc + fup * 0.01, fdir, fup, side, v3(len, 0.04, 0.048), lit(shoes.c1));
                m.obox(fc - fup * 0.032, fdir, fup, side, v3(len + 0.004, 0.008, 0.05), lit(shoes.c2));
            }
            _ => {
                m.obox(fc, fdir, fup, side, v3(len, 0.03, 0.042), lit(skin_l));
                m.obox(fc - fup * 0.035, fdir, fup, side, v3(len + 0.01, 0.007, 0.05), lit(shoes.c1));
            }
        }
    }

    // ---------------- neck accessory
    let neck_i = lk.get(Slot::Neck);
    let nb = p(NECK).lerp(p(CHEST), 0.35);
    match neck_i.style {
        1 | 2 => {
            let n = if neck_i.style == 1 { 14 } else { 12 };
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let dir = fwd_s * a.cos() + right_s * a.sin();
                let sag = (a.cos().max(0.0)).powf(2.0) * 0.07;
                let pt = nb + dir * (0.085 * w + sag * 0.3) - up * sag;
                let col = if neck_i.style == 2 && i % 2 == 1 { neck_i.c2 } else { neck_i.c1 };
                if neck_i.style == 1 {
                    m.obox(pt, dir.cross(up).norm_or(right_s), up, dir, v3(0.014, 0.009, 0.008), lit(col));
                } else {
                    m.octa(pt, 0.014, ZERO, lit(col));
                }
            }
            if neck_i.style == 1 {
                let pend = nb + fwd_s * (0.11 * w) - up * 0.1;
                m.obox(pend, right_s, up, fwd_s, v3(0.022, 0.028, 0.008), lit(neck_i.c1));
            }
        }
        3 => {
            let c = p(NECK) + fwd_s * 0.06 - up * 0.02;
            m.obox(c + right_s * 0.025, right_s, up, fwd_s, v3(0.025, 0.018, 0.01), lit(neck_i.c1));
            m.obox(c - right_s * 0.025, right_s, up, fwd_s, v3(0.025, 0.018, 0.01), lit(neck_i.c1));
            m.obox(c, right_s, up, fwd_s, v3(0.01, 0.01, 0.012), lit(scale3(neck_i.c1, 0.7)));
        }
        _ => {}
    }
}

fn draw_glove(m: &mut Mesh, g: &Item, elbow: V3, hand: V3, fwd: V3, pal: &Pal, w: f32) {
    let dir = (hand - elbow).norm_or(fwd);
    let fr = Frame::from_up_fwd(hand, dir, fwd);
    match g.style {
        1 => {
            // boxing glove: chunky rounded
            let c = hand + dir * 0.02;
            m.ico(&Frame { o: c, ..fr }, v3(0.075, 0.085, 0.07) * w.sqrt(), |l| Some(lit(if l.y < -0.55 { g.c2 } else { g.c1 })));
            m.prism(hand - dir * 0.08, hand - dir * 0.02, 0.05, 0.055, 6, fwd, lit(g.c2));
        }
        2 => {
            // hand wraps
            m.obox(hand, fr.f, fr.u, fr.r, v3(0.045, 0.05, 0.04) * w.sqrt(), lit(pal.skin));
            m.prism(hand - dir * 0.07, hand + dir * 0.01, 0.043, 0.048, 6, fwd, lit(g.c1));
        }
        _ => {
            // MMA glove: padded knuckles + wrist strap
            m.obox(hand + dir * 0.005, fr.f, fr.u, fr.r, v3(0.05, 0.055, 0.045) * w.sqrt(), lit(g.c1));
            m.obox(hand + dir * 0.03 + fr.f * 0.01, fr.f, fr.u, fr.r, v3(0.045, 0.03, 0.05) * w.sqrt(), lit(scale3(g.c1, 0.8)));
            m.prism(hand - dir * 0.075, hand - dir * 0.03, 0.044, 0.048, 6, fwd, lit(if g.c2 == g.c1 { [0.95, 0.95, 0.95] } else { g.c2 }));
        }
    }
}

fn draw_head(m: &mut Mesh, f: &Fighter, pal: &Pal, head_up: V3, fwd: V3) {
    let lk = &f.look;
    let h = f.dims.h;
    let swell = 1.0 + f.bruise[0] * 0.1;
    let fr = Frame::from_up_fwd(f.pos(HEAD), head_up, fwd);
    let sc = v3(0.102, 0.122, 0.096) * (h * swell);
    let skin_face = bruised(pal.skin, f.bruise[0]);
    let face_i = lk.get(Slot::Face);
    let blood = f.blood;
    let mut rng = Rng::new(7);
    let ko = f.is_ko();
    // base head
    m.ico(&fr, sc, |l| {
        let front = l.x > 0.25;
        let r = rng.f();
        let mut c = if front { skin_face } else { bruised(pal.skin, f.bruise[0] * 0.4) };
        if front && l.y < 0.3 && r < blood * 0.9 {
            c = mix3(c, BLOOD, 0.6 + 0.4 * blood);
        }
        Some(lit(c))
    });
    let at = |l: V3| fr.p(v3(l.x * h, l.y * h, l.z * h));
    let dark = [0.06, 0.05, 0.06];
    // ears
    for s in [-1.0f32, 1.0] {
        m.obox(at(v3(-0.005, 0.0, 0.096 * s)), fr.f, fr.u, fr.r, v3(0.018, 0.026, 0.01) * h, lit(skin_face));
    }
    // nose
    m.obox(at(v3(0.1, -0.005, 0.0)), fr.f, fr.u, fr.r, v3(0.014, 0.022, 0.012) * h, lit(scale3(skin_face, 0.9)));
    // eyes / brows / mouth
    let rocked = f.rocked();
    for s in [-1.0f32, 1.0] {
        let ec = at(v3(0.094, 0.02, 0.038 * s));
        if ko {
            let a = fr.u.rot_axis(fr.f, 0.785);
            let b = fr.u.rot_axis(fr.f, -0.785);
            for ax in [a, b] {
                let r = ax.cross(fr.f);
                m.obox(ec, fr.f, ax, r, v3(0.006, 0.02, 0.004) * h, unlit(dark));
            }
        } else {
            let eh = if rocked { 0.005 } else if matches!(f.action, Action::Hurt { .. }) { 0.004 } else { 0.012 };
            m.obox(ec, fr.f, fr.u, fr.r, v3(0.006, eh, 0.008) * h, unlit(dark));
            // angry brows
            let bu = fr.u.rot_axis(fr.f, 0.35 * s);
            m.obox(at(v3(0.098, 0.042, 0.04 * s)), fr.f, bu, bu.cross(fr.f), v3(0.008, 0.005, 0.02) * h, lit(pal.hair));
        }
    }
    let open = ko || matches!(f.action, Action::Hurt { .. } | Action::Strike { .. });
    m.obox(at(v3(0.088, -0.052, 0.0)), fr.f, fr.u, fr.r, v3(0.008, if open { 0.012 } else { 0.004 }, 0.022) * h, unlit([0.35, 0.05, 0.08]));

    // hair
    let hair_i = lk.get(Slot::Hair);
    let hair = lit(pal.hair);
    match hair_i.style {
        1 => m.ico(&fr, sc * 1.06, |l| if l.y > 0.2 || (l.x < -0.2 && l.y > -0.45) { Some(hair) } else { None }),
        2 => {
            m.ico(&fr, sc * 1.03, |l| if l.y > 0.55 && l.z.abs() < 0.35 { Some(hair) } else { None });
            for i in 0..5 {
                let t = i as f32 / 4.0;
                let a = -0.3 + t * 2.3;
                let pos = fr.p(v3(a.cos() * sc.x * 0.95, a.sin() * sc.y * 1.0 + 0.03 * h, 0.0)) ;
                let dirup = (pos - fr.o).norm();
                m.obox(pos, dirup.cross(fr.r), dirup, fr.r, v3(0.02, 0.045, 0.01) * h, hair);
            }
        }
        3 => {
            let c = Frame { o: fr.p(v3(-0.02 * h, 0.06 * h, 0.0)), ..fr };
            m.ico(&c, sc * 1.55, |l| if l.x < 0.35 || l.y > 0.45 { Some(hair) } else { None });
        }
        4 => {
            m.ico(&fr, sc * 1.05, |l| if l.y > 0.15 || (l.x < -0.2 && l.y > -0.4) { Some(hair) } else { None });
            let bun = Frame { o: fr.p(v3(-0.09 * h, 0.1 * h, 0.0)), ..fr };
            m.ico(&bun, v3(0.045, 0.045, 0.045) * h, |_| Some(hair));
        }
        5 => {
            m.ico(&fr, sc * 1.04, |l| if l.y > 0.25 { Some(hair) } else { None });
            let mut r2 = Rng::new(3);
            for _ in 0..9 {
                let mut d = r2.unit();
                d.y = d.y.abs() + 0.6;
                let d = d.norm();
                let base = fr.p(v3(d.x * sc.x, d.y * sc.y, d.z * sc.z));
                let wd = fr.d(d).norm();
                m.prism(base, base + wd * (0.07 * h), 0.022 * h, 0.002, 4, fr.f, hair);
            }
        }
        _ => {}
    }
    // beard
    let beard_i = lk.get(Slot::Beard);
    match beard_i.style {
        1 => m.ico(&fr, sc * 1.01, |l| if l.y < -0.25 && l.x > -0.1 { Some(lit(mix3(skin_face, pal.hair, 0.45))) } else { None }),
        2 => m.ico(&fr, sc * 1.08, |l| if l.y < -0.15 && l.x > -0.15 && !(l.x > 0.8 && l.y > -0.45) { Some(hair) } else { None }),
        3 => m.obox(at(v3(0.088, -0.09, 0.0)), fr.f, fr.u, fr.r, v3(0.02, 0.03, 0.022) * h, hair),
        _ => {}
    }
    // face slot
    match face_i.style {
        1 => m.ico(&fr, sc * 1.04, |l| {
            let eye = l.x > 0.6 && l.y > 0.0 && l.y < 0.4 && l.z.abs() > 0.15 && l.z.abs() < 0.6;
            let mouth = l.x > 0.7 && l.y < -0.3;
            if eye || mouth {
                None
            } else if l.z.abs() < 0.18 && l.y > -0.1 {
                Some(lit(face_i.c2))
            } else {
                Some(lit(face_i.c1))
            }
        }),
        2 => m.ico(&fr, sc * 1.035, |l| if l.x > 0.55 && l.y > -0.05 && l.y < 0.4 { None } else { Some(lit(face_i.c1)) }),
        3 => m.ico(&fr, sc * 1.05, |l| if l.y < -0.05 && l.x > -0.2 { Some(lit(if l.z.abs() < 0.12 { face_i.c2 } else { face_i.c1 })) } else { None }),
        4 => {
            let c = Frame { o: at(v3(0.118, -0.005, 0.0)), ..fr };
            m.ico(&c, v3(0.024, 0.024, 0.024) * h, |_| Some(lit(face_i.c1)));
        }
        _ => {}
    }
    // eyes slot
    let eyes_i = lk.get(Slot::Eyes);
    match eyes_i.style {
        1 | 2 | 4 => {
            for s in [-1.0f32, 1.0] {
                let c = at(v3(0.104, 0.02, 0.039 * s));
                let (he, col) = match eyes_i.style {
                    1 => (v3(0.006, 0.017, 0.026), eyes_i.c1),
                    2 => (v3(0.006, 0.02, 0.025), eyes_i.c1),
                    _ => (v3(0.006, 0.02, 0.022), eyes_i.c1),
                };
                let u = if eyes_i.style == 4 { fr.u.rot_axis(fr.f, 0.785) } else { fr.u };
                m.obox(c, fr.f, u, u.cross(fr.f), he * h, if eyes_i.style == 4 { unlit(col) } else { lit(col) });
                if eyes_i.style == 2 {
                    m.obox(c + fr.u * (0.022 * h), fr.f, fr.u, fr.r, v3(0.004, 0.003, 0.027) * h, lit(eyes_i.c2));
                }
                // temple arm
                m.obox(at(v3(0.05, 0.025, 0.07 * s)), fr.f, fr.u, fr.r, v3(0.05, 0.003, 0.003) * h, lit(if eyes_i.style == 2 { eyes_i.c2 } else { dark }));
            }
            m.obox(at(v3(0.106, 0.026, 0.0)), fr.f, fr.u, fr.r, v3(0.004, 0.003, 0.014) * h, lit(if eyes_i.style == 2 { eyes_i.c2 } else { dark }));
        }
        3 => {
            m.obox(at(v3(0.1, 0.022, 0.0)), fr.f, fr.u, fr.r, v3(0.012, 0.025, 0.07) * h, unlit(eyes_i.c1));
            m.ico(&fr, sc * 1.06, |l| if l.y > -0.05 && l.y < 0.25 && l.x < 0.5 { Some(lit(eyes_i.c2)) } else { None });
        }
        _ => {}
    }
    // head slot
    let hat = lk.get(Slot::Head);
    match hat.style {
        1 => {
            m.ico(&fr, sc * 1.1, |l| if l.y > 0.12 { Some(lit(hat.c1)) } else { None });
            m.obox(at(v3(0.115, 0.045, 0.0)), fr.f.rot_axis(fr.r, 0.12), fr.u.rot_axis(fr.r, 0.12), fr.r, v3(0.065, 0.006, 0.075) * h, lit(hat.c2));
            m.obox(at(v3(0.0, 0.13, 0.0)), fr.f, fr.u, fr.r, v3(0.012, 0.008, 0.012) * h, lit(hat.c2));
        }
        2 => {
            m.ico(&fr, sc.mul(v3(1.1, 1.18, 1.1)), |l| if l.y > 0.05 { Some(lit(hat.c1)) } else { None });
            m.ico(&fr, sc * 1.14, |l| if l.y > -0.05 && l.y < 0.22 && l.x < 0.95 { Some(lit(hat.c2)) } else { None });
            let pom = Frame { o: at(v3(0.0, 0.15, 0.0)), ..fr };
            m.ico(&pom, v3(0.03, 0.03, 0.03) * h, |_| Some(lit(hat.c2)));
        }
        3 => m.ico(&fr, sc * 1.07, |l| if l.y > 0.25 && l.y < 0.55 { Some(lit(hat.c1)) } else { None }),
        4 => {
            let n = 7;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let d = fr.d(v3(a.cos(), 0.0, a.sin()));
                let base = fr.o + fr.u * (0.1 * h) + d * (0.07 * h);
                m.prism(base, base + fr.u * (0.06 * h) + d * 0.01, 0.022 * h, 0.003, 4, d, lit(hat.c1));
                let gem = Frame { o: base + d * (0.02 * h) + fr.u * 0.01, ..fr };
                if i % 2 == 0 {
                    m.ico(&gem, v3(0.008, 0.008, 0.008) * h, |_| Some(unlit(hat.c2)));
                }
            }
            m.prism(fr.o + fr.u * (0.085 * h), fr.o + fr.u * (0.11 * h), 0.085 * h, 0.08 * h, 8, fr.f, lit(hat.c1));
        }
        5 => {
            let brim = fr.o + fr.u * (0.085 * h);
            m.prism(brim, brim + fr.u * 0.012, 0.21 * h, 0.2 * h, 8, fr.f, lit(hat.c1));
            m.prism(brim, brim + fr.u * (0.12 * h), 0.1 * h, 0.085 * h, 6, fr.f, lit(hat.c1));
            m.prism(brim + fr.u * 0.012, brim + fr.u * 0.03, 0.102 * h, 0.1 * h, 6, fr.f, lit(hat.c2));
        }
        _ => {}
    }
}
