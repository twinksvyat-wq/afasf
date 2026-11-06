import SwiftUI

struct FightView: View {
    @EnvironmentObject var core: GameCore
    @State private var damageFlash = 0.0

    var body: some View {
        ZStack {
            TouchControls(core: core)
                .ignoresSafeArea()
                .opacity(core.hud.phase <= 2 ? 1 : 0)
                .allowsHitTesting(core.hud.phase <= 2 && !core.paused)

            // red vignette when the player gets hit
            RadialGradient(colors: [.clear, Theme.red.opacity(0.45)], center: .center, startRadius: 180, endRadius: 600)
                .ignoresSafeArea()
                .opacity(damageFlash)
                .allowsHitTesting(false)

            FightHUD()
                .allowsHitTesting(false)

            BannerView(banner: core.hud.banner, t: core.hud.bannerT, round: core.hud.round, winner: core.hud.winner)
                .allowsHitTesting(false)

            TopButtons()

            if core.paused {
                PauseView()
            }
            if let r = core.result {
                ResultsView(result: r)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(.spring(response: 0.4, dampingFraction: 0.85), value: core.result?.valid)
        .onChange(of: core.hitFlash) { _ in
            damageFlash = 1
            withAnimation(.easeOut(duration: 0.45)) { damageFlash = 0 }
        }
    }
}

struct TopButtons: View {
    @EnvironmentObject var core: GameCore
    var body: some View {
        GeometryReader { geo in
            VStack {
                HStack(spacing: 10) {
                    Spacer()
                    if core.result == nil {
                        CircleIcon(system: camIcon) { core.cycleCamera() }
                        CircleIcon(system: "pause.fill") { core.paused = true; core.haptics.tap() }
                    }
                }
                .padding(.trailing, max(16, geo.safeAreaInsets.trailing))
                .padding(.top, 54)
                Spacer()
            }
        }
    }
    var camIcon: String {
        switch core.hud.camMode {
        case 1: return "cube"
        case 2: return "person.fill.viewfinder"
        default: return "video.fill"
        }
    }
}

struct CircleIcon: View {
    let system: String
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            Image(systemName: system)
                .font(.system(size: 15, weight: .black))
                .foregroundColor(.white)
                .frame(width: 40, height: 40)
                .background(Circle().fill(Theme.ink.opacity(0.85)))
        }
        .buttonStyle(PressScale())
    }
}

// MARK: HUD

struct FightHUD: View {
    @EnvironmentObject var core: GameCore
    var body: some View {
        let h = core.hud
        GeometryReader { geo in
            VStack(spacing: 0) {
                HStack(alignment: .top, spacing: 14) {
                    FighterPanel(name: core.playerName, bars: h.player, mirrored: false)
                    VStack(spacing: 0) {
                        Text(timeText(h.time))
                            .font(Theme.display(34))
                            .monospacedDigit()
                            .foregroundColor(h.time < 10 && h.phase == 1 ? Theme.red : Theme.ink)
                        Text("РАУНД \(h.round)/3")
                            .font(Theme.label(10, .heavy))
                            .foregroundColor(.white)
                            .padding(.horizontal, 8).padding(.vertical, 2)
                            .background(Slant(skew: 5).fill(Theme.ink))
                    }
                    .frame(width: 92)
                    FighterPanel(name: core.enemyName, bars: h.enemy, mirrored: true)
                }
                .padding(.horizontal, max(16, max(geo.safeAreaInsets.leading, geo.safeAreaInsets.trailing)))
                .padding(.top, 10)
                Spacer()
                if h.player.grabbed && h.phase == 1 {
                    Text("ТЕБЯ ЗАХВАТИЛИ — ЖМИ БЛОК / ЗАХВАТ!")
                        .font(Theme.display(16))
                        .foregroundColor(.white)
                        .padding(.horizontal, 16).padding(.vertical, 6)
                        .background(Slant().fill(Theme.blue))
                        .padding(.bottom, 26)
                } else if h.finisherReady == 2 && h.phase == 1 {
                    Text("СОПЕРНИК ПОПЛЫЛ — ЖМИ ФИНИШ!")
                        .font(Theme.display(16))
                        .foregroundColor(Theme.ink)
                        .padding(.horizontal, 16).padding(.vertical, 6)
                        .background(Slant().fill(Theme.gold))
                        .padding(.bottom, 26)
                }
            }
        }
    }

    func timeText(_ t: Float) -> String {
        let s = max(0, Int(ceil(t)))
        return String(format: "%d:%02d", s / 60, s % 60)
    }
}

struct FighterPanel: View {
    let name: String
    let bars: FighterBars
    let mirrored: Bool

    var body: some View {
        VStack(alignment: mirrored ? .trailing : .leading, spacing: 4) {
            HStack(spacing: 6) {
                if mirrored { kdDots; Spacer(minLength: 0) }
                Text(name.uppercased())
                    .font(Theme.display(18))
                    .fontWidth(.condensed)
                    .foregroundColor(Theme.ink)
                if bars.rocked {
                    Text("ПОПЛЫЛ").font(Theme.label(9, .black)).foregroundColor(.white)
                        .padding(.horizontal, 5).padding(.vertical, 1)
                        .background(Capsule().fill(Theme.red))
                }
                if !mirrored { Spacer(minLength: 0); kdDots }
            }
            Bar(value: bars.head, color: Theme.red, height: 16, mirrored: mirrored, ghost: true)
            HStack(spacing: 4) {
                Bar(value: bars.body, color: Color(red: 0.85, green: 0.35, blue: 0.2), height: 6, mirrored: mirrored)
                Bar(value: bars.legs, color: Color(red: 0.75, green: 0.3, blue: 0.55), height: 6, mirrored: mirrored)
            }
            Bar(value: bars.stamina, color: Theme.blue, height: 5, mirrored: mirrored)
            Bar(value: bars.special, color: Theme.gold, height: 5, mirrored: mirrored)
                .opacity(bars.special >= 1 ? 1 : 0.85)
        }
        .frame(maxWidth: .infinity)
    }

    var kdDots: some View {
        HStack(spacing: 3) {
            ForEach(0..<3) { i in
                Circle()
                    .fill(UInt32(i) < bars.knockdowns ? Theme.red : Theme.ink.opacity(0.15))
                    .frame(width: 8, height: 8)
            }
        }
    }
}

/// Slanted health bar with a trailing "ghost" segment that drains slowly.
struct Bar: View {
    let value: Float
    let color: Color
    var height: CGFloat = 10
    var mirrored = false
    var ghost = false
    @State private var lag: Float = 1

    var body: some View {
        GeometryReader { g in
            let w = g.size.width
            ZStack(alignment: mirrored ? .trailing : .leading) {
                Slant(skew: height * 0.6).fill(Theme.ink.opacity(0.85))
                if ghost {
                    Slant(skew: height * 0.6).fill(Color.white.opacity(0.85))
                        .frame(width: w * CGFloat(max(0, lag)))
                }
                Slant(skew: height * 0.6).fill(color)
                    .frame(width: w * CGFloat(max(0, min(1, value))))
            }
            .scaleEffect(x: 1, y: 1)
        }
        .frame(height: height)
        .animation(.easeOut(duration: 0.12), value: value)
        .onChange(of: value) { v in
            withAnimation(.easeIn(duration: 0.8).delay(0.35)) { lag = v }
        }
        .onAppear { lag = value }
    }
}

// MARK: Banners

struct BannerView: View {
    let banner: UInt32
    let t: Float
    let round: UInt32
    let winner: Int32

    var body: some View {
        let (text, color, dur): (String, Color, Float) = {
            switch banner {
            case 1: return ("РАУНД \(round)", Theme.ink, 1.3)
            case 2: return ("БОЙ!", Theme.red, 0.9)
            case 3: return ("K.O.", Theme.red, 3.5)
            case 4: return ("Т.К.О.", Theme.red, 3.5)
            case 5: return ("РЕШЕНИЕ СУДЕЙ", Theme.ink, 3.0)
            case 6: return ("НОКДАУН!", Theme.red, 1.2)
            case 8: return ("КОНТР!", Theme.gold, 0.7)
            case 9: return ("ВРЕМЯ!", Theme.ink, 2.0)
            default: return ("", .clear, 0)
            }
        }()
        let visible = !text.isEmpty && t < dur
        let k = CGFloat(min(1, t / 0.12))
        ZStack {
            if visible {
                Text(text)
                    .font(Theme.display(banner == 3 || banner == 4 ? 120 : 72))
                    .fontWidth(.compressed)
                    .foregroundColor(color)
                    .shadow(color: .white, radius: 0, x: 4, y: 4)
                    .shadow(color: .black.opacity(0.25), radius: 10, y: 6)
                    .scaleEffect(2.2 - 1.2 * k)
                    .opacity(Double(k) * Double(min(1, (dur - t) / 0.25)))
                    .rotationEffect(.degrees(-6))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: Pause & results

struct PauseView: View {
    @EnvironmentObject var core: GameCore
    var body: some View {
        ZStack {
            Color.white.opacity(0.75).ignoresSafeArea()
            VStack(alignment: .leading, spacing: 12) {
                Text("ПАУЗА").font(Theme.display(56)).fontWidth(.compressed).foregroundColor(Theme.ink)
                SlantButton(title: "ПРОДОЛЖИТЬ", color: Theme.red, height: 52, size: 26) { core.paused = false }
                SlantButton(title: "СМЕНИТЬ КАМЕРУ", height: 44, size: 20) { core.cycleCamera() }
                SlantButton(title: "СДАТЬСЯ", color: Theme.muted, height: 44, size: 20) { core.leaveFight() }
                Text("Управление: стик — движение (вниз = апперкот/лоу-кик), УДАР — комбо джеб-кросс-хук-апперкот, вперёд+УДАР — кросс, назад+УДАР — хук. НОГА: вперёд — тип, назад — хай-кик, вблизи — колено. ЗАХВАТ — клинч, повторно — бросок. БЛОК вовремя — парирование. СУПЕР — фирменный приём, ФИНИШ — добивание поплывшего соперника.")
                    .font(Theme.label(11, .medium))
                    .foregroundColor(Theme.muted)
                    .frame(width: 420, alignment: .leading)
            }
            .frame(width: 440)
        }
    }
}

struct ResultsView: View {
    @EnvironmentObject var core: GameCore
    let result: FcResult

    var methodText: String {
        switch result.method {
        case 1: return "НОКАУТОМ"
        case 2: return "ТЕХНИЧЕСКИМ НОКАУТОМ"
        case 3: return "РЕШЕНИЕМ СУДЕЙ"
        case 4: return "ДОБИВАНИЕМ"
        default: return ""
        }
    }

    var body: some View {
        let won = result.won != 0
        GeometryReader { geo in
            HStack {
                VStack(alignment: .leading, spacing: 10) {
                    Text(won ? "ПОБЕДА" : "ПОРАЖЕНИЕ")
                        .font(Theme.display(70))
                        .fontWidth(.compressed)
                        .foregroundColor(won ? Theme.red : Theme.ink)
                    Text(methodText + " · РАУНД \(result.rounds)")
                        .font(Theme.label(13, .heavy))
                        .foregroundColor(Theme.muted)
                    HStack(spacing: 8) {
                        ResultTile(value: "+\(result.coins)", label: "МОНЕТ", color: Theme.gold)
                        ResultTile(value: "\(result.landed)/\(result.thrown)", label: "ПОПАДАНИЙ")
                        ResultTile(value: "\(result.knockdowns)", label: "НОКДАУНОВ")
                        ResultTile(value: "\(Int(result.damage))", label: "УРОН")
                    }
                    if result.unlocked != 0 {
                        Text("ОТКРЫТ НОВЫЙ СОПЕРНИК!")
                            .font(Theme.display(16)).foregroundColor(.white)
                            .padding(.horizontal, 14).padding(.vertical, 6)
                            .background(Slant().fill(Theme.green))
                    }
                    if result.finisher != 0 {
                        Text("ЭФФЕКТНОЕ ДОБИВАНИЕ +120 ₵")
                            .font(Theme.display(16)).foregroundColor(Theme.ink)
                            .padding(.horizontal, 14).padding(.vertical, 6)
                            .background(Slant().fill(Theme.gold))
                    }
                    HStack(spacing: 10) {
                        if won && result.unlocked != 0 {
                            SlantButton(title: "СЛЕДУЮЩИЙ", color: Theme.red, height: 50, size: 22) {
                                core.startFight(core.nextOpponent())
                            }
                            .frame(width: 210)
                        } else {
                            SlantButton(title: "РЕВАНШ", color: Theme.red, height: 50, size: 22) {
                                core.startFight(result.opponent)
                            }
                            .frame(width: 180)
                        }
                        SlantButton(title: "МЕНЮ", height: 50, size: 22) { core.leaveFight() }
                            .frame(width: 140)
                    }
                    .padding(.top, 6)
                }
                .padding(.leading, max(28, geo.safeAreaInsets.leading + 8))
                .padding(.vertical, 20)
                .frame(maxHeight: .infinity)
                .background(
                    LinearGradient(colors: [Theme.paper, Theme.paper.opacity(0.9), Theme.paper.opacity(0)], startPoint: .leading, endPoint: .trailing)
                        .ignoresSafeArea()
                )
                Spacer()
            }
        }
    }
}

struct ResultTile: View {
    let value: String
    let label: String
    var color: Color = Theme.ink
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(value).font(Theme.display(24)).foregroundColor(color).monospacedDigit()
            Text(label).font(Theme.label(9, .heavy)).foregroundColor(Theme.muted)
        }
        .frame(width: 92, alignment: .leading)
        .card()
    }
}
