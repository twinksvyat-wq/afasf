//! WHITEOUT FC — physics fighting game core.
//! Exposes a C ABI (see include/fightcore.h) consumed by the iOS Swift/Metal shell.
#![allow(clippy::missing_safety_doc)]

pub mod ai;
pub mod camera;
pub mod catalog;
pub mod fight;
pub mod fighter;
pub mod fx;
pub mod game;
pub mod math;
pub mod mesh;
pub mod profile;
pub mod render;

use catalog::*;
use fight::{Event, Phase};
use fighter::*;
use game::*;
use std::ffi::{c_char, CStr};

#[repr(C)]
pub struct FcUniforms {
    pub view_proj: [f32; 16],
    pub cam_pos: [f32; 4],
    pub light_dir: [f32; 4],
    /// time, fog_start, fog_end, flash
    pub params: [f32; 4],
}

#[repr(C)]
#[derive(Default)]
pub struct FcFighterHud {
    pub head: f32,
    pub body: f32,
    pub legs: f32,
    pub stamina: f32,
    pub special: f32,
    pub stun: f32,
    pub knockdowns: u32,
    pub rocked: u32,
    pub down: u32,
    pub grabbed: u32,
    pub combo: u32,
    pub name: *const c_char,
}

#[repr(C)]
pub struct FcHud {
    /// 0 menu, 1 fight
    pub scene: u32,
    /// 0 intro, 1 fight, 2 round end, 3 ko, 4 celebrate, 5 over
    pub phase: u32,
    pub round: u32,
    pub round_time: f32,
    /// 0 none, 1 round N, 2 fight!, 3 KO, 4 TKO, 5 decision, 6 knockdown, 8 counter, 9 time
    pub banner: u32,
    pub banner_t: f32,
    pub winner: i32,
    pub method: u32,
    pub camera_mode: u32,
    pub finisher_ready: u32,
    pub slowmo: f32,
    pub player: FcFighterHud,
    pub enemy: FcFighterHud,
}

#[repr(C)]
pub struct FcItem {
    pub id: u32,
    pub slot: u32,
    pub style: u32,
    pub price: u32,
    pub owned: u32,
    pub equipped: u32,
    pub name: *const c_char,
    pub color: [f32; 4],
    pub color2: [f32; 4],
}

#[repr(C)]
pub struct FcFighterInfo {
    pub index: u32,
    pub price: u32,
    pub owned: u32,
    pub selected: u32,
    pub name: *const c_char,
    pub style: *const c_char,
    pub finisher: *const c_char,
    pub power: f32,
    pub speed: f32,
    pub kick: f32,
    pub grapple: f32,
    pub chin: f32,
}

#[repr(C)]
pub struct FcCelebrationInfo {
    pub index: u32,
    pub price: u32,
    pub owned: u32,
    pub selected: u32,
    pub name: *const c_char,
}

#[repr(C)]
pub struct FcOpponentInfo {
    pub index: u32,
    pub archetype: u32,
    pub reward: u32,
    pub unlocked: u32,
    pub beaten: u32,
    pub skill: f32,
    pub name: *const c_char,
    pub nick: *const c_char,
}

#[repr(C)]
pub struct FcProfile {
    pub coins: u32,
    pub fights: u32,
    pub wins: u32,
    pub losses: u32,
    pub kos: u32,
    pub finishers: u32,
    pub knockdowns: u32,
    pub kod_by: u32,
    pub punches: u32,
    pub kicks: u32,
    pub throws: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub damage: u32,
    pub coins_total: u32,
    pub ladder: u32,
    pub fighter: u32,
    pub celebration: u32,
}

#[inline]
unsafe fn g<'a>(p: *mut Game) -> Option<&'a mut Game> {
    p.as_mut()
}

#[no_mangle]
pub extern "C" fn fc_new(seed: u64) -> *mut Game {
    Box::into_raw(Box::new(Game::new(seed)))
}
#[no_mangle]
pub unsafe extern "C" fn fc_free(p: *mut Game) {
    if !p.is_null() {
        drop(Box::from_raw(p));
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_resize(p: *mut Game, w: f32, h: f32) {
    if let Some(g) = g(p) {
        g.resize(w, h)
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_update(p: *mut Game, dt: f32) {
    if let Some(g) = g(p) {
        g.update(dt)
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_set_stick(p: *mut Game, x: f32, y: f32) {
    if let Some(g) = g(p) {
        g.stick = (math::clampf(x, -1.0, 1.0), math::clampf(y, -1.0, 1.0));
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_button(p: *mut Game, button: u32, down: bool) {
    if let Some(g) = g(p) {
        g.button(button, down)
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_camera_orbit(p: *mut Game, dx: f32, dy: f32) {
    if let Some(g) = g(p) {
        g.cam.orbit(dx, dy)
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_camera_zoom(p: *mut Game, scale: f32) {
    if let Some(g) = g(p) {
        g.cam.zoom = math::clampf(g.cam.zoom * scale, 0.5, 2.2);
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_camera_cycle(p: *mut Game) -> u32 {
    match g(p) {
        Some(g) => {
            g.cam.cycle();
            g.cam.mode as u32
        }
        None => 0,
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_camera_reset(p: *mut Game) {
    if let Some(g) = g(p) {
        g.cam.orbit_yaw = if g.scene == Scene::Menu { 0.35 } else { 0.0 };
        g.cam.orbit_pitch = 0.0;
        g.cam.zoom = 1.0;
    }
}
/// Horizontal screen offset for the menu showcase (-1..1, fraction of half-screen).
#[no_mangle]
pub unsafe extern "C" fn fc_menu_shift(p: *mut Game, x: f32) {
    if let Some(g) = g(p) {
        g.cam.screen_shift = math::clampf(x, -1.0, 1.0);
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_menu_poke(p: *mut Game, x: f32, y: f32) {
    if let Some(g) = g(p) {
        g.poke(x, y)
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_vertices(p: *mut Game, count: *mut u32) -> *const mesh::Vertex {
    match g(p) {
        Some(g) => {
            if !count.is_null() {
                *count = g.mesh.v.len() as u32;
            }
            g.mesh.v.as_ptr()
        }
        None => std::ptr::null(),
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_uniforms(p: *mut Game, out: *mut FcUniforms) {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return };
    let l = math::v3(-0.45, 0.85, 0.4).norm();
    let flash = match &g.fight {
        Some(f) if g.scene == Scene::Fight => f.fx.flash,
        _ => 0.0,
    };
    *o = FcUniforms { view_proj: g.view_proj.0, cam_pos: [g.eye.x, g.eye.y, g.eye.z, 1.0], light_dir: [l.x, l.y, l.z, 0.0], params: [g.time, 18.0, 90.0, flash] };
}

fn fighter_hud(f: &Fighter, name: *const c_char) -> FcFighterHud {
    FcFighterHud {
        head: (f.hp_head / 100.0).clamp(0.0, 1.0),
        body: (f.hp_body / 100.0).clamp(0.0, 1.0),
        legs: (f.hp_legs / 100.0).clamp(0.0, 1.0),
        stamina: (f.stamina / 100.0).clamp(0.0, 1.0),
        special: (f.special / 100.0).clamp(0.0, 1.0),
        stun: (f.stun / 100.0).clamp(0.0, 1.0),
        knockdowns: f.knockdowns,
        rocked: f.rocked() as u32,
        down: f.is_down() as u32,
        grabbed: f.grabbed_by_other as u32,
        combo: f.combo as u32,
        name,
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc_hud(p: *mut Game, out: *mut FcHud) {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return };
    let mut h = FcHud {
        scene: 0,
        phase: 0,
        round: 0,
        round_time: 0.0,
        banner: 0,
        banner_t: 0.0,
        winner: -1,
        method: 0,
        camera_mode: g.cam.mode as u32,
        finisher_ready: 0,
        slowmo: 1.0,
        player: FcFighterHud { name: std::ptr::null(), ..Default::default() },
        enemy: FcFighterHud { name: std::ptr::null(), ..Default::default() },
    };
    if let (Scene::Fight, Some(f)) = (g.scene, &g.fight) {
        h.scene = 1;
        h.phase = match f.phase {
            Phase::Intro { .. } => 0,
            Phase::Fight => 1,
            Phase::RoundEnd { .. } => 2,
            Phase::Ko { .. } => 3,
            Phase::Celebrate { .. } => 4,
            Phase::Over => 5,
        };
        h.round = f.round;
        h.round_time = f.round_t;
        h.banner = f.banner;
        h.banner_t = f.banner_t;
        h.winner = f.winner.map(|w| w as i32).unwrap_or(-1);
        h.method = f.method as u32;
        h.slowmo = f.slowmo;
        h.finisher_ready = (f.f[0].special >= 100.0) as u32 * if f.f[1].rocked() { 2 } else { 1 };
        let pn = ARCHETYPES[g.profile.fighter as usize].name.as_ptr();
        let en = OPPONENTS[f.opp_index].name.as_ptr();
        h.player = fighter_hud(&f.f[0], pn);
        h.enemy = fighter_hud(&f.f[1], en);
    }
    *o = h;
}

#[no_mangle]
pub unsafe extern "C" fn fc_poll_event(p: *mut Game, out: *mut Event) -> bool {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return false };
    match g.events.pop_front() {
        Some(e) => {
            *o = e;
            true
        }
        None => false,
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_start_fight(p: *mut Game, opponent: u32) -> bool {
    g(p).map(|g| g.start_fight(opponent as usize)).unwrap_or(false)
}
#[no_mangle]
pub unsafe extern "C" fn fc_go_menu(p: *mut Game) {
    if let Some(g) = g(p) {
        g.go_menu()
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_set_paused(p: *mut Game, paused: bool) {
    if let Some(g) = g(p) {
        g.paused = paused
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_result(p: *mut Game, out: *mut FightResult) {
    if let (Some(g), Some(o)) = (g(p), out.as_mut()) {
        *o = g.result;
    }
}

// ---------------------------------------------------------------- shop / catalog
#[no_mangle]
pub extern "C" fn fc_slot_count() -> u32 {
    NSLOT as u32
}
#[no_mangle]
pub extern "C" fn fc_slot_name(slot: u32) -> *const c_char {
    SLOT_NAMES.get(slot as usize).map(|s| s.as_ptr()).unwrap_or(std::ptr::null())
}
#[no_mangle]
pub extern "C" fn fc_item_count() -> u32 {
    ITEMS.len() as u32
}
#[no_mangle]
pub unsafe extern "C" fn fc_item_info(p: *mut Game, index: u32, out: *mut FcItem) -> bool {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return false };
    let Some(it) = ITEMS.get(index as usize) else { return false };
    *o = FcItem {
        id: it.id as u32,
        slot: it.slot as u32,
        style: it.style as u32,
        price: it.price,
        owned: g.profile.owns(it.id) as u32,
        equipped: (g.profile.look.items[it.slot as usize] == it.id) as u32,
        name: it.name.as_ptr(),
        color: [it.c1[0], it.c1[1], it.c1[2], 1.0],
        color2: [it.c2[0], it.c2[1], it.c2[2], 1.0],
    };
    true
}
#[no_mangle]
pub unsafe extern "C" fn fc_buy_item(p: *mut Game, id: u32) -> bool {
    g(p).map(|g| {
        let ok = g.profile.buy(id as u16);
        if ok {
            g.profile.equip(id as u16);
            g.sync_menu_fighter(false);
        }
        ok
    })
    .unwrap_or(false)
}
#[no_mangle]
pub unsafe extern "C" fn fc_equip_item(p: *mut Game, id: u32) -> bool {
    g(p).map(|g| {
        let ok = g.profile.equip(id as u16);
        g.sync_menu_fighter(false);
        ok
    })
    .unwrap_or(false)
}
/// Try an item on the menu fighter without buying (-1 to clear).
#[no_mangle]
pub unsafe extern "C" fn fc_preview_item(p: *mut Game, id: i32) {
    if let Some(g) = g(p) {
        g.preview_item = if id < 0 { None } else { Some(id as u16) };
        g.sync_menu_fighter(false);
    }
}

#[no_mangle]
pub extern "C" fn fc_fighter_count() -> u32 {
    ARCHETYPES.len() as u32
}
#[no_mangle]
pub unsafe extern "C" fn fc_fighter_info(p: *mut Game, index: u32, out: *mut FcFighterInfo) -> bool {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return false };
    let Some(a) = ARCHETYPES.get(index as usize) else { return false };
    *o = FcFighterInfo {
        index,
        price: a.price,
        owned: g.profile.fighters.contains(&index) as u32,
        selected: (g.profile.fighter == index) as u32,
        name: a.name.as_ptr(),
        style: a.style.as_ptr(),
        finisher: FINISHER_NAMES[a.finisher as usize].as_ptr(),
        power: a.power,
        speed: a.speed,
        kick: a.kick,
        grapple: a.grapple,
        chin: a.chin,
    };
    true
}
#[no_mangle]
pub unsafe extern "C" fn fc_buy_fighter(p: *mut Game, index: u32) -> bool {
    g(p).map(|g| g.profile.buy_fighter(index)).unwrap_or(false)
}
#[no_mangle]
pub unsafe extern "C" fn fc_select_fighter(p: *mut Game, index: u32) -> bool {
    g(p).map(|g| {
        if g.profile.fighters.contains(&index) {
            g.profile.fighter = index;
            g.preview_fighter = None;
            g.sync_menu_fighter(false);
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}
#[no_mangle]
pub unsafe extern "C" fn fc_preview_fighter(p: *mut Game, index: i32) {
    if let Some(g) = g(p) {
        g.preview_fighter = if index < 0 || index as usize >= ARCHETYPES.len() { None } else { Some(index as u32) };
        g.sync_menu_fighter(false);
    }
}

#[no_mangle]
pub extern "C" fn fc_celebration_count() -> u32 {
    CELEBRATIONS.len() as u32
}
#[no_mangle]
pub unsafe extern "C" fn fc_celebration_info(p: *mut Game, index: u32, out: *mut FcCelebrationInfo) -> bool {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return false };
    let Some(c) = CELEBRATIONS.get(index as usize) else { return false };
    *o = FcCelebrationInfo { index, price: c.price, owned: g.profile.celebrations.contains(&index) as u32, selected: (g.profile.celebration == index) as u32, name: c.name.as_ptr() };
    true
}
#[no_mangle]
pub unsafe extern "C" fn fc_buy_celebration(p: *mut Game, index: u32) -> bool {
    g(p).map(|g| g.profile.buy_celebration(index)).unwrap_or(false)
}
#[no_mangle]
pub unsafe extern "C" fn fc_select_celebration(p: *mut Game, index: u32) -> bool {
    g(p).map(|g| {
        if g.profile.celebrations.contains(&index) {
            g.profile.celebration = index;
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}
#[no_mangle]
pub unsafe extern "C" fn fc_preview_celebration(p: *mut Game, index: i32) {
    if let Some(g) = g(p) {
        g.preview_celebration(if index < 0 { None } else { Some(index as u32) });
    }
}

#[no_mangle]
pub extern "C" fn fc_opponent_count() -> u32 {
    OPPONENTS.len() as u32
}
#[no_mangle]
pub unsafe extern "C" fn fc_opponent_info(p: *mut Game, index: u32, out: *mut FcOpponentInfo) -> bool {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return false };
    let Some(op) = OPPONENTS.get(index as usize) else { return false };
    *o = FcOpponentInfo {
        index,
        archetype: op.archetype,
        reward: op.reward,
        unlocked: (index <= g.profile.ladder) as u32,
        beaten: (index < g.profile.ladder || (index == g.profile.ladder && index as usize == OPPONENTS.len() - 1 && g.profile.stats.wins > 0 && false)) as u32,
        skill: op.skill,
        name: op.name.as_ptr(),
        nick: op.nick.as_ptr(),
    };
    true
}
/// Show an opponent on the menu stage (-1 to return to the player's fighter).
#[no_mangle]
pub unsafe extern "C" fn fc_preview_opponent(p: *mut Game, index: i32) {
    if let Some(g) = g(p) {
        g.preview_opp = if index < 0 || index as usize >= OPPONENTS.len() { None } else { Some(index as usize) };
        g.sync_menu_fighter(false);
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc_profile(p: *mut Game, out: *mut FcProfile) {
    let (Some(g), Some(o)) = (g(p), out.as_mut()) else { return };
    let s = &g.profile.stats;
    *o = FcProfile {
        coins: g.profile.coins,
        fights: s.fights,
        wins: s.wins,
        losses: s.losses,
        kos: s.kos,
        finishers: s.finishers,
        knockdowns: s.knockdowns,
        kod_by: s.kod_by,
        punches: s.punches,
        kicks: s.kicks,
        throws: s.throws,
        streak: s.streak,
        best_streak: s.best_streak,
        damage: s.damage,
        coins_total: s.coins_total,
        ladder: g.profile.ladder,
        fighter: g.profile.fighter,
        celebration: g.profile.celebration,
    };
}
/// Serialized profile; pointer valid until the next fc_save call.
#[no_mangle]
pub unsafe extern "C" fn fc_save(p: *mut Game) -> *const c_char {
    g(p).map(|g| g.save()).unwrap_or(std::ptr::null())
}
#[no_mangle]
pub unsafe extern "C" fn fc_load(p: *mut Game, text: *const c_char) -> bool {
    let Some(g) = g(p) else { return false };
    if text.is_null() {
        return false;
    }
    match CStr::from_ptr(text).to_str() {
        Ok(s) => g.load(s),
        Err(_) => false,
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc_reset_profile(p: *mut Game) {
    if let Some(g) = g(p) {
        g.profile = profile::Profile::default();
        g.sync_menu_fighter(true);
    }
}
/// Debug/cheat helper for testing the shop.
#[no_mangle]
pub unsafe extern "C" fn fc_add_coins(p: *mut Game, n: u32) {
    if let Some(g) = g(p) {
        g.profile.coins += n;
    }
}
