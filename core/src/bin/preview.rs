//! Headless preview: runs the real game core and rasterizes its mesh in software
//! with the same shading model as the Metal shader. Used for tuning & CI checks.
//!
//!   preview fight <seconds> <out.rgb|-> [w h]   raw rgb24 frames (pipe into ffmpeg)
//!   preview shot <scene> <out.ppm> [w h]          single frame
//!   preview sim <fights>                           headless AI-vs-AI statistics
use fightcore::ai::Ai;
use fightcore::fight::*;
use fightcore::game::*;
use fightcore::math::*;
use std::io::Write;

struct Raster {
    w: usize,
    h: usize,
    col: Vec<[f32; 3]>,
    z: Vec<f32>,
}

const FOG: [f32; 3] = [0.972, 0.973, 0.978];

fn shade(albedo: [f32; 4], n: V3, wp: V3, cam: V3, flash: f32) -> [f32; 3] {
    let l = v3(-0.45, 0.85, 0.4).norm();
    let ndl = n.dot(l).max(0.0);
    let t = n.y * 0.5 + 0.5;
    let amb = [lerpf(0.78, 0.92, t), lerpf(0.74, 0.95, t), lerpf(0.72, 1.0, t)];
    let vdir = (cam - wp).norm();
    let rim = (1.0 - n.dot(vdir).max(0.0)).powi(3) * 0.18;
    let key = [1.0, 0.97, 0.92];
    let mut c = [0.0; 3];
    let dist = (wp - cam).len();
    let mut fog = clampf((dist - 18.0) / (90.0 - 18.0), 0.0, 1.0);
    fog = fog * fog * (3.0 - 2.0 * fog);
    for i in 0..3 {
        let lc = albedo[i] * (amb[i] * 0.62 + key[i] * ndl * 0.55) + rim * albedo[3];
        let v = lerpf(albedo[i], lc, albedo[3]);
        let v = lerpf(v, FOG[i], fog);
        c[i] = lerpf(v, 1.0, flash * 0.3).min(1.0);
    }
    c
}

fn shade_nofog(albedo: [f32; 4], n: V3, wp: V3, cam: V3, flash: f32) -> [f32; 3] {
    // evaluate with camera at the point so fog is zero
    let _ = cam;
    shade(albedo, n, wp, wp + (cam - wp).norm() * 0.001, flash)
}

impl Raster {
    fn new(w: usize, h: usize) -> Raster {
        Raster { w, h, col: vec![FOG; w * h], z: vec![f32::INFINITY; w * h] }
    }
    fn clear(&mut self) {
        self.col.iter_mut().for_each(|c| *c = FOG);
        self.z.iter_mut().for_each(|z| *z = f32::INFINITY);
    }
    fn draw(&mut self, g: &Game, flash: f32) {
        let vp = &g.view_proj;
        let cam = g.eye;
        let v = &g.mesh.v;
        for t in v.chunks_exact(3) {
            let ws = [V3 { x: t[0].pos[0], y: t[0].pos[1], z: t[0].pos[2] }, V3 { x: t[1].pos[0], y: t[1].pos[1], z: t[1].pos[2] }, V3 { x: t[2].pos[0], y: t[2].pos[1], z: t[2].pos[2] }];
            let n = V3 { x: t[0].nrm[0], y: t[0].nrm[1], z: t[0].nrm[2] };
            let col = t[0].col;
            // clip against near plane (w > eps) — Sutherland-Hodgman
            let mut poly: Vec<([f32; 4], V3)> = ws.iter().map(|&p| (vp.xform(p), p)).collect();
            let eps = 0.05;
            if poly.iter().any(|(c, _)| c[3] < eps) {
                let mut out = Vec::new();
                for i in 0..poly.len() {
                    let (a, pa) = poly[i];
                    let (b, pb) = poly[(i + 1) % poly.len()];
                    let ina = a[3] >= eps;
                    let inb = b[3] >= eps;
                    if ina {
                        out.push((a, pa));
                    }
                    if ina != inb {
                        let tt = (eps - a[3]) / (b[3] - a[3]);
                        let c = [lerpf(a[0], b[0], tt), lerpf(a[1], b[1], tt), lerpf(a[2], b[2], tt), lerpf(a[3], b[3], tt)];
                        out.push((c, pa.lerp(pb, tt)));
                    }
                }
                poly = out;
                if poly.len() < 3 {
                    continue;
                }
            }
            let scr: Vec<(f32, f32, f32, f32, V3)> = poly
                .iter()
                .map(|(c, p)| {
                    let iw = 1.0 / c[3];
                    ((c[0] * iw * 0.5 + 0.5) * self.w as f32, (1.0 - (c[1] * iw * 0.5 + 0.5)) * self.h as f32, c[2] * iw, iw, *p)
                })
                .collect();
            for k in 1..scr.len() - 1 {
                self.tri(scr[0], scr[k], scr[k + 1], col, n, cam, flash);
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn tri(&mut self, a: (f32, f32, f32, f32, V3), b: (f32, f32, f32, f32, V3), c: (f32, f32, f32, f32, V3), col: [f32; 4], n: V3, cam: V3, flash: f32) {
        let minx = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
        let maxx = a.0.max(b.0).max(c.0).ceil().min(self.w as f32 - 1.0) as i32;
        let miny = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
        let maxy = a.1.max(b.1).max(c.1).ceil().min(self.h as f32 - 1.0) as i32;
        if minx > maxx || miny > maxy {
            return;
        }
        let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        if area.abs() < 1e-9 {
            return;
        }
        // one shade per triangle at centroid for speed (flat shading); fog per triangle
        let cen = (a.4 + b.4 + c.4) * (1.0 / 3.0);
        let shaded = shade(col, n, cen, cam, flash);
        let unf = shade_nofog(col, n, cen, cam, flash);
        let big = (a.4 - b.4).len() > 6.0 || (a.4 - c.4).len() > 6.0;
        for y in miny..=maxy {
            for x in minx..=maxx {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = ((b.0 - px) * (c.1 - py) - (b.1 - py) * (c.0 - px)) / area;
                let w1 = ((c.0 - px) * (a.1 - py) - (c.1 - py) * (a.0 - px)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
                if !(0.0..=1.0).contains(&z) {
                    continue;
                }
                let i = y as usize * self.w + x as usize;
                if z < self.z[i] {
                    self.z[i] = z;
                    self.col[i] = if big {
                        let iw = w0 * a.3 + w1 * b.3 + w2 * c.3;
                        let wp = (a.4 * (w0 * a.3) + b.4 * (w1 * b.3) + c.4 * (w2 * c.3)) / iw;
                        let dist = (wp - cam).len();
                        let mut fog = clampf((dist - 18.0) / 72.0, 0.0, 1.0);
                        fog = fog * fog * (3.0 - 2.0 * fog);
                        [lerpf(unf[0], FOG[0], fog), lerpf(unf[1], FOG[1], fog), lerpf(unf[2], FOG[2], fog)]
                    } else {
                        shaded
                    };
                }
            }
        }
    }
    fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 3]) {
        let (sx, sy) = (self.w as f32, self.h as f32);
        for y in (y0 * sy) as usize..((y1 * sy) as usize).min(self.h) {
            for x in (x0 * sx) as usize..((x1 * sx) as usize).min(self.w) {
                self.col[y * self.w + x] = c;
            }
        }
    }
    fn hud(&mut self, g: &Game) {
        let Some(f) = &g.fight else { return };
        for (i, fi) in f.f.iter().enumerate() {
            let (x0, dir) = if i == 0 { (0.04, 1.0) } else { (0.96, -1.0) };
            let bar = |r: &mut Raster, y: f32, v: f32, c: [f32; 3], h: f32| {
                let w = 0.36;
                let (a, b) = if dir > 0.0 { (x0, x0 + w) } else { (x0 - w, x0) };
                r.rect(a, y, b, y + h, [0.15, 0.15, 0.17]);
                let (a2, b2) = if dir > 0.0 { (x0, x0 + w * v) } else { (x0 - w * v, x0) };
                r.rect(a2, y, b2, y + h, c);
            };
            bar(self, 0.04, (fi.hp_head / 100.0).clamp(0.0, 1.0), [0.95, 0.2, 0.2], 0.03);
            bar(self, 0.08, (fi.stamina / 100.0).clamp(0.0, 1.0), [0.2, 0.75, 1.0], 0.012);
            bar(self, 0.1, (fi.special / 100.0).clamp(0.0, 1.0), [1.0, 0.8, 0.1], 0.012);
        }
    }
    fn downsample(&self, s: usize) -> Vec<u8> {
        let (w, h) = (self.w / s, self.h / s);
        let mut out = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 3];
                for dy in 0..s {
                    for dx in 0..s {
                        let c = self.col[(y * s + dy) * self.w + x * s + dx];
                        for k in 0..3 {
                            acc[k] += c[k];
                        }
                    }
                }
                for k in 0..3 {
                    out.push((acc[k] / (s * s) as f32 * 255.0).clamp(0.0, 255.0) as u8);
                }
            }
        }
        out
    }
}

fn flash(g: &Game) -> f32 {
    g.fight.as_ref().map(|f| f.fx.flash).unwrap_or(0.0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("sim");
    let w: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(960);
    let h: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(540);
    const SS: usize = 2;
    match mode {
        "sim" => {
            let n: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let mut wins = 0;
            let mut total_t = 0.0;
            for i in 0..n {
                let mut g = Game::new(1000 + i as u64);
                g.profile.ladder = 7;
                g.autoplay = Some(Ai::new(0.6, 77 + i as u64));
                g.start_fight(i % 8);
                g.resize(960.0, 540.0);
                let mut t = 0.0;
                let mut parts = [[0u32; 4]; 2];
                let mut dmgp = [[0f32; 4]; 2];
                let mut blocks = [0u32; 2];
                while g.result.valid == 0 && t < 400.0 {
                    g.update(1.0 / 60.0);
                    while let Some(e) = g.events.pop_front() {
                        if e.kind == 1 { parts[e.fighter as usize][e.b as usize] += 1; dmgp[e.fighter as usize][e.b as usize] += e.a; }
                        if e.kind == 2 { blocks[e.fighter as usize] += 1; }
                    }
                    t += 1.0 / 60.0;
                    let f = g.fight.as_ref().unwrap();
                    assert!(f.f[0].is_finite() && f.f[1].is_finite(), "NaN at t={t}");
                }
                total_t += t;
                println!("   hits taken P {:?} dmg {:?} blocks {} | E {:?} dmg {:?} blocks {}", parts[0], dmgp[0].map(|x| x as i32), blocks[0], parts[1], dmgp[1].map(|x| x as i32), blocks[1]);
                let f = g.fight.as_ref().unwrap();
                let r = g.result;
                if r.won == 1 {
                    wins += 1;
                }
                println!(
                    "fight {i:2} opp {} -> won={} method={} t={:5.1}s rounds={} | P landed {:3}/{:3} kd {} dmg {:5.0} | E landed {:3}/{:3} kd {} | hp P {:4.0}/{:4.0} E {:4.0}/{:4.0}",
                    i % 8, r.won, r.method, t, r.rounds, f.f[0].stats.landed, f.f[0].stats.thrown, f.f[0].stats.knockdowns, f.f[0].stats.damage,
                    f.f[1].stats.landed, f.f[1].stats.thrown, f.f[1].stats.knockdowns, f.f[0].hp_head, f.f[0].hp_body, f.f[1].hp_head, f.f[1].hp_body
                );
            }
            println!("player wins {wins}/{n}, avg fight {:.1}s", total_t / n as f32);
        }
        "fight" | "menu" | "cel" => {
            let secs: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10.0);
            let out = args.get(3).cloned().unwrap_or("-".into());
            let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(42);
            let mut g = Game::new(seed);
            g.resize(w as f32, h as f32);
            if let Ok(c) = std::env::var("PCEL") { g.profile.celebration = c.parse().unwrap_or(0); }
            g.profile.ladder = 7;
            let opp: usize = std::env::var("OPP").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
            if let Ok(c) = std::env::var("CAM") {
                g.cam.mode = match c.as_str() {
                    "iso" => fightcore::camera::CamMode::Iso,
                    "third" => fightcore::camera::CamMode::Third,
                    _ => fightcore::camera::CamMode::Side,
                };
            }
            if mode == "fight" {
                g.autoplay = Some(Ai::new(0.75, 5));
                g.start_fight(opp);
                if let Ok(c) = std::env::var("CAM") {
                    g.cam.mode = match c.as_str() {
                        "iso" => fightcore::camera::CamMode::Iso,
                        "third" => fightcore::camera::CamMode::Third,
                        _ => fightcore::camera::CamMode::Side,
                    };
                }
            } else {
                g.cam.screen_shift = 0.35;
                if mode == "cel" {
                    let k: i32 = std::env::var("CEL").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
                    g.preview_celebration(Some(k as u32));
                }
                if let Ok(items) = std::env::var("ITEMS") {
                    for id in items.split(',').filter_map(|x| x.parse::<u16>().ok()) {
                        g.profile.owned.push(id);
                        g.profile.equip(id);
                    }
                    g.sync_menu_fighter(true);
                }
                if let Ok(f) = std::env::var("FIGHTER") {
                    g.preview_fighter = f.parse().ok();
                }
            }
            let skip: f32 = std::env::var("SKIP").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let mut t = 0.0;
            while t < skip {
                g.update(1.0 / 60.0);
                t += 1.0 / 60.0;
            }
            let frames = (secs * 30.0) as usize;
            let mut r = Raster::new(w * SS, h * SS);
            let mut sink: Box<dyn Write> = if out == "-" { Box::new(std::io::stdout().lock()) } else { Box::new(std::fs::File::create(&out).unwrap()) };
            let single = out.ends_with(".ppm");
            for fi in 0..frames.max(1) {
                g.update(1.0 / 60.0);
                g.update(1.0 / 60.0);
                if single && fi + 1 < frames {
                    continue;
                }
                r.clear();
                r.draw(&g, flash(&g));
                if std::env::var("HUD").is_ok() {
                    r.hud(&g);
                }
                let px = r.downsample(SS);
                if single {
                    write!(sink, "P6\n{} {}\n255\n", w, h).unwrap();
                }
                sink.write_all(&px).unwrap();
                if std::env::var("VERBOSE").is_ok() {
                    if let Some(f) = &g.fight {
                        eprintln!("t={:.2} phase={:?} tris={} p_state={:?} e_state={:?} p_act={:?}", fi as f32 / 30.0, f.phase, g.mesh.v.len() / 3, f.f[0].state, f.f[1].state, f.f[0].action);
                    }
                }
            }
        }
        "find" => {
            let opp: usize = std::env::var("OPP").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
            for seed in 1..60u64 {
                let mut g = Game::new(seed);
                g.profile.ladder = 7;
                g.autoplay = Some(Ai::new(0.75, 5));
                g.start_fight(opp);
                let mut t = 0.0;
                while g.result.valid == 0 && t < 120.0 {
                    g.update(1.0 / 60.0);
                    t += 1.0 / 60.0;
                }
                let r = g.result;
                if r.won == 1 && r.method != 3 && t < 60.0 {
                    println!("seed {seed}: won method {} at {:.1}s", r.method, t);
                }
            }
        }
        _ => eprintln!("unknown mode"),
    }
    let _ = Phase::Over;
}
