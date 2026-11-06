// C-side ABI smoke test: exercises the full public API through the header.
#include "fightcore.h"
#include <stdio.h>
#include <string.h>
#include <assert.h>

int main(void) {
    printf("sizes: vertex=%zu uniforms=%zu hudf=%zu hud=%zu event=%zu result=%zu item=%zu finfo=%zu cel=%zu opp=%zu prof=%zu\n",
        sizeof(FcVertex), sizeof(FcUniforms), sizeof(FcFighterHud), sizeof(FcHud), sizeof(FcEvent), sizeof(FcResult),
        sizeof(FcItem), sizeof(FcFighterInfo), sizeof(FcCelebrationInfo), sizeof(FcOpponentInfo), sizeof(FcProfile));
    FcGame *g = fc_new(7);
    fc_resize(g, 2532, 1170);
    for (int i = 0; i < 60; i++) fc_update(g, 1.0f/60.0f);
    uint32_t n = 0; const FcVertex *v = fc_vertices(g, &n);
    printf("menu verts=%u first=(%.2f %.2f %.2f)\n", n, v[0].pos[0], v[0].pos[1], v[0].pos[2]);
    FcItem it; int owned = 0;
    for (uint32_t i = 0; i < fc_item_count(); i++) { fc_item_info(g, i, &it); owned += it.owned; }
    printf("items=%u owned=%d slot0=%s\n", fc_item_count(), owned, fc_slot_name(0));
    fc_add_coins(g, 5000);
    assert(fc_buy_item(g, 54)); // crown
    FcFighterInfo fi; fc_fighter_info(g, 2, &fi); printf("fighter2=%s (%s) finisher=%s\n", fi.name, fi.style, fi.finisher);
    assert(fc_buy_fighter(g, 2) && fc_select_fighter(g, 2));
    assert(fc_buy_celebration(g, 1) && fc_select_celebration(g, 1));
    assert(fc_start_fight(g, 0));
    assert(!fc_start_fight(g, 5)); // locked
    FcHud h; FcEvent e; int hits = 0;
    for (int i = 0; i < 60*20; i++) {
        float t = i / 60.0f;
        fc_set_stick(g, (i/90)%2 ? 0.8f : 0.0f, 0.0f);
        if (i % 20 == 0) { fc_button(g, FC_BTN_PUNCH, true); fc_button(g, FC_BTN_PUNCH, false); }
        if (i % 97 == 0) { fc_button(g, FC_BTN_KICK, true); fc_button(g, FC_BTN_KICK, false); }
        fc_update(g, 1.0f/60.0f);
        while (fc_poll_event(g, &e)) if (e.kind == FC_EV_HIT) hits++;
        (void)t;
    }
    fc_hud(g, &h);
    printf("fight: scene=%u phase=%u round=%u time=%.1f P(head %.2f stam %.2f sp %.2f %s) E(head %.2f %s) hits=%d\n",
        h.scene, h.phase, h.round, h.round_time, h.player.head, h.player.stamina, h.player.special, h.player.name, h.enemy.head, h.enemy.name, hits);
    v = fc_vertices(g, &n); printf("fight verts=%u (%.0f KB)\n", n, n*40/1024.0);
    FcUniforms u; fc_uniforms(g, &u); printf("cam=(%.2f %.2f %.2f)\n", u.cam_pos[0], u.cam_pos[1], u.cam_pos[2]);
    const char *s = fc_save(g); char buf[4096]; strncpy(buf, s, sizeof buf - 1);
    printf("save:\n%s", buf);
    fc_reset_profile(g);
    assert(fc_load(g, buf));
    FcProfile p; fc_profile(g, &p); printf("after load coins=%u fighter=%u cel=%u\n", p.coins, p.fighter, p.celebration);
    assert(p.fighter == 2 && p.celebration == 1);
    fc_go_menu(g); fc_update(g, 0.016f);
    fc_free(g);
    printf("ABI OK\n");
    return 0;
}
