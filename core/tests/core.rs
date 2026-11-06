use fightcore::ai::Ai;
use fightcore::catalog::*;
use fightcore::fighter::*;
use fightcore::game::*;
use fightcore::math::*;
use fightcore::profile::Profile;
use std::mem::size_of;

#[test]
fn ffi_struct_sizes_match_c_header() {
    // values printed by ctest/abi_test.c compiled against include/fightcore.h
    assert_eq!(size_of::<fightcore::mesh::Vertex>(), 40);
    assert_eq!(size_of::<fightcore::FcUniforms>(), 112);
    assert_eq!(size_of::<fightcore::FcFighterHud>(), 56);
    assert_eq!(size_of::<fightcore::FcHud>(), 160);
    assert_eq!(size_of::<fightcore::fight::Event>(), 16);
    assert_eq!(size_of::<FightResult>(), 48);
    assert_eq!(size_of::<fightcore::FcItem>(), 64);
    assert_eq!(size_of::<fightcore::FcFighterInfo>(), 64);
    assert_eq!(size_of::<fightcore::FcCelebrationInfo>(), 24);
    assert_eq!(size_of::<fightcore::FcOpponentInfo>(), 40);
    assert_eq!(size_of::<fightcore::FcProfile>(), 72);
}

#[test]
fn idle_fighter_stays_upright() {
    for a in ARCHETYPES {
        let mut g = Game::new(3);
        g.shadowbox = false;
        g.preview_fighter = Some(a.id);
        for _ in 0..600 {
            g.update(1.0 / 60.0);
        }
        let f = &g.menu_fighter;
        assert!(f.is_finite());
        let head = f.pos(HEAD).y;
        assert!(head > 1.35 * a.height, "{:?} head at {head}", a.name);
        assert!(f.pos(PELVIS).flat().len() < 0.6, "drifted");
    }
}

#[test]
fn ai_fights_finish_without_nans() {
    for i in 0..4 {
        let mut g = Game::new(100 + i);
        g.profile.ladder = 7;
        g.autoplay = Some(Ai::new(0.7, i));
        assert!(g.start_fight((i * 2) as usize));
        let mut t = 0.0;
        while g.result.valid == 0 {
            g.update(1.0 / 60.0);
            t += 1.0 / 60.0;
            let f = g.fight.as_ref().unwrap();
            assert!(f.f[0].is_finite() && f.f[1].is_finite());
            assert!(t < 300.0, "fight never ended");
        }
        assert!(g.profile.stats.fights == 1);
        assert!(!g.mesh.v.is_empty());
    }
}

#[test]
fn all_celebrations_are_stable() {
    for c in 0..CELEBRATIONS.len() as u32 {
        let mut g = Game::new(5);
        g.preview_celebration(Some(c));
        for _ in 0..400 {
            g.update(1.0 / 60.0);
            assert!(g.menu_fighter.is_finite());
        }
        let p = g.menu_fighter.pos(PELVIS);
        assert!(p.y > 0.0 && p.flat().len() < 2.5, "celebration {c} wandered: {p:?}");
    }
}

#[test]
fn profile_roundtrip_and_shop_rules() {
    let mut p = Profile::default();
    assert!(!p.buy(54)); // crown too expensive
    p.coins = 10_000;
    assert!(p.buy(54) && p.equip(54));
    assert!(p.buy_fighter(3));
    assert!(p.buy_celebration(4));
    p.fighter = 3;
    p.celebration = 4;
    let r = p.record_fight(0, true, 1, 30, 20, 10, 1, 2, 120.0, false);
    assert!(r.coins > 0 && r.unlocked_opponent);
    let q = Profile::deserialize(&p.serialize()).unwrap();
    assert_eq!(p, q);
    assert!(Profile::deserialize("garbage").is_none());
}

#[test]
fn every_cosmetic_renders() {
    let mut g = Game::new(9);
    g.profile.coins = 1_000_000;
    for it in ITEMS {
        g.profile.buy(it.id);
        g.profile.equip(it.id);
        g.sync_menu_fighter(false);
        g.update(1.0 / 60.0);
        for v in &g.mesh.v {
            assert!(v.pos.iter().all(|x| x.is_finite()) && v.nrm.iter().all(|x| x.is_finite()));
        }
    }
}

#[test]
fn punch_lands_and_hurts() {
    let mut g = Game::new(1);
    assert!(g.start_fight(0));
    // skip intro
    for _ in 0..140 {
        g.update(1.0 / 60.0);
    }
    let before = g.fight.as_ref().unwrap().f[1].hp_head + g.fight.as_ref().unwrap().f[1].hp_body;
    for i in 0..600 {
        let f = g.fight.as_ref().unwrap();
        let d = (f.f[1].root - f.f[0].root).flat().len();
        g.stick = if d > 1.0 { (1.0, 0.0) } else { (0.0, 0.0) };
        if i % 25 == 0 && d < 1.3 {
            g.button(BTN_PUNCH, true);
            g.button(BTN_PUNCH, false);
        }
        g.update(1.0 / 60.0);
    }
    let f = g.fight.as_ref().unwrap();
    let after = f.f[1].hp_head + f.f[1].hp_body;
    assert!(f.f[0].stats.landed > 0, "no punches landed");
    assert!(after < before);
    let _ = v3(0.0, 0.0, 0.0);
}
