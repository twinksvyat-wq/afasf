// WHITEOUT FC — C interface to the Rust game core (libfightcore.a)
// All functions must be called from one thread (the iOS main thread).
#ifndef FIGHTCORE_H
#define FIGHTCORE_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FcGame FcGame;

// Interleaved vertex: 40 bytes. col.a = 1 lit, 0 unlit.
typedef struct {
    float pos[3];
    float nrm[3];
    float col[4];
} FcVertex;

typedef struct {
    float view_proj[16];   // column-major, clip z in [0,1]
    float cam_pos[4];
    float light_dir[4];
    float params[4];       // time, fog_start, fog_end, flash
} FcUniforms;

typedef struct {
    float head, body, legs, stamina, special, stun;   // 0..1
    uint32_t knockdowns, rocked, down, grabbed, combo;
    const char *name;
} FcFighterHud;

typedef struct {
    uint32_t scene;        // 0 menu, 1 fight
    uint32_t phase;        // 0 intro, 1 fight, 2 round end, 3 ko, 4 celebrate, 5 over
    uint32_t round;
    float round_time;
    uint32_t banner;       // 0 none, 1 round N, 2 fight!, 3 KO, 4 TKO, 5 decision, 6 knockdown, 8 counter, 9 time
    float banner_t;
    int32_t winner;        // -1 none, 0 player, 1 enemy
    uint32_t method;       // 0 none, 1 KO, 2 TKO, 3 decision, 4 finisher
    uint32_t camera_mode;  // 0 side, 1 iso, 2 third person
    uint32_t finisher_ready; // 0 no, 1 special ready, 2 finisher ready (opponent rocked)
    float slowmo;
    FcFighterHud player;
    FcFighterHud enemy;
} FcHud;

enum {
    FC_EV_HIT = 1, FC_EV_BLOCK = 2, FC_EV_PARRY = 3, FC_EV_KNOCKDOWN = 4, FC_EV_KO = 5,
    FC_EV_ROUND_START = 6, FC_EV_ROUND_END = 7, FC_EV_FIGHT_END = 8, FC_EV_THROW = 9,
    FC_EV_THUD = 10, FC_EV_FINISHER = 12, FC_EV_COUNTER = 13
};

typedef struct {
    uint32_t kind;
    uint32_t fighter;  // for hits: the fighter who got hit (0 player, 1 enemy)
    float a;           // damage / strength
    float b;           // body part (0 head, 1 body, 2 arm, 3 leg)
} FcEvent;

typedef struct {
    uint32_t valid, won, method, coins, landed, thrown, knockdowns;
    float damage;
    uint32_t finisher, unlocked, opponent, rounds;
} FcResult;

typedef struct {
    uint32_t id, slot, style, price, owned, equipped;
    const char *name;
    float color[4];
    float color2[4];
} FcItem;

typedef struct {
    uint32_t index, price, owned, selected;
    const char *name;
    const char *style;
    const char *finisher;
    float power, speed, kick, grapple, chin;
} FcFighterInfo;

typedef struct {
    uint32_t index, price, owned, selected;
    const char *name;
} FcCelebrationInfo;

typedef struct {
    uint32_t index, archetype, reward, unlocked, beaten;
    float skill;
    const char *name;
    const char *nick;
} FcOpponentInfo;

typedef struct {
    uint32_t coins, fights, wins, losses, kos, finishers, knockdowns, kod_by;
    uint32_t punches, kicks, throws, streak, best_streak, damage, coins_total;
    uint32_t ladder, fighter, celebration;
} FcProfile;

enum { FC_BTN_PUNCH = 0, FC_BTN_KICK = 1, FC_BTN_GRAB = 2, FC_BTN_BLOCK = 3, FC_BTN_SPECIAL = 4 };

// lifecycle & frame
FcGame *fc_new(uint64_t seed);
void fc_free(FcGame *g);
void fc_resize(FcGame *g, float width, float height);
void fc_update(FcGame *g, float dt);
const FcVertex *fc_vertices(FcGame *g, uint32_t *count);
void fc_uniforms(FcGame *g, FcUniforms *out);
void fc_hud(FcGame *g, FcHud *out);
bool fc_poll_event(FcGame *g, FcEvent *out);

// input
void fc_set_stick(FcGame *g, float x, float y);
void fc_button(FcGame *g, uint32_t button, bool down);
void fc_camera_orbit(FcGame *g, float dx, float dy);
void fc_camera_zoom(FcGame *g, float scale);
uint32_t fc_camera_cycle(FcGame *g);
void fc_camera_reset(FcGame *g);
void fc_menu_shift(FcGame *g, float x);
void fc_menu_poke(FcGame *g, float ndc_x, float ndc_y);

// flow
bool fc_start_fight(FcGame *g, uint32_t opponent);
void fc_go_menu(FcGame *g);
void fc_set_paused(FcGame *g, bool paused);
void fc_result(FcGame *g, FcResult *out);

// shop & progression
uint32_t fc_slot_count(void);
const char *fc_slot_name(uint32_t slot);
uint32_t fc_item_count(void);
bool fc_item_info(FcGame *g, uint32_t index, FcItem *out);
bool fc_buy_item(FcGame *g, uint32_t id);
bool fc_equip_item(FcGame *g, uint32_t id);
void fc_preview_item(FcGame *g, int32_t id);

uint32_t fc_fighter_count(void);
bool fc_fighter_info(FcGame *g, uint32_t index, FcFighterInfo *out);
bool fc_buy_fighter(FcGame *g, uint32_t index);
bool fc_select_fighter(FcGame *g, uint32_t index);
void fc_preview_fighter(FcGame *g, int32_t index);

uint32_t fc_celebration_count(void);
bool fc_celebration_info(FcGame *g, uint32_t index, FcCelebrationInfo *out);
bool fc_buy_celebration(FcGame *g, uint32_t index);
bool fc_select_celebration(FcGame *g, uint32_t index);
void fc_preview_celebration(FcGame *g, int32_t index);

uint32_t fc_opponent_count(void);
bool fc_opponent_info(FcGame *g, uint32_t index, FcOpponentInfo *out);
void fc_preview_opponent(FcGame *g, int32_t index);

void fc_profile(FcGame *g, FcProfile *out);
const char *fc_save(FcGame *g);
bool fc_load(FcGame *g, const char *text);
void fc_reset_profile(FcGame *g);
void fc_add_coins(FcGame *g, uint32_t n);

#ifdef __cplusplus
}
#endif
#endif
