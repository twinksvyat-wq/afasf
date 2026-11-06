import SwiftUI

// MARK: - Main menu

struct MainMenuView: View {
    @EnvironmentObject var core: GameCore
    @State private var appear = false

    var body: some View {
        GeometryReader { geo in
            ZStack(alignment: .topLeading) {
                LinearGradient(colors: [Theme.paper, Theme.paper.opacity(0)], startPoint: .leading, endPoint: .center)
                    .ignoresSafeArea()
                    .allowsHitTesting(false)
                VStack(alignment: .leading, spacing: 10) {
                    VStack(alignment: .leading, spacing: -8) {
                        Text("WHITEOUT")
                            .font(Theme.display(64))
                            .fontWidth(.compressed)
                            .foregroundColor(Theme.ink)
                        HStack(spacing: 8) {
                            Text("FC")
                                .font(Theme.display(64))
                                .fontWidth(.compressed)
                                .foregroundColor(Theme.red)
                            Text("ФИЗИКА · КРОВЬ · СЛАВА")
                                .font(Theme.label(11, .heavy))
                                .foregroundColor(Theme.muted)
                                .padding(.top, 18)
                        }
                    }
                    .offset(x: appear ? 0 : -40)
                    .opacity(appear ? 1 : 0)
                    .padding(.bottom, 6)

                    let opp = core.opponents().first { $0.index == core.nextOpponent() }
                    SlantButton(title: "В БОЙ", subtitle: opp.map { "vs \($0.name)" }, color: Theme.red, height: 62, size: 32) {
                        core.screen = .ladder
                    }
                    .frame(width: 300)
                    Group {
                        SlantButton(title: "БОЙЦЫ", subtitle: "стили", height: 44, size: 21) { core.screen = .fighters }
                        SlantButton(title: "МАГАЗИН", subtitle: "образ", height: 44, size: 21) { core.screen = .shop }
                        SlantButton(title: "ПРАЗДНОВАНИЯ", height: 44, size: 21) { core.screen = .celebrations }
                        SlantButton(title: "СТАТИСТИКА", height: 44, size: 21) { core.screen = .stats }
                    }
                    .frame(width: 270)
                    Spacer()
                    Text("Тапни бойца — он почувствует. Тяни экран — крути камеру.")
                        .font(Theme.label(11, .semibold))
                        .foregroundColor(Theme.muted)
                }
                .padding(.leading, max(28, geo.safeAreaInsets.leading + 8))
                .padding(.vertical, 18)

                HStack {
                    Spacer()
                    VStack(alignment: .trailing, spacing: 6) {
                        CoinBadge(coins: core.profile.coins)
                        Text("\(core.profile.wins)–\(core.profile.losses) · \(core.profile.kos) KO")
                            .font(Theme.label(11, .heavy))
                            .foregroundColor(Theme.muted)
                    }
                }
                .padding(.trailing, max(20, geo.safeAreaInsets.trailing))
                .padding(.top, 16)
            }
        }
        .onAppear { withAnimation(.spring(response: 0.5, dampingFraction: 0.7)) { appear = true } }
    }
}

// MARK: - Opponent ladder

struct LadderView: View {
    @EnvironmentObject var core: GameCore
    @State private var selected: UInt32 = 0

    var body: some View {
        let opps = core.opponents()
        let archetypes = core.fighters()
        SidePanel(title: "ЛЕСТНИЦА", back: { core.screen = .menu }) {
            ScrollViewReader { proxy in
                ScrollView(showsIndicators: false) {
                    VStack(spacing: 8) {
                        ForEach(opps) { o in
                            Button {
                                guard o.unlocked else { core.haptics.error(); return }
                                selected = o.index
                                core.previewOpponent(Int32(o.index))
                                core.haptics.tap()
                            } label: {
                                HStack(spacing: 12) {
                                    Text("\(o.index + 1)")
                                        .font(Theme.display(26))
                                        .foregroundColor(o.unlocked ? Theme.red : Theme.muted)
                                        .frame(width: 34)
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(o.unlocked ? o.name.uppercased() : "???")
                                            .font(Theme.display(20)).fontWidth(.condensed)
                                            .foregroundColor(Theme.ink)
                                        Text(o.unlocked ? o.nick + " · " + archName(archetypes, o.archetype) : "Победи предыдущего")
                                            .font(Theme.label(11, .semibold)).foregroundColor(Theme.muted)
                                    }
                                    Spacer()
                                    if o.beaten {
                                        Image(systemName: "checkmark.seal.fill").foregroundColor(Theme.green)
                                    } else if !o.unlocked {
                                        Image(systemName: "lock.fill").foregroundColor(Theme.muted)
                                    } else {
                                        VStack(alignment: .trailing, spacing: 2) {
                                            SkillDots(skill: o.skill)
                                            Text("+\(o.reward) ₵").font(Theme.label(11, .heavy)).foregroundColor(Theme.gold)
                                        }
                                    }
                                }
                                .card(selected: selected == o.index && o.unlocked)
                                .opacity(o.unlocked ? 1 : 0.55)
                            }
                            .buttonStyle(PressScale())
                            .id(o.index)
                        }
                    }
                    .padding(.vertical, 4).padding(.horizontal, 4)
                }
                .onAppear {
                    selected = core.nextOpponent()
                    core.previewOpponent(Int32(selected))
                    proxy.scrollTo(selected, anchor: .center)
                }
            }
            SlantButton(title: "НА БОЙ!", subtitle: "3 раунда", color: Theme.red, height: 54, size: 28) {
                core.startFight(selected)
            }
        }
    }
}

private func archName(_ list: [FighterCard], _ idx: UInt32) -> String {
    list.first(where: { $0.index == idx })?.name ?? ""
}

struct SkillDots: View {
    let skill: Float
    var body: some View {
        HStack(spacing: 3) {
            ForEach(0..<5) { i in
                Circle().fill(Float(i) < skill * 5 + 0.5 ? Theme.red : Theme.ink.opacity(0.12)).frame(width: 7, height: 7)
            }
        }
    }
}

// MARK: - Shop / customization

struct ShopView: View {
    @EnvironmentObject var core: GameCore
    @State private var slot: UInt32 = 10
    @State private var focused: ShopItem?

    private let order: [UInt32] = [10, 8, 9, 11, 4, 5, 6, 7, 1, 2, 3, 0]

    var body: some View {
        let names = core.slotNames()
        let _ = core.catalogRevision
        let items = core.items(slot: slot)
        SidePanel(title: "МАГАЗИН", back: { core.screen = .menu }) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(order, id: \.self) { s in
                        Button {
                            slot = s
                            focused = nil
                            core.preview(item: nil)
                            core.haptics.tap()
                        } label: {
                            Text(names[Int(s)].uppercased())
                                .font(Theme.label(12, .heavy))
                                .foregroundColor(slot == s ? .white : Theme.ink)
                                .padding(.horizontal, 12).padding(.vertical, 7)
                                .background(Slant(skew: 6).fill(slot == s ? Theme.ink : Color.white))
                        }
                    }
                }
                .padding(.horizontal, 2)
            }
            ScrollView(showsIndicators: false) {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 108), spacing: 8)], spacing: 8) {
                    ForEach(items) { it in
                        Button {
                            focused = it
                            core.preview(item: it)
                            core.haptics.tap()
                        } label: {
                            VStack(spacing: 6) {
                                ZStack {
                                    Circle().fill(Color(red: it.r, green: it.g, blue: it.b)).frame(width: 34, height: 34)
                                    Circle().trim(from: 0, to: 0.5).fill(Color(red: it.r2, green: it.g2, blue: it.b2)).frame(width: 34, height: 34)
                                        .rotationEffect(.degrees(-45)).opacity(slot <= 3 ? 0 : 1)
                                    if it.equipped {
                                        Image(systemName: "checkmark").font(.system(size: 14, weight: .black)).foregroundColor(.white)
                                            .shadow(radius: 2)
                                    }
                                }
                                Text(it.name).font(Theme.label(11, .bold)).foregroundColor(Theme.ink).lineLimit(1).minimumScaleFactor(0.7)
                                Text(it.owned ? (it.equipped ? "НАДЕТО" : "ЕСТЬ") : "\(it.price) ₵")
                                    .font(Theme.label(10, .heavy))
                                    .foregroundColor(it.owned ? Theme.green : (core.profile.coins >= it.price ? Theme.gold : Theme.red))
                            }
                            .frame(maxWidth: .infinity)
                            .card(selected: focused?.id == it.id)
                        }
                        .buttonStyle(PressScale())
                    }
                }
                .padding(4)
            }
            HStack {
                CoinBadge(coins: core.profile.coins)
                Spacer()
                if let f = focused, let live = items.first(where: { $0.id == f.id }) {
                    if live.owned {
                        SlantButton(title: live.equipped ? "НАДЕТО" : "НАДЕТЬ", color: live.equipped ? Theme.muted : Theme.ink, height: 46, size: 20, enabled: !live.equipped) {
                            core.equip(live)
                            focused = live
                        }
                        .frame(width: 170)
                    } else {
                        SlantButton(title: "КУПИТЬ", subtitle: "\(live.price) ₵", color: Theme.red, height: 46, size: 20, enabled: core.profile.coins >= live.price) {
                            core.buy(live)
                        }
                        .frame(width: 190)
                    }
                }
            }
        }
    }
}

// MARK: - Fighters

struct FightersView: View {
    @EnvironmentObject var core: GameCore
    @State private var focus: UInt32 = 0

    var body: some View {
        let _ = core.catalogRevision
        let list = core.fighters()
        SidePanel(title: "БОЙЦЫ", back: { core.screen = .menu }) {
            ScrollView(showsIndicators: false) {
                VStack(spacing: 8) {
                    ForEach(list) { f in
                        Button {
                            focus = f.index
                            core.previewFighter(Int32(f.index))
                            core.haptics.tap()
                        } label: {
                            VStack(alignment: .leading, spacing: 6) {
                                HStack {
                                    Text(f.name.uppercased()).font(Theme.display(22)).fontWidth(.condensed).foregroundColor(Theme.ink)
                                    Spacer()
                                    if f.selected {
                                        Text("ВЫБРАН").font(Theme.label(11, .heavy)).foregroundColor(Theme.green)
                                    } else if !f.owned {
                                        Text("\(f.price) ₵").font(Theme.label(12, .heavy)).foregroundColor(Theme.gold)
                                    }
                                }
                                Text(f.style).font(Theme.label(11, .semibold)).foregroundColor(Theme.muted)
                                if focus == f.index {
                                    VStack(spacing: 4) {
                                        StatBar(label: "СИЛА", value: f.power)
                                        StatBar(label: "СКОРОСТЬ", value: f.speed)
                                        StatBar(label: "НОГИ", value: f.kick)
                                        StatBar(label: "БОРЬБА", value: f.grapple)
                                        StatBar(label: "ЧЕЛЮСТЬ", value: f.chin)
                                    }
                                    .padding(.top, 2)
                                    Text("Добивание: \(f.finisher)").font(Theme.label(11, .heavy)).foregroundColor(Theme.red)
                                }
                            }
                            .card(selected: focus == f.index)
                        }
                        .buttonStyle(PressScale())
                    }
                }
                .padding(4)
            }
            if let f = list.first(where: { $0.index == focus }) {
                if f.owned {
                    SlantButton(title: f.selected ? "ВЫБРАН" : "ВЫБРАТЬ", color: f.selected ? Theme.muted : Theme.ink, height: 48, size: 22, enabled: !f.selected) {
                        core.selectFighter(f.index)
                    }
                } else {
                    SlantButton(title: "РАЗБЛОКИРОВАТЬ", subtitle: "\(f.price) ₵", color: Theme.red, height: 48, size: 22, enabled: core.profile.coins >= f.price) {
                        core.buyFighter(f.index)
                    }
                }
            }
        }
        .onAppear { focus = core.profile.fighter }
    }
}

// MARK: - Celebrations

struct CelebrationsView: View {
    @EnvironmentObject var core: GameCore
    @State private var focus: UInt32 = 0

    var body: some View {
        let _ = core.catalogRevision
        let list = core.celebrations()
        SidePanel(title: "ПРАЗДНОВАНИЯ", back: { core.screen = .menu }) {
            ScrollView(showsIndicators: false) {
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8)], spacing: 8) {
                    ForEach(list) { c in
                        Button {
                            focus = c.index
                            core.previewCelebration(Int32(c.index))
                            core.haptics.tap()
                        } label: {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(c.name.uppercased()).font(Theme.display(18)).fontWidth(.condensed).foregroundColor(Theme.ink).lineLimit(1).minimumScaleFactor(0.6)
                                Text(c.selected ? "ВЫБРАНО" : (c.owned ? "ЕСТЬ" : "\(c.price) ₵"))
                                    .font(Theme.label(11, .heavy))
                                    .foregroundColor(c.selected ? Theme.green : (c.owned ? Theme.muted : Theme.gold))
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .card(selected: focus == c.index)
                        }
                        .buttonStyle(PressScale())
                    }
                }
                .padding(4)
            }
            if let c = list.first(where: { $0.index == focus }) {
                if c.owned {
                    SlantButton(title: c.selected ? "ВЫБРАНО" : "ВЫБРАТЬ", color: c.selected ? Theme.muted : Theme.ink, height: 48, size: 22, enabled: !c.selected) {
                        core.selectCelebration(c.index)
                    }
                } else {
                    SlantButton(title: "КУПИТЬ", subtitle: "\(c.price) ₵", color: Theme.red, height: 48, size: 22, enabled: core.profile.coins >= c.price) {
                        core.buyCelebration(c.index)
                    }
                }
            }
        }
        .onAppear {
            focus = core.profile.celebration
            core.previewCelebration(Int32(focus))
        }
    }
}

// MARK: - Stats

struct StatsView: View {
    @EnvironmentObject var core: GameCore
    @State private var confirmReset = false

    var body: some View {
        let p = core.profile
        let rows: [(String, String)] = [
            ("БОИ", "\(p.fights)"), ("ПОБЕДЫ", "\(p.wins)"), ("ПОРАЖЕНИЯ", "\(p.losses)"),
            ("НОКАУТЫ", "\(p.kos)"), ("ДОБИВАНИЯ", "\(p.finishers)"), ("НОКДАУНЫ", "\(p.knockdowns)"),
            ("ТЕБЯ ВЫРУБИЛИ", "\(p.kod_by)"), ("УДАРЫ РУКАМИ", "\(p.punches)"), ("УДАРЫ НОГАМИ", "\(p.kicks)"),
            ("БРОСКИ", "\(p.throws)"), ("СЕРИЯ", "\(p.streak)"), ("ЛУЧШАЯ СЕРИЯ", "\(p.best_streak)"),
            ("УРОН", "\(p.damage)"), ("ЗАРАБОТАНО ₵", "\(p.coins_total)"),
            ("ВИНРЕЙТ", p.fights > 0 ? "\(Int(Double(p.wins) / Double(p.fights) * 100))%" : "—"),
        ]
        SidePanel(title: "СТАТИСТИКА", back: { core.screen = .menu }) {
            ScrollView(showsIndicators: false) {
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8)], spacing: 8) {
                    ForEach(rows, id: \.0) { r in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(r.1).font(Theme.display(26)).foregroundColor(Theme.ink).monospacedDigit()
                            Text(r.0).font(Theme.label(9, .heavy)).foregroundColor(Theme.muted).lineLimit(1).minimumScaleFactor(0.6)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .card()
                    }
                }
                .padding(4)
                Button("Сбросить прогресс") { confirmReset = true }
                    .font(Theme.label(12, .bold))
                    .foregroundColor(Theme.red)
                    .padding(.top, 8)
            }
        }
        .alert("Сбросить весь прогресс?", isPresented: $confirmReset) {
            Button("Сбросить", role: .destructive) { core.resetProgress() }
            Button("Отмена", role: .cancel) {}
        }
    }
}
