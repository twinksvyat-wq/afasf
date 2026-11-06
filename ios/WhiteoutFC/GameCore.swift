import Foundation
import Combine
import FightCore

/// Event kinds (mirror FC_EV_* in fightcore.h)
enum Ev {
    static let hit: UInt32 = 1, block: UInt32 = 2, parry: UInt32 = 3, knockdown: UInt32 = 4, ko: UInt32 = 5
    static let roundStart: UInt32 = 6, roundEnd: UInt32 = 7, fightEnd: UInt32 = 8, thrown: UInt32 = 9
    static let thud: UInt32 = 10, finisher: UInt32 = 12, counter: UInt32 = 13
}

/// Buttons (mirror FC_BTN_* in fightcore.h)
enum Btn {
    static let punch: Int32 = 0, kick: Int32 = 1, grab: Int32 = 2, block: Int32 = 3, special: Int32 = 4
}

enum Screen: Equatable {
    case menu, ladder, shop, fighters, celebrations, stats, fight
}

struct FighterBars: Equatable {
    var head: Float = 1, body: Float = 1, legs: Float = 1
    var stamina: Float = 1, special: Float = 0, stun: Float = 0
    var knockdowns: UInt32 = 0
    var rocked = false, down = false, grabbed = false
    var combo: UInt32 = 0
}

struct HudState: Equatable {
    var phase: UInt32 = 0
    var round: UInt32 = 1
    var time: Float = 0
    var banner: UInt32 = 0
    var bannerT: Float = 0
    var winner: Int32 = -1
    var method: UInt32 = 0
    var camMode: UInt32 = 0
    var finisherReady: UInt32 = 0
    var player = FighterBars()
    var enemy = FighterBars()
}

struct ShopItem: Identifiable, Equatable {
    let id: UInt32
    let slot: UInt32
    let style: UInt32
    let price: UInt32
    let owned: Bool
    let equipped: Bool
    let name: String
    let r: Double, g: Double, b: Double
    let r2: Double, g2: Double, b2: Double
}

struct FighterCard: Identifiable, Equatable {
    var id: UInt32 { index }
    let index: UInt32
    let price: UInt32
    let owned: Bool
    let selected: Bool
    let name: String
    let style: String
    let finisher: String
    let power: Float, speed: Float, kick: Float, grapple: Float, chin: Float
}

struct CelebrationCard: Identifiable, Equatable {
    var id: UInt32 { index }
    let index: UInt32
    let price: UInt32
    let owned: Bool
    let selected: Bool
    let name: String
}

struct OpponentCard: Identifiable, Equatable {
    var id: UInt32 { index }
    let index: UInt32
    let archetype: UInt32
    let reward: UInt32
    let unlocked: Bool
    let beaten: Bool
    let skill: Float
    let name: String
    let nick: String
}

/// Swift facade over the Rust core. Everything runs on the main thread.
final class GameCore: ObservableObject {
    let g: OpaquePointer
    @Published var screen: Screen = .menu {
        didSet { screenChanged(from: oldValue) }
    }
    @Published private(set) var hud = HudState()
    @Published private(set) var profile = FcProfile()
    @Published private(set) var result: FcResult?
    @Published var paused = false {
        didSet { fc_set_paused(g, paused) }
    }
    @Published private(set) var playerName = ""
    @Published private(set) var enemyName = ""
    @Published private(set) var hitFlash: Int = 0
    @Published private(set) var lastOpponent: UInt32 = 0
    @Published var catalogRevision = 0

    let haptics = Haptics()
    let sound = SoundEngine()
    private var frame = 0
    private static let saveKey = "whiteout.profile.v1"

    init() {
        g = fc_new(UInt64(Date().timeIntervalSince1970 * 1000))
        if let saved = UserDefaults.standard.string(forKey: Self.saveKey) {
            saved.withCString { _ = fc_load(g, $0) }
        }
        refreshProfile()
        fc_menu_shift(g, 0.42)
    }

    deinit { fc_free(g) }

    // MARK: frame

    func tick(dt: Float) {
        fc_update(g, dt)
        var e = FcEvent()
        while fc_poll_event(g, &e) { handle(e) }
        frame += 1
        if frame % 2 == 0 { pullHud() }
    }

    private func pullHud() {
        var h = FcHud()
        fc_hud(g, &h)
        var s = HudState()
        s.phase = h.phase
        s.round = h.round
        s.time = h.round_time
        s.banner = h.banner
        s.bannerT = h.banner_t
        s.winner = h.winner
        s.method = h.method
        s.camMode = h.camera_mode
        s.finisherReady = h.finisher_ready
        s.player = bars(h.player)
        s.enemy = bars(h.enemy)
        if s != hud { hud = s }
        if h.scene == 1 && h.phase == 5 && result == nil {
            var r = FcResult()
            fc_result(g, &r)
            if r.valid != 0 {
                result = r
                save()
                refreshProfile()
            }
        }
    }

    private func bars(_ f: FcFighterHud) -> FighterBars {
        FighterBars(head: f.head, body: f.body, legs: f.legs, stamina: f.stamina, special: f.special, stun: f.stun,
                    knockdowns: f.knockdowns, rocked: f.rocked != 0, down: f.down != 0, grabbed: f.grabbed != 0, combo: f.combo)
    }

    private func handle(_ e: FcEvent) {
        switch e.kind {
        case Ev.hit:
            let playerHit = e.fighter == 0
            haptics.hit(strength: e.a, onPlayer: playerHit && screen == .fight)
            sound.hit(strength: e.a, kick: e.b == 3)
            if playerHit && screen == .fight { hitFlash += 1 }
        case Ev.block:
            haptics.block(); sound.block()
        case Ev.parry:
            haptics.block(); sound.parry()
        case Ev.knockdown, Ev.thrown:
            haptics.knockdown()
        case Ev.thud:
            sound.thud(strength: e.a)
        case Ev.ko:
            haptics.ko(); sound.bell(short: false)
        case Ev.roundStart, Ev.roundEnd:
            sound.bell(short: true)
        default:
            break
        }
    }

    // MARK: flow

    func startFight(_ opp: UInt32) {
        fc_preview_opponent(g, -1)
        guard fc_start_fight(g, opp) else { return }
        lastOpponent = opp
        result = nil
        paused = false
        var h = FcHud()
        fc_hud(g, &h)
        playerName = h.player.name.map { String(cString: $0) } ?? "ТЫ"
        enemyName = h.enemy.name.map { String(cString: $0) } ?? "???"
        screen = .fight
    }

    func nextOpponent() -> UInt32 {
        min(profile.ladder, fc_opponent_count() - 1)
    }

    func leaveFight() {
        fc_go_menu(g)
        result = nil
        paused = false
        screen = .menu
        save()
        refreshProfile()
    }

    private func screenChanged(from old: Screen) {
        guard old != screen else { return }
        // reset menu-stage previews when leaving a screen
        switch old {
        case .shop: fc_preview_item(g, -1)
        case .ladder: fc_preview_opponent(g, -1)
        case .fighters: fc_preview_fighter(g, -1)
        case .celebrations: fc_preview_celebration(g, -1)
        default: break
        }
        if screen != .fight {
            fc_menu_shift(g, screen == .menu ? 0.42 : 0.5)
        }
        fc_camera_reset(g)
    }

    // MARK: input passthrough

    func stick(_ x: Float, _ y: Float) { fc_set_stick(g, x, y) }
    func button(_ b: Int32, _ down: Bool) { fc_button(g, UInt32(b), down) }
    func orbit(_ dx: Float, _ dy: Float) { fc_camera_orbit(g, dx, dy) }
    func zoom(_ s: Float) { fc_camera_zoom(g, s) }
    func cycleCamera() { _ = fc_camera_cycle(g); haptics.tap() }
    func poke(_ x: Float, _ y: Float) { fc_menu_poke(g, x, y) }
    func resize(_ w: Float, _ h: Float) { fc_resize(g, w, h) }

    // MARK: profile & shop

    func refreshProfile() {
        var p = FcProfile()
        fc_profile(g, &p)
        profile = p
        catalogRevision += 1
    }

    func save() {
        if let p = fc_save(g) {
            UserDefaults.standard.set(String(cString: p), forKey: Self.saveKey)
        }
    }

    func slotNames() -> [String] {
        (0..<fc_slot_count()).map { String(cString: fc_slot_name($0)) }
    }

    func items(slot: UInt32) -> [ShopItem] {
        var out: [ShopItem] = []
        var it = FcItem()
        for i in 0..<fc_item_count() {
            guard fc_item_info(g, i, &it), it.slot == slot else { continue }
            out.append(ShopItem(id: it.id, slot: it.slot, style: it.style, price: it.price, owned: it.owned != 0, equipped: it.equipped != 0,
                                name: String(cString: it.name),
                                r: Double(it.color.0), g: Double(it.color.1), b: Double(it.color.2),
                                r2: Double(it.color2.0), g2: Double(it.color2.1), b2: Double(it.color2.2)))
        }
        return out
    }

    func preview(item: ShopItem?) { fc_preview_item(g, item.map { Int32($0.id) } ?? -1) }

    @discardableResult
    func buy(_ item: ShopItem) -> Bool {
        let ok = fc_buy_item(g, item.id)
        if ok { sound.coin(); haptics.success(); save() } else { haptics.error() }
        refreshProfile()
        return ok
    }

    func equip(_ item: ShopItem) {
        _ = fc_equip_item(g, item.id)
        haptics.tap()
        save()
        refreshProfile()
    }

    func fighters() -> [FighterCard] {
        var out: [FighterCard] = []
        var f = FcFighterInfo()
        for i in 0..<fc_fighter_count() {
            guard fc_fighter_info(g, i, &f) else { continue }
            out.append(FighterCard(index: f.index, price: f.price, owned: f.owned != 0, selected: f.selected != 0,
                                   name: String(cString: f.name), style: String(cString: f.style), finisher: String(cString: f.finisher),
                                   power: f.power, speed: f.speed, kick: f.kick, grapple: f.grapple, chin: f.chin))
        }
        return out
    }
    func previewFighter(_ i: Int32) { fc_preview_fighter(g, i) }
    func buyFighter(_ i: UInt32) {
        if fc_buy_fighter(g, i) { _ = fc_select_fighter(g, i); sound.coin(); haptics.success(); save() } else { haptics.error() }
        refreshProfile()
    }
    func selectFighter(_ i: UInt32) {
        _ = fc_select_fighter(g, i); haptics.tap(); save(); refreshProfile()
    }

    func celebrations() -> [CelebrationCard] {
        var out: [CelebrationCard] = []
        var c = FcCelebrationInfo()
        for i in 0..<fc_celebration_count() {
            guard fc_celebration_info(g, i, &c) else { continue }
            out.append(CelebrationCard(index: c.index, price: c.price, owned: c.owned != 0, selected: c.selected != 0, name: String(cString: c.name)))
        }
        return out
    }
    func previewCelebration(_ i: Int32) { fc_preview_celebration(g, i) }
    func buyCelebration(_ i: UInt32) {
        if fc_buy_celebration(g, i) { _ = fc_select_celebration(g, i); sound.coin(); haptics.success(); save() } else { haptics.error() }
        refreshProfile()
    }
    func selectCelebration(_ i: UInt32) {
        _ = fc_select_celebration(g, i); haptics.tap(); save(); refreshProfile()
    }

    func opponents() -> [OpponentCard] {
        var out: [OpponentCard] = []
        var o = FcOpponentInfo()
        for i in 0..<fc_opponent_count() {
            guard fc_opponent_info(g, i, &o) else { continue }
            out.append(OpponentCard(index: o.index, archetype: o.archetype, reward: o.reward, unlocked: o.unlocked != 0, beaten: o.beaten != 0,
                                    skill: o.skill, name: String(cString: o.name), nick: String(cString: o.nick)))
        }
        return out
    }
    func previewOpponent(_ i: Int32) { fc_preview_opponent(g, i) }

    func resetProgress() {
        fc_reset_profile(g)
        save()
        refreshProfile()
    }
}
